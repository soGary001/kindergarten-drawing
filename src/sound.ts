// Tiny celebratory "ta-da" using the Web Audio API — no audio file needed, works offline.
let _ctx: AudioContext | null = null;
function ctx(): AudioContext {
  if (!_ctx) {
    const C = (window.AudioContext || (window as any).webkitAudioContext) as typeof AudioContext;
    _ctx = new C();
  }
  return _ctx;
}

export function playCelebration() {
  try {
    const ac = ctx();
    if (ac.state === "suspended") ac.resume();
    const now = ac.currentTime;
    // ascending arpeggio C5-E5-G5-C6 + a little sparkle
    const notes = [523.25, 659.25, 783.99, 1046.5];
    notes.forEach((f, i) => {
      const o = ac.createOscillator();
      const g = ac.createGain();
      o.type = "triangle";
      o.frequency.value = f;
      const t = now + i * 0.11;
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(0.32, t + 0.03);
      g.gain.exponentialRampToValueAtTime(0.0001, t + 0.38);
      o.connect(g);
      g.connect(ac.destination);
      o.start(t);
      o.stop(t + 0.42);
    });
  } catch {
    /* audio not available — ignore */
  }
}
