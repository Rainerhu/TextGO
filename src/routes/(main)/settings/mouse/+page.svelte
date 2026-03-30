<script lang="ts">
  import { Label, Setting, Toggle } from '$lib/components';
  import { m } from '$lib/paraglide/messages';
  import { iBeamCursor, lazySelection, longPress, longPressDuration, nativeSelectionOnly } from '$lib/stores.svelte';
  import { CursorClickIcon, CursorTextIcon, SelectionIcon } from 'phosphor-svelte';
</script>

<div class="flex flex-col gap-2">
  <Setting icon={CursorTextIcon} title={m.cursor_settings()}>
    <fieldset class="flex items-center justify-between gap-1">
      <Label tip={m.ibeam_cursor_explain()} tipPlacement="duplex">{m.ibeam_cursor_enabled()}</Label>
      <Toggle bind:value={iBeamCursor.current} />
    </fieldset>
  </Setting>
  <Setting icon={SelectionIcon} title={m.lazy_selection_settings()}>
    <fieldset class="flex items-center justify-between gap-1">
      <Label tip={m.lazy_selection_explain()} tipPlacement="duplex">{m.lazy_selection_enabled()}</Label>
      <Toggle bind:value={lazySelection.current} />
    </fieldset>
    <div class="divider my-0 opacity-60"></div>
    <fieldset class="flex items-center justify-between gap-1">
      <Label tip={m.native_selection_only_explain()} tipPlacement="duplex">{m.native_selection_only_enabled()}</Label>
      <Toggle bind:value={nativeSelectionOnly.current} />
    </fieldset>
  </Setting>
  <Setting icon={CursorClickIcon} title={m.long_press_settings()}>
    <fieldset class="flex items-center justify-between gap-1">
      <Label tip={m.long_press_explain()} tipPlacement="duplex">{m.long_press_enabled()}</Label>
      <Toggle bind:value={longPress.current} />
    </fieldset>
    <div class="divider my-0 opacity-60"></div>
    <fieldset class="flex items-center justify-between gap-1">
      <Label>{m.long_press_duration()}</Label>
      <label class="flex max-w-2/5 grow flex-col gap-2 pt-2">
        <input
          class="range w-full text-emphasis range-xs"
          type="range"
          min="500"
          max="2000"
          step="100"
          bind:value={longPressDuration.current}
        />
        <div class="flex justify-between text-xs opacity-70">
          <span>0.5s</span>
          <span>1.0s</span>
          <span>1.5s</span>
          <span>2.0s</span>
        </div>
      </label>
    </fieldset>
  </Setting>
</div>
