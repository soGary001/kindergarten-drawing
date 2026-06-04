import type { App } from '../state';
import { api, fileUrl } from '../api';

export function renderDescribe(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  const picUrl = app.round.picture ? fileUrl(app.round.picture.path) : '';
  el.innerHTML = `
    <div class="panel" style="width:min(58vw, calc(42vh * 16 / 9));aspect-ratio:16/9"><img src="${picUrl}" style="width:100%;height:100%;object-fit:cover;border-radius:16px"/></div>
    <div id="status" class="display" style="font-size:clamp(15px,3vmin,28px);min-height:36px;color:var(--ink);text-align:center;max-width:80vw">🎤 Starting… <span class="zh-line">正在开启麦克风…</span></div>
    <div id="timer" class="display" style="font-size:clamp(32px,8vmin,72px);color:var(--mint);min-height:1.1em"></div>
    <button class="btn pink hidden" id="gen"><span class="en">✨ Generate</span><span class="zh">生成图片</span></button>`;
  root.appendChild(el);
  const statusEl = el.querySelector<HTMLDivElement>('#status')!;
  const timerEl = el.querySelector<HTMLDivElement>('#timer')!;
  const gen = el.querySelector<HTMLButtonElement>('#gen')!;

  const DURATION = 60;
  let remaining = DURATION;
  let ticker: number | undefined;
  let done = false;

  function renderTimer() {
    timerEl.textContent = `⏱ ${remaining}`;
    timerEl.style.color = remaining <= 10 ? 'var(--pink)' : 'var(--mint)';
  }

  async function start() {
    done = false;
    gen.classList.add('hidden');
    gen.disabled = false;
    timerEl.textContent = '';
    statusEl.innerHTML = '🎤 Starting… <span class="zh-line">正在开启麦克风…</span>';
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
    timerEl.textContent = '';
    statusEl.innerHTML = '⏳ Transcribing… <span class="zh-line">识别中…</span>';
    let t = '';
    try {
      t = await api.asrStop();
    } catch (e) {
      app.showError(String(e));
      return;
    }
    app.round.transcript = t;
    if (!t) {
      // Heard nothing — give another try.
      statusEl.textContent = "(没听清，再说一次吧 / didn't catch that — let's try again)";
      start();
      return;
    }
    statusEl.innerHTML = `<span class="said display" style="font-size:clamp(18px,3.6vmin,32px);color:var(--pink)"></span><span class="zh-line">✨ Making your picture… 正在生成图片…</span>`;
    statusEl.querySelector<HTMLSpanElement>('.said')!.textContent = `“${t}”`;
    setTimeout(() => app.go('generating'), 1400);
  }

  gen.onclick = () => finalize();

  // Auto-start recording + countdown on entering the screen.
  start();
}
