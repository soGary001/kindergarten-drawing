// Celebratory + background sounds via Web Audio (no audio files, works offline).
let _ctx: AudioContext | null = null;
function ctx(): AudioContext {
  if (!_ctx) {
    const C = (window.AudioContext || (window as any).webkitAudioContext) as typeof AudioContext;
    _ctx = new C();
  }
  if (_ctx.state === "suspended") _ctx.resume();
  return _ctx;
}

export function playCelebration() {
  try {
    const ac = ctx();
    const now = ac.currentTime;
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
    /* ignore */
  }
}

// Gentle looping melody to play while the AI is drawing. Returns a stop function.
let musicTimer: number | undefined;
export function startEncourageMusic(): () => void {
  stopEncourageMusic();
  try {
    const ac = ctx();
    const melody = [523.25, 587.33, 659.25, 783.99, 659.25, 587.33, 523.25, 659.25];
    let i = 0;
    const playNote = () => {
      try {
        const o = ac.createOscillator();
        const g = ac.createGain();
        o.type = "sine";
        o.frequency.value = melody[i % melody.length];
        i++;
        const t = ac.currentTime;
        g.gain.setValueAtTime(0.0001, t);
        g.gain.exponentialRampToValueAtTime(0.16, t + 0.05);
        g.gain.exponentialRampToValueAtTime(0.0001, t + 0.5);
        o.connect(g);
        g.connect(ac.destination);
        o.start(t);
        o.stop(t + 0.55);
      } catch {
        /* ignore */
      }
    };
    playNote();
    musicTimer = window.setInterval(playNote, 430);
  } catch {
    /* ignore */
  }
  return stopEncourageMusic;
}

export function stopEncourageMusic() {
  if (musicTimer !== undefined) {
    clearInterval(musicTimer);
    musicTimer = undefined;
  }
}
