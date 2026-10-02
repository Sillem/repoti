<script>
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';

  let { config, onclose } = $props();

  let workMin = $state(config.work_threshold_s / 60);
  let breakMin = $state(config.break_threshold_s / 60);
  let dataPath = $state(config.data_path);
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
    try {
      await invoke('set_config', {
        cfg: {
          work_threshold_s: Math.round(workMin * 60),
          break_threshold_s: Math.round(breakMin * 60),
          data_path: dataPath,
        },
      });
      onclose();
    } catch (err) {
      error = String(err);
    }
  }
</script>

<form class="overlay settings" onsubmit={submit}>
  <label>work (min) <input type="number" min="1" max="600" step="1" bind:value={workMin} required /></label>
  <label>break (min) <input type="number" min="1" max="600" step="1" bind:value={breakMin} required /></label>
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
