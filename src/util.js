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

const UNIT = { h: 3600, m: 60, s: 1 };

// "1h30m10s", "30m 10s", "45m", "90s"; a bare number means minutes.
// Returns seconds, or null if the text is not a valid positive duration
// (zero is accepted only with allowZero).
export function parseDuration(text, allowZero = false) {
  const t = text.trim().toLowerCase();
  const ok = (n) => (n || allowZero ? n : null);
  if (/^\d+$/.test(t)) return ok(Number(t) * 60);
  if (!/^(\d+\s*[hms]\s*)+$/.test(t)) return null;
  let total = 0;
  const seen = new Set();
  for (const [, n, u] of t.matchAll(/(\d+)\s*([hms])/g)) {
    if (seen.has(u)) return null;
    seen.add(u);
    total += Number(n) * UNIT[u];
  }
  return ok(total);
}

// Inverse of parseDuration, e.g. 5410 -> "1h 30m 10s".
export function formatDuration(total) {
  const parts = [];
  for (const [u, n] of Object.entries(UNIT)) {
    const v = Math.floor(total / n);
    total %= n;
    if (v) parts.push(`${v}${u}`);
  }
  return parts.join(' ') || '0s';
}
