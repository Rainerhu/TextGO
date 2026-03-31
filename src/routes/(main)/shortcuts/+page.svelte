<script lang="ts">
  import { alert, Binder, Button, BWList, confirm, Icon, IconSelector, Modal, Radio, Recorder, Shortcut, Toggle } from '$lib/components';
  import { CTRL_CLICK_SHORTCUT, DBCLICK_SHORTCUT, DRAG_SHORTCUT, LONG_PRESS_SHORTCUT, SHIFT_CLICK_SHORTCUT } from '$lib/constants';
  import { formatShortcut, isMouseShortcut } from '$lib/helpers';
  import { NoData } from '$lib/icons';
  import { m } from '$lib/paraglide/messages';
  import { blacklist, longPress, shortcuts } from '$lib/stores.svelte';
  import type { DisplayMode } from '$lib/types';
  import {
    ArrowCircleRightIcon,
    ArrowClockwiseIcon,
    ArrowFatUpIcon,
    ArrowsClockwiseIcon,
    ColumnsIcon,
    CursorClickIcon,
    FolderPlusIcon,
    GearSixIcon,
    KeyboardIcon,
    MouseLeftClickIcon,
    ProhibitIcon,
    ProhibitInsetIcon,
    RowsIcon,
    SparkleIcon,
    StackPlusIcon,
    TrashIcon,
    WaveSineIcon
  } from 'phosphor-svelte';
  import { onMount, tick } from 'svelte';
  import { fly } from 'svelte/transition';

  // shortcut recorder
  let recorder: Recorder;

  // rule binder
  let ruleBinder: Binder | null = $state(null);

  // rule updater
  let ruleUpdater: Binder | null = $state(null);

  // blacklist manager
  let blacklistManager: BWList;

  // dropdown element
  let dropdown: HTMLDetailsElement;
  let dropdownOpen: boolean = $state(false);

  /**
   * Register new shortcut.
   *
   * @param shortcut - shortcut string to register
   */
  async function register(shortcut: string) {
    if (!shortcut) {
      return;
    }

    // check duplicate
    if (shortcuts.current[shortcut]) {
      alert({ level: 'error', message: m.shortcut_already_registered() });
      return;
    }

    // register new shortcut
    shortcuts.current[shortcut] = {
      mode: isMouseShortcut(shortcut) ? 'toolbar' : 'quiet',
      rules: []
    };

    // wait for DOM update then scroll to newly registered shortcut position
    await tick();
    const element = document.querySelector(`[data-shortcut="${shortcut}"]`);
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  }

  /**
   * Compare two shortcut strings for sorting.
   *
   * @param a - first shortcut string
   * @param b - second shortcut string
   */
  function compareShortcut(a: string, b: string) {
    if (a === DRAG_SHORTCUT) return -1;
    if (b === DRAG_SHORTCUT) return 1;
    if (a === DBCLICK_SHORTCUT) return -1;
    if (b === DBCLICK_SHORTCUT) return 1;
    if (a === SHIFT_CLICK_SHORTCUT) return -1;
    if (b === SHIFT_CLICK_SHORTCUT) return 1;
    if (a === CTRL_CLICK_SHORTCUT) return -1;
    if (b === CTRL_CLICK_SHORTCUT) return 1;
    if (a === LONG_PRESS_SHORTCUT) return -1;
    if (b === LONG_PRESS_SHORTCUT) return 1;
    return a.localeCompare(b);
  }

  /**
   * Get shortcut hint text.
   *
   * @param shortcut - shortcut string
   */
  function shortcutHint(shortcut: string) {
    if (shortcut === DRAG_SHORTCUT) return m.mouse_drag_hint();
    if (shortcut === DBCLICK_SHORTCUT) return m.mouse_dbclick_hint();
    if (shortcut === SHIFT_CLICK_SHORTCUT) return m.mouse_shift_click_hint();
    if (shortcut === CTRL_CLICK_SHORTCUT) return m.mouse_ctrl_click_hint();
    if (shortcut === LONG_PRESS_SHORTCUT) return m.long_press_hint();
    return m.keyboard_shortcut_hint();
  }

  // control display delay when no data to avoid flickering
  let showNoData = $state(false);
  onMount(() => {
    setTimeout(() => {
      showNoData = true;
    }, 100);
  });

  // folder management modal state
  let folderModal: Modal;
  let folderShortcut: string = $state('');
  let folderName: string = $state('');
  let folderIcon: string = $state('');
  let folderDisplayMode: DisplayMode = $state('both');
  let folderEditName: string = $state(''); // non-empty means editing

  function openFolderModal(shortcutKey: string, editName?: string) {
    folderShortcut = shortcutKey;
    if (editName) {
      folderEditName = editName;
      folderName = editName;
      const config = shortcuts.current[shortcutKey]?.groups?.[editName];
      folderIcon = config?.icon || '';
      folderDisplayMode = config?.displayMode || 'both';
    } else {
      folderEditName = '';
      folderName = '';
      folderIcon = '';
      folderDisplayMode = 'both';
    }
    folderModal.show();
  }

  function saveFolder() {
    if (!folderName.trim()) return;
    const s = shortcuts.current[folderShortcut];
    if (!s) return;
    if (!s.groups) s.groups = {};

    if (folderEditName) {
      // editing existing folder — update the marker's id and config
      if (folderEditName !== folderName) {
        // rename: update the folder marker in rules array
        const marker = s.rules.find((r) => r.isFolder && r.id === folderEditName);
        if (marker) marker.id = folderName;
        delete s.groups[folderEditName];
      }
    } else {
      // creating new folder — insert a folder marker at the end of rules
      s.rules.push({
        id: folderName,
        shortcut: folderShortcut,
        case: '',
        action: '',
        isFolder: true
      });
    }

    s.groups[folderName] = { icon: folderIcon || undefined, displayMode: folderDisplayMode };
    folderModal.close();
  }

  function deleteFolder(shortcutKey: string, name: string) {
    const s = shortcuts.current[shortcutKey];
    if (!s) return;
    // remove the folder marker from rules array
    const idx = s.rules.findIndex((r) => r.isFolder && r.id === name);
    if (idx !== -1) {
      s.rules.splice(idx, 1);
    }
    // delete group config
    if (s.groups) {
      delete s.groups[name];
    }
  }
</script>

<svelte:window
  onclick={(event) => {
    // close the dropdown when clicking outside of it
    if (event.target instanceof Node && !dropdown.contains(event.target)) {
      dropdownOpen = false;
    }
  }}
/>

<div class="relative min-h-(--app-h) rounded-container">
  <div class="flex items-center gap-2">
    <div class="flex flex-col gap-0.5">
      <Toggle
        bind:value={longPress.current}
        icon={CursorClickIcon}
        iconClass="size-4.5"
        label={m.long_press_enabled()}
        labelClass="text-sm"
        toggleClass="toggle-xs"
      />
      <span class="text-xs opacity-50">{m.long_press_explain()}</span>
    </div>
    <button class="btn ml-auto btn-soft btn-sm" onclick={() => blacklistManager.showModal()}>
      <ProhibitIcon class="size-5 rotate-90" />
      <span class="text-sm font-normal">{m.blacklist()}</span>
    </button>
    <details class="dropdown dropdown-end text-nowrap" bind:this={dropdown} bind:open={dropdownOpen}>
      <summary
        class="btn text-sm btn-sm btn-submit"
        onclick={(event) => {
          if (
            shortcuts.current[DRAG_SHORTCUT] &&
            shortcuts.current[DBCLICK_SHORTCUT] &&
            shortcuts.current[SHIFT_CLICK_SHORTCUT] &&
            shortcuts.current[CTRL_CLICK_SHORTCUT] &&
            shortcuts.current[LONG_PRESS_SHORTCUT]
          ) {
            // all mouse shortcuts are registered, open recorder directly
            event.preventDefault();
            recorder.showModal();
          }
        }}
      >
        <StackPlusIcon class="size-5" />{m.register_shortcut()}
      </summary>
      <ul class="dropdown-content menu z-1 mt-1 min-w-42 gap-1 rounded-box border bg-base-100 p-1 shadow-lg">
        <!-- mouse drag-select option -->
        <li class={shortcuts.current[DRAG_SHORTCUT] ? 'hidden' : ''}>
          <button
            class="btn px-1 btn-sm"
            onclick={() => {
              register(DRAG_SHORTCUT);
              dropdownOpen = false;
            }}
          >
            <span class="flex">
              <MouseLeftClickIcon class="size-4" />
              <WaveSineIcon class="size-4" />
            </span>
            <span class="mx-auto tracking-wider">{m.mouse_drag()}</span>
          </button>
        </li>
        <!-- mouse double-click option -->
        <li class={shortcuts.current[DBCLICK_SHORTCUT] ? 'hidden' : ''}>
          <button
            class="btn px-1 btn-sm"
            onclick={() => {
              register(DBCLICK_SHORTCUT);
              dropdownOpen = false;
            }}
          >
            <span class="flex">
              <MouseLeftClickIcon class="size-4" />
              <MouseLeftClickIcon class="size-4" />
            </span>
            <span class="mx-auto tracking-wider">{m.mouse_dbclick()}</span>
          </button>
        </li>
        <!-- mouse shift-click option -->
        <li class={shortcuts.current[SHIFT_CLICK_SHORTCUT] ? 'hidden' : ''}>
          <button
            class="btn px-1 btn-sm"
            onclick={() => {
              register(SHIFT_CLICK_SHORTCUT);
              dropdownOpen = false;
            }}
          >
            <span class="flex">
              <ArrowFatUpIcon class="size-4" />
              <MouseLeftClickIcon class="size-4" />
            </span>
            <span class="mx-auto tracking-wider">{m.mouse_shift_click()}</span>
          </button>
        </li>
        <!-- mouse ctrl-click option -->
        <li class={shortcuts.current[CTRL_CLICK_SHORTCUT] ? 'hidden' : ''}>
          <button
            class="btn px-1 btn-sm"
            onclick={() => {
              register(CTRL_CLICK_SHORTCUT);
              dropdownOpen = false;
            }}
          >
            <span class="flex">
              <span class="text-xs font-bold">⌃</span>
              <MouseLeftClickIcon class="size-4" />
            </span>
            <span class="mx-auto tracking-wider">{m.mouse_ctrl_click()}</span>
          </button>
        </li>
        <!-- long press option -->
        <li class={shortcuts.current[LONG_PRESS_SHORTCUT] ? 'hidden' : ''}>
          <button
            class="btn px-1 btn-sm"
            onclick={() => {
              register(LONG_PRESS_SHORTCUT);
              dropdownOpen = false;
            }}
          >
            <CursorClickIcon class="mx-1.75 size-4.5" />
            <span class="mx-auto tracking-wider">{m.long_press_enabled()}</span>
          </button>
        </li>
        <!-- keyboard shortcut option -->
        <li>
          <button
            class="btn px-1 btn-sm"
            onclick={() => {
              recorder.showModal();
              dropdownOpen = false;
            }}
          >
            <KeyboardIcon class="mx-1.75 size-4.5" />
            <span class="mx-auto tracking-wider">{m.keyboard_keys()}</span>
          </button>
        </li>
      </ul>
    </details>
  </div>
  {#if showNoData && Object.keys(shortcuts.current).length === 0}
    <div class="pointer-events-none absolute inset-0 flex items-center justify-center">
      <NoData class="m-auto size-64 pl-4 opacity-10" />
    </div>
  {/if}
  {#each Object.keys(shortcuts.current).sort(compareShortcut) as shortcut (shortcut)}
    {@const mode = shortcuts.current[shortcut].mode}
    {@const rules = shortcuts.current[shortcut].rules}
    {@const disabled = shortcuts.current[shortcut].disabled}
    <div data-shortcut={shortcut} in:fly={{ x: -15, duration: 150 }} out:fly={{ x: 15, duration: 150 }}>
      <div class="mt-4 flex items-center border-t border-dashed pt-4 pb-2">
        <Shortcut {shortcut} class={disabled ? 'text-inactive/50' : 'text-shortcut'} />
        <button
          class="group ml-4 badge cursor-pointer bg-base-200 transition-all hover:opacity-100"
          class:opacity-50={disabled}
          class:opacity-80={!disabled}
          class:border={mode === 'toolbar'}
          class:gradient={mode === 'toolbar'}
          class:shadow-sm={mode === 'toolbar'}
          class:text-inactive={mode !== 'toolbar'}
          onclick={() => {
            // swap shortcut execution mode
            const s = shortcuts.current[shortcut];
            s.mode = s.mode === 'toolbar' ? 'quiet' : 'toolbar';
          }}
        >
          <label class="swap swap-rotate group-hover:swap-active">
            <ArrowsClockwiseIcon weight="bold" class="swap-on size-4" />
            <ArrowCircleRightIcon weight="bold" class="swap-off size-4" />
          </label>
          <span class="text-sm">
            {#if mode === 'toolbar'}
              {m.toolbar_mode()}
            {:else}
              {m.quiet_mode()}
            {/if}
          </span>
        </button>
        {#if mode === 'toolbar'}
          {@const layout = shortcuts.current[shortcut].toolbarLayout || 'horizontal'}
          <button
            class="group ml-1 badge cursor-pointer bg-base-200 opacity-60 transition-all hover:opacity-100"
            onclick={() => {
              const s = shortcuts.current[shortcut];
              s.toolbarLayout = (s.toolbarLayout || 'horizontal') === 'horizontal' ? 'vertical' : 'horizontal';
            }}
            title={layout === 'horizontal' ? m.toolbar_vertical() : m.toolbar_horizontal()}
          >
            {#if layout === 'horizontal'}
              <RowsIcon class="size-3.5" />
            {:else}
              <ColumnsIcon class="size-3.5" />
            {/if}
            <span class="text-xs">{layout === 'horizontal' ? m.toolbar_horizontal() : m.toolbar_vertical()}</span>
          </button>
        {/if}
        <Button
          icon={disabled ? ArrowClockwiseIcon : ProhibitInsetIcon}
          size="sm"
          class="ml-auto {disabled ? 'text-emphasis' : 'text-inactive'}"
          iconClass={disabled ? '' : 'rotate-90'}
          text={disabled ? m.enable_shortcut() : m.disable_shortcut()}
          onclick={() => {
            shortcuts.current[shortcut].disabled = !shortcuts.current[shortcut].disabled;
          }}
        />
        <Button
          icon={TrashIcon}
          size="sm"
          class="ml-1 text-emphasis"
          text={m.delete_shortcut()}
          onclick={() => {
            const clear = () => ruleBinder?.clear(shortcut);
            // delete directly if rule is empty, otherwise need confirmation
            if (rules.length > 0) {
              confirm({
                title: m.delete_shortcut_title({ shortcut: formatShortcut(shortcut) }),
                message: m.delete_confirm_message(),
                onconfirm: clear
              });
            } else {
              clear();
            }
          }}
        />
      </div>
      <!-- rules list with folder grouping -->
      <div class="mt-2 overflow-hidden rounded-box border shadow-xs">
        <div
          class="flex items-center justify-between gradient px-2 py-1"
          style="border-bottom: 1px inset var(--color-border)"
        >
          <span class="flex items-center gap-1 text-base-content/80">
            <Button class="swap swap-rotate {shortcuts.current[shortcut].collapsed ? '' : 'swap-active'}" onclick={() => (shortcuts.current[shortcut].collapsed = !shortcuts.current[shortcut].collapsed)}>
              <span class="swap-on"><SparkleIcon class="size-4.5" /></span>
              <span class="swap-off"><SparkleIcon class="size-4.5" /></span>
            </Button>
            <span class="text-sm tracking-wide opacity-60">
              {m.rule_count({ count: rules.filter((r) => !r.isFolder).length })}
            </span>
          </span>
          <span class="flex items-center gap-1">
            <Button
              icon={FolderPlusIcon}
              iconWeight="bold"
              text="{m.add()}{m.rule_group()}"
              class="text-emphasis"
              onclick={() => openFolderModal(shortcut)}
            />
            <Button
              icon={StackPlusIcon}
              iconWeight="bold"
              text="{m.add()}{m.rule()}"
              class="text-green-800"
              onclick={() => ruleBinder?.showModal(shortcut)}
            />
          </span>
        </div>
        {#if !shortcuts.current[shortcut].collapsed}
          <ul class="list overflow-y-auto bg-base-100 scrollbar-none [&_.list-row]:min-h-10 [&_.list-row]:py-1">
            {#if rules.length === 0}
              <li class="list-row mx-auto items-center gap-1 text-surface/35">
                {shortcutHint(shortcut)}
              </li>
            {/if}
            {#each rules as item, index (item.id)}
              {#if item.isFolder}
                <!-- folder header row -->
                {@const folderConfig = shortcuts.current[shortcut]?.groups?.[item.id]}
                {@const folderCollapsed = !!item.disabled}
                {@const childRules = rules.filter((r) => !r.isFolder && r.group === item.id)}
                <li class="list-row items-center rounded-none border-b border-dashed bg-base-150">
                  <button
                    class="flex cursor-pointer items-center gap-2 pl-2"
                    onclick={() => { item.disabled = !item.disabled; }}
                  >
                    <span class="text-xs opacity-40">{folderCollapsed ? '▶' : '▼'}</span>
                    {#if folderConfig?.icon}
                      <Icon icon={folderConfig.icon} class="size-5 shrink-0 opacity-70" />
                    {:else}
                      <FolderPlusIcon class="size-5 shrink-0 opacity-40" />
                    {/if}
                    <span class="text-sm font-medium opacity-70">{item.id}</span>
                    <span class="badge badge-xs opacity-30">{childRules.length}</span>
                  </button>
                  <span class="ml-auto flex items-center gap-1">
                    <Button
                      icon={GearSixIcon}
                      iconWeight="fill"
                      onclick={(event) => {
                        event.stopPropagation();
                        openFolderModal(shortcut, item.id);
                      }}
                    />
                    <Button
                      icon={TrashIcon}
                      onclick={(event) => {
                        event.stopPropagation();
                        confirm({
                          title: `${m.delete()}${m.rule_group()} [${item.id}]`,
                          message: m.delete_confirm_message(),
                          onconfirm: () => deleteFolder(shortcut, item.id)
                        });
                      }}
                    />
                  </span>
                </li>
                <!-- folder child rules (shown when expanded) -->
                {#if !folderCollapsed}
                  {#each childRules as child (child.id)}
                    {@const { label: caseLabel, icon: caseIcon } = ruleBinder?.getCaseOption(child.case) ?? {}}
                    {@const { label: actionLabel, icon: actionIcon } = ruleBinder?.getActionOption(child.action) ?? {}}
                    <li class="list-row items-center rounded-none pl-8 hover:bg-base-300" class:opacity-40={child.disabled}>
                      <div class="list-col-grow flex items-center gap-3">
                        <div class="flex items-center gap-1.5 truncate" title={actionLabel}>
                          {#if actionIcon}
                            <Icon icon={actionIcon} class="size-5 shrink-0" />
                          {/if}
                          <span class="truncate text-sm opacity-80">{actionLabel || child.action}</span>
                        </div>
                      </div>
                      <span class="flex items-center gap-1">
                        <Button
                          icon={GearSixIcon}
                          iconWeight="fill"
                          onclick={() => ruleUpdater?.showModal(shortcut, child.id)}
                        />
                        <Toggle
                          value={!child.disabled}
                          toggleClass="toggle-xs"
                          onchange={() => { child.disabled = !child.disabled; }}
                        />
                      </span>
                    </li>
                  {/each}
                {/if}
              {:else if !item.group}
                <!-- standalone rule (not in any folder) -->
                {@const { label: actionLabel, icon: actionIcon } = ruleBinder?.getActionOption(item.action) ?? {}}
                <li class="list-row items-center rounded-none hover:bg-base-300" class:opacity-40={item.disabled}>
                  <div class="list-col-grow flex items-center gap-1.5 pl-2" title={actionLabel}>
                    {#if actionIcon}
                      <Icon icon={actionIcon} class="size-5 shrink-0" />
                    {/if}
                    <span class="truncate text-sm opacity-80">{actionLabel || item.action}</span>
                  </div>
                  <span class="flex items-center gap-1">
                    <Button
                      icon={GearSixIcon}
                      iconWeight="fill"
                      onclick={() => ruleUpdater?.showModal(shortcut, item.id)}
                    />
                    <Toggle
                      value={!item.disabled}
                      toggleClass="toggle-xs"
                      onchange={() => { item.disabled = !item.disabled; }}
                    />
                  </span>
                </li>
              {/if}
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  {/each}
</div>

<Recorder bind:this={recorder} onrecord={register} />

<Binder bind:this={ruleBinder} />

<Binder bind:this={ruleUpdater} />

<BWList bind:this={blacklistManager} bind:list={blacklist.current} />

<Modal maxWidth="28rem" icon={FolderPlusIcon} title="{folderEditName ? m.update() : m.add()}{m.rule_group()}" bind:this={folderModal}>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      saveFolder();
    }}
  >
    <fieldset class="fieldset mt-4 flex flex-col gap-3">
      <div class="flex items-center gap-2">
        <label class="text-sm opacity-80 w-20 shrink-0">{m.type_name()}</label>
        <input class="autofocus input input-sm grow" required bind:value={folderName} />
      </div>
      <div class="flex items-center gap-2">
        <label class="text-sm opacity-80 w-20 shrink-0">{m.rule_group_icon()}</label>
        <IconSelector bind:icon={folderIcon} />
        <span class="truncate text-sm opacity-50">{folderIcon || ''}</span>
      </div>
      <div class="flex items-center gap-2">
        <label class="text-sm opacity-80 w-20 shrink-0">{m.toolbar_display()}</label>
        <div class="flex gap-3">
          <Radio bind:group={folderDisplayMode} value="both" label={m.icon_and_label()} labelClass="text-sm" radioClass="radio-sm" />
          <Radio bind:group={folderDisplayMode} value="icon" label={m.icon_only()} labelClass="text-sm" radioClass="radio-sm" />
          <Radio bind:group={folderDisplayMode} value="label" label={m.label_only()} labelClass="text-sm" radioClass="radio-sm" />
        </div>
      </div>
    </fieldset>
    <div class="modal-action">
      <button type="button" class="btn" onclick={() => folderModal?.close()}>{m.cancel()}</button>
      <button type="submit" class="btn btn-submit">{m.confirm()}</button>
    </div>
  </form>
</Modal>
