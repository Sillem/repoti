<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import Settings from './Settings.svelte';
  import { fmt, ping } from './util.js';

  const TEXT = {
    work: { title: 'WORK', down: 'Hold on for at least:', up: 'You were working for:', next: 'need a break' },
    break: { title: 'BREAK', down: 'Rest for at least:', up: 'You were resting for:', next: 'back to work' },
  };

  let view = $state(null);
  let saving = $state(false);
  let saveError = $state('');
  let error = $state('');
  let config = $state(null);

  let t = $derived(view ? TEXT[view.mode] : null);

  onMount(() => {
    invoke('get_state').then((v) => (view = v));
    const unlisten = [
      listen('tick', (e) => {
        if (!saving) view = e.payload;
      }),
      listen('threshold-reached', ping),
      listen('close-requested', quit),
    ];
    return () => unlisten.forEach((p) => p.then((f) => f()));
  });

  async function act(cmd) {
    if (saving) return;
    error = '';
    try {
      view = await invoke(cmd);
    } catch (e) {
      error = String(e);
    }
  }

  // Freeze the UI, persist the running session, then the backend exits the app.
  async function quit() {
    if (saving && !saveError) return;
    saving = true;
    saveError = '';
    try {
      await invoke('quit');
    } catch (e) {
      saveError = String(e);
    }
  }

  async function openSettings() {
    if (saving) return;
    config = await invoke('get_config');
  }

  async function settingsClosed() {
    config = null;
    view = await invoke('get_state');
  }
</script>

{#if view}
  <main class={view.mode}>
    <button class="icon corner" title="settings" onclick={openSettings} disabled={saving}>
      <svg viewBox="0 0 24 24"><path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z" /></svg>
    </button>

    <h1>{t.title}</h1>
    <p class="label">{view.counting_up ? t.up : t.down}</p>
    <p class="time" class:paused={view.paused}>{fmt(view.display_s)}</p>

    <div class="row">
      <button class="icon" title="reset this interval" onclick={() => act('reset')} disabled={saving}>
        <svg viewBox="0 0 24 24"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" /><path d="M3 3v5h5" /></svg>
      </button>
      <button class="icon" title={view.paused ? 'resume' : 'pause'} onclick={() => act('toggle_pause')} disabled={saving}>
        {#if view.paused}
          <svg viewBox="0 0 24 24"><polygon points="7 4 20 12 7 20 7 4" /></svg>
        {:else}
          <svg viewBox="0 0 24 24"><path d="M9 4v16M15 4v16" /></svg>
        {/if}
      </button>
      <button class="switch" onclick={() => act('switch_mode')} disabled={saving}>{t.next}</button>
    </div>

    <button class="done" onclick={quit} disabled={saving}>that's it for now</button>

    {#if error}<p class="error">{error}</p>{/if}

    {#if saving}
      <div class="overlay">
        {#if saveError}
          <p class="error">Could not save session:<br />{saveError}</p>
          <div class="row">
            <button class="done" onclick={quit}>retry</button>
            <button class="done" onclick={() => { saving = false; saveError = ''; }}>cancel</button>
          </div>
        {:else}
          <p>saving…</p>
        {/if}
      </div>
    {/if}

    {#if config}
      <Settings {config} onclose={settingsClosed} />
    {/if}
  </main>
{/if}
