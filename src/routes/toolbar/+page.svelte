<script lang="ts">
  import { Icon } from '$lib/components';
  import { PROMPT_MARK, SCRIPT_MARK, SEARCHER_MARK } from '$lib/constants';
  import { CONVERT_ACTIONS, DEFAULT_ACTIONS, execute, GENERAL_ACTIONS, PROCESS_ACTIONS } from '$lib/executor';
  import { prompts, scripts, searchers, shortcuts as shortcutStore } from '$lib/stores.svelte';
  import type { Rule, WindowPlacement } from '$lib/types';
  import { invoke } from '@tauri-apps/api/core';
  import { LogicalPosition, LogicalSize } from '@tauri-apps/api/dpi';
  import { listen } from '@tauri-apps/api/event';
  import { Image } from '@tauri-apps/api/image';
  import { IconMenuItem, Menu } from '@tauri-apps/api/menu';
  import { currentMonitor, getCurrentWindow } from '@tauri-apps/api/window';
  import { type } from '@tauri-apps/plugin-os';
  import { memoize } from 'es-toolkit/function';
  import type { IconComponentProps } from 'phosphor-svelte';
  import {
    CodeIcon,
    DotsThreeVerticalIcon,
    FolderIcon,
    LineVerticalIcon,
    MagnifyingGlassIcon,
    RobotIcon
  } from 'phosphor-svelte';
  import type { Component } from 'svelte';
  import { mount, onMount, tick, unmount } from 'svelte';
  import { fly } from 'svelte/transition';

  // maximum visible actions in toolbar
  const MAX_VISIBLE_ACTIONS = 6;

  // operating system type
  const osType = type();

  // current window
  const currentWindow = getCurrentWindow();

  // toolbar initialized state
  let initialized = $state(false);

  // main container element
  let container: HTMLDivElement | null = $state(null);

  // current text selection
  let selection: string = $state('');

  // whether this is a lazy selection event (selection deferred until action click)
  let isLazy: boolean = $state(false);

  // toolbar layout direction
  let layout: 'horizontal' | 'vertical' = $state('horizontal');

  // track if mouse is inside toolbar
  let mouseEntered = $state(true);

  // toolbar action type
  type Action = {
    id: string;
    icon?: Component<IconComponentProps> | string;
    label: string;
    rule: Rule;
  };

  // matched actions to display
  let actions: Action[] = $state([]);

  // folder group type for toolbar display
  type FolderGroup = {
    name: string;
    icon?: Component<IconComponentProps> | string;
    displayMode?: 'icon' | 'label' | 'both';
    actions: Action[];
  };

  // toolbar items: either a standalone action or a folder group
  type ToolbarItem = { type: 'action'; action: Action } | { type: 'folder'; folder: FolderGroup };

  // organized toolbar items (actions + folders)
  let toolbarItems: ToolbarItem[] = $derived.by(() => {
    const items: ToolbarItem[] = [];
    const groupMap = new Map<string, Action[]>();
    const ungrouped: Action[] = [];

    // separate grouped and ungrouped actions, preserving order of first appearance
    const groupOrder: string[] = [];
    for (const action of actions) {
      if (action.rule.group) {
        const groupName = action.rule.group;
        if (!groupMap.has(groupName)) {
          groupMap.set(groupName, []);
          groupOrder.push(groupName);
        }
        groupMap.get(groupName)!.push(action);
      } else {
        ungrouped.push(action);
      }
    }

    // build toolbar items in order: ungrouped first, then folders
    for (const action of ungrouped) {
      items.push({ type: 'action', action });
    }
    for (const groupName of groupOrder) {
      const groupActions = groupMap.get(groupName)!;
      // get group config from shortcut store
      const shortcutKey = groupActions[0]?.rule.shortcut;
      const groupConfig = shortcutKey ? shortcutStore.current[shortcutKey]?.groups?.[groupName] : undefined;
      items.push({
        type: 'folder',
        folder: {
          name: groupName,
          icon: groupConfig?.icon || FolderIcon,
          displayMode: groupConfig?.displayMode || 'both',
          actions: groupActions
        }
      });
    }
    return items;
  });

  // visible and overflow toolbar items
  let visibleItems: ToolbarItem[] = $derived(toolbarItems.slice(0, MAX_VISIBLE_ACTIONS));
  let overflowItems: ToolbarItem[] = $derived(toolbarItems.slice(MAX_VISIBLE_ACTIONS));

  // flatten overflow items to actions for the overflow menu
  let overflowActions: Action[] = $derived(
    overflowItems.flatMap((item) => (item.type === 'action' ? [item.action] : item.folder.actions))
  );

  // currently expanded folder name
  let expandedFolder: string | null = $state(null);

  // expanded folder actions
  let expandedActions: Action[] = $derived.by(() => {
    if (!expandedFolder) return [];
    const item = toolbarItems.find((i) => i.type === 'folder' && i.folder.name === expandedFolder);
    return item?.type === 'folder' ? item.folder.actions : [];
  });

  // custom action types
  let actionTypes = $derived([
    {
      mark: SCRIPT_MARK,
      collection: scripts.current,
      defaultIcon: CodeIcon
    },
    {
      mark: PROMPT_MARK,
      collection: prompts.current,
      defaultIcon: RobotIcon
    },
    {
      mark: SEARCHER_MARK,
      collection: searchers.current,
      defaultIcon: MagnifyingGlassIcon
    }
  ]);

  // memoized function to find built-in action
  const findBuiltinAction = memoize((action: string) =>
    [...DEFAULT_ACTIONS, ...GENERAL_ACTIONS, ...CONVERT_ACTIONS, ...PROCESS_ACTIONS].find((a) => a.value === action)
  );

  /**
   * Map rule to toolbar action.
   *
   * @param rule - rule object
   */
  function mapToAction(rule: Rule): Action | undefined {
    const actionId = rule.action;

    // check for custom action types
    for (const type of actionTypes) {
      if (actionId.startsWith(type.mark)) {
        const itemId = actionId.substring(type.mark.length);
        const item = type.collection.find((i) => i.id === itemId);
        if (item) {
          return {
            id: actionId,
            icon: item.icon || type.defaultIcon,
            label: itemId,
            rule: rule
          };
        }
      }
    }

    // check for built-in action
    const builtin = findBuiltinAction(actionId);
    if (builtin) {
      return {
        id: actionId,
        icon: builtin.icon,
        label: builtin.label,
        rule: rule
      };
    }

    return undefined;
  }

  /**
   * Setup toolbar with given rules and selection.
   *
   * @param data - toolbar setup data
   */
  async function setup(data: { rules: Rule[]; selection: string; lazy?: boolean; layout?: string }) {
    if (!data || !data.rules || !Array.isArray(data.rules)) {
      return;
    }

    // update current selection and lazy state
    selection = data.selection || '';
    isLazy = data.lazy || false;
    layout = (data.layout as 'horizontal' | 'vertical') || 'horizontal';
    expandedFolder = null;

    // map rules to actions
    actions = data.rules.map(mapToAction).filter((a) => !!a);
    for (const action of actions) {
      if (action.rule.preview) {
        // replace label with preview result
        const result = await execute(action.rule, selection);
        if (result) {
          action.label = result;
        } else {
          // disable preview if execution failed
          action.rule.preview = false;
        }
      }
    }

    // mark as initialized
    initialized = true;

    // resize window to fit content after actions are updated
    await resizeToFit();
  }

  /**
   * Resize window to fit toolbar content.
   */
  async function resizeToFit() {
    await tick();
    if (container) {
      try {
        // get container size
        const rect = container.getBoundingClientRect();
        const dpr = window.devicePixelRatio;
        const scale = await currentWindow.scaleFactor();
        const width = ((rect.width + 10) * dpr) / scale;
        const height = ((rect.height + 10) * dpr) / scale;
        // set window size with some padding
        currentWindow.setSize(new LogicalSize(width, height));
      } catch (error) {
        console.error(`Failed to resize window: ${error}`);
      }
    }
  }

  /**
   * Show more actions in overflow menu.
   */
  async function showMoreActions() {
    try {
      // create menu items with icons
      const menu = await Menu.new({
        items: await Promise.all(
          overflowActions.map(async (action) => {
            return await IconMenuItem.new({
              id: action.id,
              text: action.label,
              icon: await iconToImage(action.icon),
              action: () => executeAction(action)
            });
          })
        )
      });

      // calculate bottom-right corner position
      const size = await currentWindow.innerSize();
      const scale = await currentWindow.scaleFactor();
      const width = size.width / scale;
      const height = size.height / scale;
      const bottomRightPosition = new LogicalPosition(width - 32, height - 2);

      // popup menu at bottom-right corner of toolbar window
      await menu.popup(bottomRightPosition, currentWindow);
    } catch (error) {
      console.error(`Failed to show more actions menu: ${error}`);
    }
  }

  /**
   * Convert icon component or base64 string to menu item image.
   *
   * @param icon - action icon
   */
  async function iconToImage(icon: Component<IconComponentProps> | string | undefined): Promise<Image | undefined> {
    if (!icon) {
      return undefined;
    }

    // create a temporary container to render the icon
    const tempElement = document.createElement('div');

    // render the Icon component using mount function
    const iconComponent = mount(Icon, {
      target: tempElement,
      props: { icon }
    });

    // wait for rendering to complete
    await tick();

    try {
      // get the svg or img element
      const svg = tempElement.querySelector('svg');
      const img = tempElement.querySelector('img');
      if (!svg && !img) {
        return undefined;
      }

      // set fill color for SVG icons
      if (svg && osType === 'macos') {
        // detect if system is in dark mode
        const prefersDark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
        svg.style.color = prefersDark ? '#ffffff' : '#000000';
      }

      // create URL for the icon
      let url: string;
      if (svg) {
        // handle SVG element
        const svgData = new XMLSerializer().serializeToString(svg);
        const svgBlob = new Blob([svgData], { type: 'image/svg+xml;charset=utf-8' });
        url = URL.createObjectURL(svgBlob);
      } else if (img) {
        // handle base64 image
        url = img.src;
      } else {
        return undefined;
      }

      // create canvas to draw the icon
      const canvas = document.createElement('canvas');
      const size = 32;
      canvas.width = size;
      canvas.height = size;
      const ctx = canvas.getContext('2d');
      if (!ctx) {
        if (svg) {
          URL.revokeObjectURL(url);
        }
        return undefined;
      }

      // load image from URL
      const imageEl = new window.Image();
      await new Promise<void>((resolve, reject) => {
        imageEl.onload = () => resolve();
        imageEl.onerror = reject;
        imageEl.src = url;
      });

      // cleanup URL object
      if (svg) {
        URL.revokeObjectURL(url);
      }

      // draw image onto canvas
      ctx.clearRect(0, 0, size, size);
      ctx.drawImage(imageEl, 0, 0, size, size);

      // get RGBA pixel data
      const imageData = ctx.getImageData(0, 0, size, size);
      const rgbaBytes = new Uint8Array(imageData.data);

      // create menu item image from pixel data
      return await Image.new(rgbaBytes, size, size);
    } catch (error) {
      console.error(`Failed to convert icon to Image: ${error}`);
      return undefined;
    } finally {
      // cleanup component and temp container
      unmount(iconComponent);
      tempElement.remove();
    }
  }

  /**
   * Get current window placement information.
   */
  async function windowPlacement(): Promise<WindowPlacement> {
    const outerPosition = await currentWindow.outerPosition();
    const scaleFactor = await currentWindow.scaleFactor();
    const monitor = await currentMonitor();
    return {
      screenSize: monitor?.size.toLogical(scaleFactor),
      screenPosition: monitor?.position.toLogical(scaleFactor),
      windowPosition: outerPosition.toLogical(scaleFactor)
    };
  }

  /**
   * Handle action click event.
   *
   * @param action - toolbar action
   */
  async function executeAction(action: Action) {
    try {
      // if lazy selection, fetch the selection text now
      if (isLazy && !selection) {
        selection = await invoke('get_selection', { mouse: true });
        if (!selection || !selection.trim()) {
          // no text selected, hide toolbar
          await currentWindow.hide();
          return;
        }
      }

      // get current window placement
      const placement = await windowPlacement();
      // hide the toolbar window
      await currentWindow.hide();

      if (action.rule.preview) {
        if (action.rule.outputMode === 'replace') {
          // replace selection with preview text
          await invoke('enter_text', {
            text: action.label,
            clipboard: action.rule.clipboard
          });
        } else if (action.rule.outputMode === 'popup') {
          // show popup with preview text
          await invoke('show_popup_sameplace', {
            payload: JSON.stringify({
              id: crypto.randomUUID(),
              result: action.label,
              copyOnPopup: action.rule.clipboard
            }),
            placement: placement
          });
        } else if (action.rule.outputMode === undefined && action.rule.clipboard) {
          // copy preview text to clipboard
          await invoke('set_clipboard_text', { text: action.label });
        }
      } else {
        // execute the action normally
        await execute(action.rule, selection, placement);
      }
    } catch (error) {
      console.error(`Failed to execute action: ${error}`);
    }
  }

  onMount(async () => {
    // mark toolbar as initialized
    await invoke('mark_toolbar_initialized');
  });

  onMount(() => {
    // listen to window show/hide events
    const unlistenWindowShow = listen<string>('show-toolbar', (event) => {
      initialized = false;
      setup(JSON.parse(event.payload)).then(() => {
        // show window without focusing
        currentWindow.isVisible().then((visible) => {
          if (!visible) {
            invoke('show_toolbar_regardless');
          }
        });
      });
    });
    const unlistenWindowHide = listen('hide-toolbar', () => {
      initialized = false;
    });

    // listen to mouse enter/exit events
    const unlistenMouseEntered = listen('toolbar-entered', () => {
      mouseEntered = true;
    });
    const unlistenMouseExited = listen('toolbar-exited', () => {
      mouseEntered = false;
    });

    return () => {
      unlistenWindowShow.then((fn) => fn());
      unlistenWindowHide.then((fn) => fn());
      unlistenMouseExited.then((fn) => fn());
      unlistenMouseEntered.then((fn) => fn());
    };
  });
</script>

<main class="bg-transparent p-1 select-none">
  {#if initialized && actions.length > 0}
    <div
      class="w-fit overflow-hidden rounded-box border shadow-sm"
      in:fly={{ y: -10, duration: 100 }}
      onmouseleave={() => {
        if (expandedFolder) {
          expandedFolder = null;
          resizeToFit();
        }
      }}
    >
      <div
        class="flex bg-base-200/95 backdrop-blur-sm"
        class:flex-col={layout === 'horizontal'}
        class:flex-row={layout === 'vertical'}
        bind:this={container}
      >
        <!-- main toolbar row/column -->
        <div class="flex" class:flex-col={layout === 'vertical'}>
          <span
            class="flex cursor-grabbing items-center opacity-20 transition-opacity"
            class:hover:opacity-90={mouseEntered}
            data-tauri-drag-region
          >
            <LineVerticalIcon class="pointer-events-none size-4 {layout === 'vertical' ? 'rotate-90' : ''}" />
          </span>
          {#each visibleItems as item (item.type === 'action' ? item.action.id : item.folder.name)}
            {#if item.type === 'action'}
              {@const action = item.action}
              {@const showIcon = action.rule.displayMode !== 'label'}
              {@const showLabel = action.rule.displayMode !== 'icon'}
              <button
                class="flex h-8 cursor-pointer items-center gap-0.5 px-1.75 transition-colors"
                class:hover:bg-btn-hover={mouseEntered}
                class:hover:text-primary={mouseEntered}
                onclick={() => executeAction(action)}
                title={action.label}
              >
                {#if showIcon && action.icon}
                  <Icon icon={action.icon} class="size-4.5 shrink-0" />
                {/if}
                {#if showLabel}
                  <span class="max-w-30 truncate text-xs font-[450]">{action.label}</span>
                {/if}
              </button>
            {:else}
              {@const folder = item.folder}
              {@const showIcon = folder.displayMode !== 'label'}
              {@const showLabel = folder.displayMode !== 'icon'}
              <button
                class="flex h-8 cursor-pointer items-center gap-0.5 px-1.75 transition-colors"
                class:hover:bg-btn-hover={mouseEntered}
                class:hover:text-primary={mouseEntered}
                class:bg-btn-hover={expandedFolder === folder.name}
                class:text-primary={expandedFolder === folder.name}
                onmouseenter={() => {
                  expandedFolder = folder.name;
                  resizeToFit();
                }}
                title={folder.name}
              >
                {#if showIcon && folder.icon}
                  <Icon icon={folder.icon} class="size-4.5 shrink-0" />
                {/if}
                {#if showLabel}
                  <span class="max-w-30 truncate text-xs font-[450]">{folder.name}</span>
                {/if}
              </button>
            {/if}
          {/each}
          {#if overflowActions.length > 0}
            <button
              class="flex h-8 cursor-pointer items-center opacity-60 transition-all"
              class:hover:bg-btn-hover={mouseEntered}
              class:hover:opacity-100={mouseEntered}
              onclick={showMoreActions}
            >
              <DotsThreeVerticalIcon weight="bold" class="size-5 {layout === 'vertical' ? 'rotate-90' : ''}" />
            </button>
          {/if}
        </div>
        <!-- expanded folder actions (compact, centered) -->
        {#if expandedFolder && expandedActions.length > 0}
          <div
            class="flex items-center justify-center"
            class:border-t={layout === 'horizontal'}
            class:border-l={layout === 'vertical'}
            class:border-base-300={true}
            in:fly={{ y: layout === 'horizontal' ? -5 : 0, x: layout === 'vertical' ? -5 : 0, duration: 100 }}
            onmouseleave={() => {
              expandedFolder = null;
              resizeToFit();
            }}
          >
            <div class="flex" class:flex-col={layout === 'vertical'}>
              {#each expandedActions as action (action.id)}
                {@const showIcon = action.rule.displayMode !== 'label'}
                {@const showLabel = action.rule.displayMode !== 'icon'}
                <button
                  class="flex h-8 cursor-pointer items-center gap-0.5 px-1.75 transition-colors"
                  class:hover:bg-btn-hover={mouseEntered}
                  class:hover:text-primary={mouseEntered}
                  onclick={() => executeAction(action)}
                  title={action.label}
                >
                  {#if showIcon && action.icon}
                    <Icon icon={action.icon} class="size-4.5 shrink-0" />
                  {/if}
                  {#if showLabel}
                    <span class="max-w-30 truncate text-xs font-[450]">{action.label}</span>
                  {/if}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</main>

<style>
  :global {
    html,
    body {
      background: transparent;
    }
  }
</style>
