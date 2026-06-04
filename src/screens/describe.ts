import type { App } from '../state';
import { api, fileUrl } from '../api';

export function renderDescribe(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  const picUrl = app.round.picture ? fileUrl(app.round.picture.path) : '';
  el.innerHTML = `
    <div class="panel" style="width:min(58vw, calc(48vh * 16 / 9));aspect-ratio:16/9"><img src="${picUrl}" style="width:100%;height:100%;object-fit:cover;border-radius:16px"/></div>
    <div id="transcript" class="display" style="font-size:clamp(15px,3vmin,30px);min-height:44px;color:var(--ink);text-align:center;max-width:70vw">🎤 Starting… <span class="zh-line">正在开启麦克风…</span></div>
    <div style="display:flex;gap:18px">
      <button class="btn pink" id="mic" disabled><span class="en">⏹ Stop</span><span class="zh">停止</span></button>
      <button class="btn mint hidden" id="gen"><span class="en">Make it! ✨</span><span class="zh">生成图片</span></button>
    </div>`;
  root.appendChild(el);
  const tEl = el.querySelector<HTMLDivElement>('#transcript')!;
  const mic = el.querySelector<HTMLButtonElement>('#mic')!;
  const gen = el.querySelector<HTMLButtonElement>('#gen')!;

  let recording = false;

  async function startRec() {
    mic.disabled = true;
    tEl.innerHTML = '🎤 Starting… <span class="zh-line">正在开启麦克风…</span>';
    mic.innerHTML = '<span class="en">⏹ Stop</span><span class="zh">停止</span>';
    gen.classList.add('hidden');
    try {
      await api.asrStart();
      recording = true;
      mic.disabled = false;
      tEl.innerHTML = '🔴 Recording… <span class="zh-line">录音中…说完点「停止」</span>';
    } catch (e) {
      app.showError(String(e));
    }
  }

  async function stopRec() {
    recording = false;
    mic.disabled = true;
    tEl.innerHTML = '⏳ Transcribing… <span class="zh-line">识别中…</span>';
    try {
      const t = await api.asrStop();
      app.round.transcript = t;
      tEl.textContent = t || "(没听清，请点「再说一次」/ didn't catch that, tap Talk again)";
    } catch (e) {
      app.showError(String(e));
      return;
    }
    mic.disabled = false;
    mic.innerHTML = '<span class="en">🎤 Talk again</span><span class="zh">再说一次</span>';
    gen.classList.remove('hidden');
  }

  mic.onclick = () => { if (recording) stopRec(); else startRec(); };
  gen.onclick = () => { app.go('generating'); };

  // Auto-start recording on entering the screen (no extra "Start" tap).
  startRec();
}
