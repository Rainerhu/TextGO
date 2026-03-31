<script lang="ts">
  import { Button, Label, Select, Setting, Toggle, alert, confirm } from '$lib/components';
  import { setupTray } from '$lib/helpers';
  import { m } from '$lib/paraglide/messages';
  import { getLocale, setLocale, type Locale } from '$lib/paraglide/runtime';
  import { manager } from '$lib/shortcut';
  import {
    accessibility,
    autoStart,
    autoUpdate,
    blacklist,
    historySize,
    iBeamCursor,
    lazySelection,
    longPress,
    longPressDuration,
    minimizeToTray,
    models,
    nativeSelectionOnly,
    popupDefaultSize,
    popupRememberSize,
    prompts,
    regexps,
    scripts,
    searchers,
    shortcuts,
    theme
  } from '$lib/stores.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { save, open as openDialog } from '@tauri-apps/plugin-dialog';
  import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
  import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
  import { type } from '@tauri-apps/plugin-os';
  import {
    AppWindowIcon,
    ArrowSquareInIcon,
    ArrowSquareOutIcon,
    CheckCircleIcon,
    ClockCounterClockwiseIcon,
    ExportIcon,
    MonitorIcon,
    ShieldCheckIcon,
    WarningCircleIcon
  } from 'phosphor-svelte';
  import { onMount } from 'svelte';

  // operating system type
  const osType = type();

  // current language
  let locale: Locale = $state(getLocale());

  /**
   * Toggle auto start status.
   */
  async function toggleAutoStart(enabled: boolean) {
    try {
      if (enabled) {
        await enable();
      } else {
        await disable();
      }
      autoStart.current = enabled;
    } catch (error) {
      console.error(`Failed to toggle auto start status: ${error}`);
      autoStart.current = !enabled;
    }
  }

  /**
   * Export all configuration to a JSON file.
   */
  async function exportConfig() {
    try {
      const data = {
        _format: 'textgo-config',
        _version: 1,
        shortcuts: shortcuts.current,
        blacklist: blacklist.current,
        models: models.current,
        regexps: regexps.current,
        scripts: scripts.current,
        prompts: prompts.current,
        searchers: searchers.current,
        settings: {
          theme: theme.current,
          historySize: historySize.current,
          longPress: longPress.current,
          longPressDuration: longPressDuration.current,
          iBeamCursor: iBeamCursor.current,
          lazySelection: lazySelection.current,
          nativeSelectionOnly: nativeSelectionOnly.current,
          popupRememberSize: popupRememberSize.current,
          popupDefaultSize: popupDefaultSize.current,
          minimizeToTray: minimizeToTray.current
        }
      };

      const path = await save({
        defaultPath: 'textgo-config.json',
        filters: [{ name: 'JSON', extensions: ['json'] }]
      });
      if (!path) return;

      await writeTextFile(path, JSON.stringify(data, null, 2));
      alert(m.export_success());
    } catch (error) {
      console.error(`Failed to export config: ${error}`);
      alert({ level: 'error', message: m.export_failed() });
    }
  }

  /**
   * Import configuration from a JSON file.
   */
  async function importConfig() {
    try {
      const path = await openDialog({
        multiple: false,
        filters: [{ name: 'JSON', extensions: ['json'] }]
      });
      if (!path) return;

      const content = await readTextFile(path);
      const data = JSON.parse(content);

      if (data._format !== 'textgo-config' || !data._version) {
        alert({ level: 'error', message: m.import_invalid_format() });
        return;
      }

      confirm({
        title: m.import_confirm_title(),
        message: m.import_confirm_message(),
        onconfirm: async () => {
          try {
            // import shortcuts and re-register rules
            if (data.shortcuts) {
              // unregister all existing keyboard shortcuts
              for (const s of Object.values(shortcuts.current) as { rules: { shortcut: string }[] }[]) {
                for (const rule of s.rules) {
                  try { await invoke('unregister_shortcut', { shortcut: rule.shortcut }); } catch { /* ignore */ }
                }
              }
              shortcuts.current = data.shortcuts;
              // re-register all shortcuts
              for (const s of Object.values(shortcuts.current) as { rules: { shortcut: string }[] }[]) {
                for (const rule of s.rules) {
                  await manager.register(rule as any);
                }
              }
            }
            if (data.blacklist) blacklist.current = data.blacklist;
            if (data.models) models.current = data.models;
            if (data.regexps) regexps.current = data.regexps;
            if (data.scripts) scripts.current = data.scripts;
            if (data.prompts) prompts.current = data.prompts;
            if (data.searchers) searchers.current = data.searchers;

            // import settings
            if (data.settings) {
              const s = data.settings;
              if (s.theme !== undefined) theme.current = s.theme;
              if (s.historySize !== undefined) historySize.current = s.historySize;
              if (s.longPress !== undefined) longPress.current = s.longPress;
              if (s.longPressDuration !== undefined) longPressDuration.current = s.longPressDuration;
              if (s.iBeamCursor !== undefined) iBeamCursor.current = s.iBeamCursor;
              if (s.lazySelection !== undefined) lazySelection.current = s.lazySelection;
              if (s.nativeSelectionOnly !== undefined) nativeSelectionOnly.current = s.nativeSelectionOnly;
              if (s.popupRememberSize !== undefined) popupRememberSize.current = s.popupRememberSize;
              if (s.popupDefaultSize !== undefined) popupDefaultSize.current = s.popupDefaultSize;
              if (s.minimizeToTray !== undefined) minimizeToTray.current = s.minimizeToTray;
            }

            alert(m.import_success());
          } catch (error) {
            console.error(`Failed to apply imported config: ${error}`);
            alert({ level: 'error', message: m.import_failed() });
          }
        }
      });
    } catch (error) {
      console.error(`Failed to import config: ${error}`);
      alert({ level: 'error', message: m.import_failed() });
    }
  }

  // check auto start status on mount
  onMount(async () => {
    try {
      autoStart.current = await isEnabled();
      accessibility.current = await invoke<boolean>('check_accessibility');
    } catch (error) {
      console.error(`Failed to check auto start status: ${error}`);
    }
  });
</script>

<div class="flex flex-col gap-2">
  <Setting icon={MonitorIcon} title={m.appearance_settings()}>
    <fieldset class="flex items-center justify-between gap-1">
      <Label>{m.language_settings()}</Label>
      <Select
        value={locale}
        options={[
          { value: 'en', label: 'English' },
          { value: 'zh-CN', label: '简体中文' }
        ]}
        class="w-36 select-sm"
        onchange={async (event) => {
          const target = event.currentTarget;
          locale = target.value as Locale;
          setLocale(locale);
          await setupTray();
        }}
      />
    </fieldset>
    <div class="divider my-0 opacity-60"></div>
    <fieldset class="flex items-center justify-between gap-1">
      <Label>{m.theme_settings()}</Label>
      <Select
        options={[
          { value: 'light', label: m.light_theme() },
          { value: 'dark', label: m.dark_theme() },
          { value: 'system', label: m.system_theme() }
        ]}
        bind:value={theme.current}
        class="w-36 select-sm"
      />
    </fieldset>
  </Setting>
  <Setting icon={ClockCounterClockwiseIcon} title={m.history_records()}>
    <fieldset class="flex items-center justify-between gap-1">
      <Label>{m.history_records_retention()}</Label>
      <Select
        options={[
          { value: 0, label: m.history_none() },
          { value: 3, label: m.history_recent_3() },
          { value: 5, label: m.history_recent_5() },
          { value: 10, label: m.history_recent_10() },
          { value: 20, label: m.history_recent_20() }
        ]}
        bind:value={historySize.current}
        class="w-36 select-sm"
      />
    </fieldset>
  </Setting>
  <Setting icon={AppWindowIcon} title={m.popup_settings()}>
    <fieldset class="flex items-center justify-between gap-1">
      <Label tip={m.popup_remember_size_explain()} tipPlacement="duplex">{m.popup_remember_size()}</Label>
      <Toggle bind:value={popupRememberSize.current} />
    </fieldset>
    <div class="divider my-0 opacity-60"></div>
    <fieldset class="flex items-center justify-between gap-1">
      <Label>{m.popup_default_width()}</Label>
      <input
        type="number"
        class="input w-24 input-sm"
        min="200"
        max="1200"
        step="50"
        bind:value={popupDefaultSize.current.width}
      />
    </fieldset>
    <div class="divider my-0 opacity-60"></div>
    <fieldset class="flex items-center justify-between gap-1">
      <Label>{m.popup_default_height()}</Label>
      <input
        type="number"
        class="input w-24 input-sm"
        min="150"
        max="900"
        step="50"
        bind:value={popupDefaultSize.current.height}
      />
    </fieldset>
  </Setting>
  <Setting icon={ShieldCheckIcon} title={m.behavior_settings()}>
    {#if osType === 'macos'}
      <fieldset class="flex items-center justify-between gap-1">
        <Label tip={m.accessibility_explain()} tipPlacement="duplex">{m.accessibility()}</Label>
        {#if accessibility.current}
          <div class="badge bg-base-200 text-emphasis">
            <CheckCircleIcon class="size-4" />
            <span class="text-sm">{m.permission_granted()}</span>
          </div>
        {:else}
          <Button
            icon={WarningCircleIcon}
            text={m.request_permission()}
            square={false}
            class="border-emphasis/30 bg-base-200 text-emphasis"
            onclick={() => invoke('open_accessibility')}
          />
        {/if}
      </fieldset>
      <div class="divider my-0 opacity-60"></div>
    {/if}
    <fieldset class="flex items-center justify-between gap-1">
      <Label>{m.auto_update()}</Label>
      <Toggle bind:value={autoUpdate.current} />
    </fieldset>
    <div class="divider my-0 opacity-60"></div>
    <fieldset class="flex items-center justify-between gap-1">
      <Label>{m.auto_start()}</Label>
      <Toggle value={autoStart.current} onchange={(event) => toggleAutoStart(event.currentTarget.checked)} />
    </fieldset>
    <div class="divider my-0 opacity-60"></div>
    <fieldset class="flex items-center justify-between gap-1">
      <Label>{m.minimize_to_tray()}</Label>
      <Toggle bind:value={minimizeToTray.current} />
    </fieldset>
  </Setting>
  <Setting icon={ExportIcon} title={m.data_management()}>
    <fieldset class="flex items-center justify-between gap-1">
      <Label tip={m.export_explain()} tipPlacement="duplex">{m.export_config()}</Label>
      <Button
        icon={ArrowSquareOutIcon}
        text={m.export()}
        square={false}
        class="border-emphasis/30 bg-base-200 text-emphasis"
        onclick={exportConfig}
      />
    </fieldset>
    <div class="divider my-0 opacity-60"></div>
    <fieldset class="flex items-center justify-between gap-1">
      <Label tip={m.import_explain()} tipPlacement="duplex">{m.import_config()}</Label>
      <Button
        icon={ArrowSquareInIcon}
        text={m.import()}
        square={false}
        class="border-emphasis/30 bg-base-200 text-emphasis"
        onclick={importConfig}
      />
    </fieldset>
  </Setting>
</div>
