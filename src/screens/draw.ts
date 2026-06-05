import type { App } from '../state';
import { api, fileUrl } from '../api';
import { playCelebration } from '../sound';

export function renderDraw(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  el.innerHTML = `
    <h1 style="color:var(--pink);font-size:clamp(22px,5.2vmin,44px)">Lucky Draw 🎴<span class="zh-sub">幸运抽卡</span></h1>
    <div id="card" class="panel" style="width:min(62vw, calc(54vh * 16 / 9));aspect-ratio:16/9;display:flex;align-items:center;justify-content:center;font-size:clamp(56px,14vmin,120px);overflow:hidden">🎴</div>
    <button class="btn" id="draw"><span class="en">Draw a Card! 🎴</span><span class="zh">抽一张卡</span></button>`;
  root.appendChild(el);
  const card = el.querySelector<HTMLDivElement>('#card')!;
  const drawBtn = el.querySelector<HTMLButtonElement>('#draw')!;

  let spin: number | undefined;
  let advance: number | undefined;
  app.setLeaveHook(() => {
    if (spin !== undefined) clearInterval(spin);
    if (advance !== undefined) clearTimeout(advance);
  });

  drawBtn.onclick = async () => {
    drawBtn.disabled = true;
    // Gather gallery images to flash as card faces during the shuffle.
    let faces: string[] = [];
    try { faces = (await api.listGallery()).map((g) => fileUrl(g.path)); } catch { /* ignore */ }
    // Draw the actual pick.
    let pic;
    try {
      pic = await api.drawRandom();
    } catch (e) {
      drawBtn.disabled = false;
      card.style.fontSize = '22px';
      card.innerHTML = `<div style="text-align:center;padding:20px;color:var(--lav)" class="display">🖼️ No pictures yet!<span style="font-size:16px;display:block;margin-top:6px">Ask a grown-up: ⚙️ Settings → Gallery folder<br>还没有图片,请老师在 ⚙️ 设置 → 图片文件夹 中添加</span></div>`;
      return;
    }
    const finalUrl = fileUrl(pic.path);
    card.style.fontSize = '0';
    const pool = faces.length ? faces : [finalUrl];
    let i = 0;
    const start = Date.now();
    spin = window.setInterval(() => {
      card.innerHTML = `<img src="${pool[i % pool.length]}" style="width:100%;height:100%;object-fit:cover;border-radius:16px"/>`;
      i++;
      if (Date.now() - start > 1200) {
        if (spin !== undefined) clearInterval(spin);
        card.innerHTML = `<img src="${finalUrl}" class="pop" style="width:100%;height:100%;object-fit:cover;border-radius:16px"/>`;
        app.round.picture = pic;
        playCelebration();
        advance = window.setTimeout(() => app.go('describe'), 1300); // auto-advance — no confirm tap
      }
    }, 90);
  };
}
