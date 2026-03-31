<script lang="ts">
  import { alert, Binder, Button, BWList, confirm, Icon, IconSelector, List, Modal, Radio, Recorder, Shortcut, Toggle } from '$lib/components';
  import { DBCLICK_SHORTCUT, DRAG_SHORTCUT, SHIFT_CLICK_SHORTCUT } from '$lib/constants';
  import { formatShortcut, isMouseShortcut } from '$lib/helpers';
  import { NoData } from '$lib/icons';
  import { m } from '$lib/paraglide/messages';
  import { blacklist, longPress, shortcuts } from '$lib/stores.svelte';
  import type { DisplayMode } from '$lib/types';
  import {
    ArrowArcRightIcon,
    ArrowCircleRightIcon,
    ArrowClockwiseIcon,
    ArrowFatLineRightIcon,
    ArrowFatUpIcon,
    ArrowsClockwiseIcon,
    BrowserIcon,
    ColumnsIcon,
    CursorClickIcon,
    FolderPlusIcon,
    GearSixIcon,
    KeyboardIcon,
    MouseLeftClickIcon,
    PencilSimpleIcon,
    ProhibitIcon,
    ProhibitInsetIcon,
    RowsIcon,
    SparkleIcon,
    StackPlusIcon,
    TrashIcon,
    WarningIcon,
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
            shortcuts.current[SHIFT_CLICK_SHORTCUT]
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
      <List
        name={m.rule()}
        hint={shortcutHint(shortcut)}
        bind:data={shortcuts.current[shortcut].rules}
        bind:collapsed={shortcuts.current[shortcut].collapsed}
        collapsible
        oncreate={() => ruleBinder?.showModal(shortcut)}
        ondelete={(item) => {
          if (item.isFolder) {
            // delete folder config
            if (shortcuts.current[shortcut].groups) {
              delete shortcuts.current[shortcut].groups[item.id];
            }
          } else {
            ruleBinder?.unbind(item);
          }
        }}
      >
        {#snippet title()}
          <SparkleIcon class="mx-1 size-4 opacity-60" />
          <span class="text-sm tracking-wide opacity-60">
            {#if rules.length > 0}
              {m.rule_count({ count: rules.filter((r) => !r.isFolder).length })}
            {:else}
              {m.rule_empty()}
            {/if}
          </span>
          {#if mode === 'toolbar'}
            <button
              class="ml-2 flex cursor-pointer items-center gap-0.5 text-xs opacity-40 transition-opacity hover:opacity-80"
              onclick={(e) => {
                e.stopPropagation();
                openFolderModal(shortcut);
              }}
            >
              <FolderPlusIcon class="size-3.5" />
              {m.add()}{m.rule_group()}
            </button>
          {/if}
        {/snippet}
        {#snippet row(item)}
          {#if item.isFolder}
            <!-- folder marker row -->
            {@const folderConfig = shortcuts.current[shortcut]?.groups?.[item.id]}
            <div class="list-col-grow flex items-center gap-2 pl-4">
              {#if folderConfig?.icon}
                <Icon icon={folderConfig.icon} class="size-5 shrink-0 opacity-70" />
              {:else}
                <FolderPlusIcon class="size-5 shrink-0 opacity-40" />
              {/if}
              <span class="text-sm font-medium opacity-70">{item.id}</span>
              <span class="badge badge-xs opacity-40">{m.rule_group()}</span>
            </div>
            <Button
              icon={GearSixIcon}
              iconWeight="fill"
              onclick={(event) => {
                event.stopPropagation();
                openFolderModal(shortcut, item.id);
              }}
            />
          {:else}
            <!-- regular rule row -->
            {@const { label: caseLabel, icon: caseIcon } = ruleBinder?.getCaseOption(item.case) ?? {}}
            {@const { label: actionLabel, icon: actionIcon } = ruleBinder?.getActionOption(item.action) ?? {}}
            <div
              class="list-col-grow grid grid-cols-12 items-center gap-4 pl-4"
              class:opacity-40={item.disabled}
            >
            <div class="col-span-5 flex items-center gap-1.5" title={caseLabel}>
              {#if item.case === ''}
                <!-- default type -->
                <ArrowArcRightIcon class="size-5 shrink-0 opacity-30" />
                <span class="truncate opacity-30">{caseLabel}</span>
              {:else if !caseLabel}
                <!-- invalid type -->
                <WarningIcon class="size-5 shrink-0 opacity-50" />
                <span class="truncate line-through opacity-50">
                  {item.case.substring(item.case.indexOf('-') + 1)}
                </span>
              {:else}
                <!-- valid type -->
                {#if caseIcon}
                  <Icon icon={caseIcon} class="size-5 shrink-0" />
                {/if}
                <span class="truncate opacity-80">{caseLabel}</span>
              {/if}
            </div>
            <div class="col-span-1 flex items-center justify-center">
              <ArrowFatLineRightIcon class="size-5 shrink-0 opacity-15" />
            </div>
            <div class="col-span-6 flex items-center gap-1.5" title={actionLabel}>
              {#if item.action === ''}
                <!-- default action -->
                <BrowserIcon class="size-5 shrink-0 opacity-30" />
                <span class="truncate opacity-30">{actionLabel}</span>
              {:else if !actionLabel}
                <!-- invalid action -->
                <WarningIcon class="size-5 shrink-0 opacity-50" />
                <span class="truncate line-through opacity-50">
                  {item.action.substring(item.action.indexOf('-') + 1)}
                </span>
              {:else}
                <!-- valid action -->
                {#if actionIcon}
                  <Icon icon={actionIcon} class="size-5 shrink-0" />
                {/if}
                <span class="truncate opacity-80">{actionLabel}</span>
              {/if}
            </div>
          </div>
          <Button
            icon={item.disabled ? ProhibitInsetIcon : ProhibitInsetIcon}
            iconClass={item.disabled ? 'rotate-90 text-error/60' : 'rotate-90 opacity-30'}
            onclick={(event) => {
              event.stopPropagation();
              item.disabled = !item.disabled;
            }}
          />
          <Button
            icon={GearSixIcon}
            iconWeight="fill"
            onclick={(event) => {
              event.stopPropagation();
              ruleUpdater?.showModal(shortcut, item.id);
            }}
          />
          {/if}
        {/snippet}
      </List>
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
