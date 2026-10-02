// MM:SS below one hour, HH:MM:SS from one hour on.
export function fmt(total) {
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const p = (n) => String(n).padStart(2, '0');
  return h > 0 ? `${p(h)}:${p(m)}:${p(s)}` : `${p(m)}:${p(s)}`;
}

let ctx;

// Two short sine beeps generated on the fly, no audio asset needed.
export function ping() {
  try {
    ctx ??= new AudioContext();
    ctx.resume();
    for (const offset of [0, 0.25]) {
      const t = ctx.currentTime + offset;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.frequency.value = 880;
      gain.gain.setValueAtTime(0.0001, t);
      gain.gain.exponentialRampToValueAtTime(0.4, t + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.2);
      osc.connect(gain).connect(ctx.destination);
      osc.start(t);
      osc.stop(t + 0.22);
    }
  } catch (e) {
    console.error('ping failed', e);
  }
}
