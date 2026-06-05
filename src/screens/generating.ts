import type { App } from '../state';
import { api } from '../api';
export function renderGenerating(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  el.innerHTML = `
    <div class="display spin" style="font-size:clamp(46px,11vmin,90px)">🎨</div>
    <h1 style="color:var(--pink);font-size:clamp(22px,5.2vmin,44px)">AI is painting…<span class="zh-sub">AI 正在画画…</span></h1>
    <p class="display" style="font-size:clamp(14px,2.8vmin,24px);color:var(--lav)">"${app.round.transcript || '...'}"</p>`;
  root.appendChild(el);
  (async () => {
    try {
      const res = await api.generateImage(app.round.transcript);
      app.round.generatedPath = res.path;
      app.go('compare');
    } catch (e) { app.showError(String(e)); }
  })();
}
