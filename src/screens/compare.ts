import type { App } from '../state';
import { api, fileUrl } from '../api';
import { composeSnapshot } from '../snapshot';

export function renderCompare(root: HTMLElement, app: App) {
  const orig = app.round.picture ? fileUrl(app.round.picture.path) : '';
  const gen = app.round.generatedPath ? fileUrl(app.round.generatedPath) : '';
  const el = document.createElement('div'); el.className = 'screen';
  el.innerHTML = `
    <h1 style="color:var(--pink);font-size:clamp(20px,4.8vmin,40px)">Look what you made! 🎉<span class="zh-sub">看看你创造了什么！</span></h1>
    <div style="display:flex;gap:22px;align-items:center;width:88vw;max-width:96vw;flex-wrap:nowrap">
      <div class="panel" style="flex:1"><div class="display" style="text-align:center;color:var(--pink);margin-bottom:6px">THE PICTURE 原图</div>
        <img src="${orig}" style="width:100%;aspect-ratio:16/9;max-height:40vh;object-fit:cover;border-radius:14px"/></div>
      <div class="display" style="font-size:clamp(18px,4.5vmin,40px);color:var(--pink)">VS</div>
      <div class="panel gen" style="flex:1"><div class="display" style="text-align:center;color:#11625e;margin-bottom:6px">YOUR AI IMAGE ✨ 你的 AI 作品</div>
        <img src="${gen}" class="pop" style="width:100%;aspect-ratio:16/9;max-height:40vh;object-fit:cover;border-radius:14px"/></div>
    </div>
    <p class="display" style="font-size:clamp(13px,2.6vmin,24px);color:var(--lav)">"${app.round.transcript}"</p>
    <div style="display:flex;gap:18px">
      <button class="btn mint" id="save"><span class="en">💾 Save</span><span class="zh">保存</span></button>
      <button class="btn pink" id="next"><span class="en">Next Child ➡️</span><span class="zh">下一位</span></button>
    </div>`;
  root.appendChild(el);
  el.querySelector<HTMLButtonElement>('#save')!.onclick = async (e) => {
    const btn = e.currentTarget as HTMLButtonElement;
    try {
      const s = await api.getSettings();
      const png = await composeSnapshot(orig, gen, app.round.transcript, s.child_label);
      await api.saveSnapshot(png);
      btn.innerHTML = '<span class="en">✅ Saved</span><span class="zh">已保存</span>';
    } catch (err) { app.showError(String(err)); }
  };
  el.querySelector<HTMLButtonElement>('#next')!.onclick = () => { app.resetRound(); app.go('draw'); };
}
