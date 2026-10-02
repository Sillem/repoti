<script>
  import { untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import { parseDuration, formatDuration } from './util.js';

  let { config, onclose } = $props();
  // The form edits a draft; the config prop is only read once when it opens.
  const initial = untrack(() => config);

  let workText = $state(formatDuration(initial.work_threshold_s));
  let breakText = $state(formatDuration(initial.break_threshold_s));
  let workS = $derived(parseDuration(workText));
  let breakS = $derived(parseDuration(breakText));
  let minText = $state(formatDuration(initial.min_session_s));
  let minS = $derived(parseDuration(minText, true));
  let dataPath = $state(initial.data_path);
  let error = $state('');

  async function browse() {
    const picked = await save({
      defaultPath: dataPath,
      filters: [{ name: 'CSV', extensions: ['csv'] }],
    });
    if (picked) dataPath = picked;
  }

  async function submit(e) {
    e.preventDefault();
    error = '';
    if (!workS || !breakS || minS === null) {
      error = 'use e.g. 1h30m10s, 30m 10s or 45m';
      return;
    }
    try {
      await invoke('set_config', {
        cfg: {
          work_threshold_s: workS,
          break_threshold_s: breakS,
          data_path: dataPath,
          min_session_s: minS,
        },
      });
      onclose();
    } catch (err) {
      error = String(err);
    }
  }
</script>

<form class="overlay settings" onsubmit={submit}>
  <label>work <input type="text" class:invalid={!workS} placeholder="25m" bind:value={workText} required /></label>
  <label>break <input type="text" class:invalid={!breakS} placeholder="5m" bind:value={breakText} required /></label>
  <label title="sessions shorter than this are not saved">skip under <input type="text" class:invalid={minS === null} placeholder="5s" bind:value={minText} required /></label>
  <label class="path">
    data file
    <span><input type="text" bind:value={dataPath} required /><button type="button" onclick={browse}>…</button></span>
  </label>
  {#if error}<p class="error">{error}</p>{/if}
  <div class="row">
    <button type="submit" class="done">save</button>
    <button type="button" class="done" onclick={onclose}>cancel</button>
  </div>
</form>
