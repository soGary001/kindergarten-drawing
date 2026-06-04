import type { App } from '../state';
import { api, fileUrl, onEvent } from '../api';

export function renderDescribe(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  const picUrl = app.round.picture ? fileUrl(app.round.picture.path) : '';
  el.innerHTML = `
    <div class="panel" style="width:min(54vw, calc(36vh * 16 / 9));aspect-ratio:16/9"><img src="${picUrl}" style="width:100%;height:100%;object-fit:cover;border-radius:16px"/></div>
    <div id="status" class="display" style="font-size:clamp(13px,2.2vmin,20px);color:var(--lav);min-height:22px;text-align:center">🎤 Starting… 正在开启麦克风…</div>
    <div id="words" style="font-family:var(--font-display);font-weight:700;font-size:clamp(16px,3vmin,30px);color:var(--ink);max-width:82vw;max-height:24vh;overflow-y:auto;min-height:1.4em;text-align:center;line-height:1.35"></div>
    <div id="timer" class="display" style="font-size:clamp(28px,6vmin,56px);color:var(--mint);min-height:1.1em"></div>
    <button class="btn pink hidden" id="gen"><span class="en">✨ Generate</span><span class="zh">生成图片</span></button>`;
  root.appendChild(el);
  const statusEl = el.querySelector<HTMLDivElement>('#status')!;
  const wordsEl = el.querySelector<HTMLDivElement>('#words')!;
  const timerEl = el.querySelector<HTMLDivElement>('#timer')!;
  const gen = el.querySelector<HTMLButtonElement>('#gen')!;

  const DURATION = 60;
  let remaining = DURATION;
  let ticker: number | undefined;
  let done = false;
  let finals: string[] = [];
  let partial = '';
  const unlisten: Array<Promise<() => void>> = [];

  function render() {
    const confirmed = finals.join(' ');
    if (!confirmed && !partial) {
      wordsEl.textContent = '… 👂';
    } else {
      wordsEl.innerHTML = '';
      const c = document.createElement('span');
      c.textContent = confirmed ? confirmed + (partial ? ' ' : '') : '';
      const p = document.createElement('span');
      p.style.opacity = '0.45';
      p.textContent = partial;
      wordsEl.appendChild(c);
      wordsEl.appendChild(p);
      wordsEl.scrollTop = wordsEl.scrollHeight;
    }
    app.round.transcript = confirmed;
  }

  function renderTimer() {
    timerEl.textContent = `⏱ ${remaining}`;
    timerEl.style.color = remaining <= 10 ? 'var(--pink)' : 'var(--mint)';
  }

  unlisten.push(onEvent<string>('asr://partial', (s) => { partial = s; render(); }));
  unlisten.push(onEvent<string>('asr://final', (s) => { if (s.trim()) finals.push(s.trim()); partial = ''; render(); }));
  unlisten.push(onEvent<string>('asr://error', (e) => { statusEl.textContent = '⚠️ ' + e; }));

  function cleanup() { unlisten.forEach((u) => u.then((f) => f())); }

  async function start() {
    statusEl.textContent = '🎤 Starting… 正在开启麦克风…';
    try {
      await api.asrStart();
    } catch (e) {
      app.showError(String(e));
      return;
    }
    statusEl.innerHTML = '🔴 Recording — keep describing! <span class="zh-line">录音中…可以一直说,说完点「生成图片」</span>';
    gen.classList.remove('hidden');
    remaining = DURATION;
    renderTimer();
    ticker = window.setInterval(() => {
      remaining -= 1;
      renderTimer();
      if (remaining <= 0) finalize();
    }, 1000);
  }

  async function finalize() {
    if (done) return;
    done = true;
    if (ticker !== undefined) { clearInterval(ticker); ticker = undefined; }
    gen.disabled = true;
    statusEl.innerHTML = '⏳ Finishing… <span class="zh-line">整理中…</span>';
    try { await api.asrStop(); } catch { /* ignore */ }
    await new Promise((r) => setTimeout(r, 1200)); // catch trailing final sentence(s)
    const text = finals.join(' ').trim();
    app.round.transcript = text;
    if (!text) {
      // Heard nothing — let the child try again (keep listeners + same session loop).
      statusEl.textContent = "(没听清，再说一次吧 / let's try again)";
      done = false;
      gen.disabled = false;
      finals = [];
      partial = '';
      render();
      start();
      return;
    }
    cleanup();
    app.go('generating');
  }

  gen.onclick = () => finalize();

  render();
  start();
}
