import type { App } from '../state';
import { api, fileUrl } from '../api';

export function renderDraw(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  el.innerHTML = `
    <h1 style="color:var(--pink);font-size:44px">Lucky Draw 🎴<span class="zh-sub">幸运抽卡</span></h1>
    <div id="card" class="panel" style="width:62vw;aspect-ratio:16/9;display:flex;align-items:center;justify-content:center;font-size:120px">🎴</div>
    <div style="display:flex;gap:18px">
      <button class="btn" id="draw"><span class="en">Draw a Card! 🎴</span><span class="zh">抽一张卡</span></button>
      <button class="btn mint hidden" id="use"><span class="en">Use this one ✅</span><span class="zh">就选它</span></button>
      <button class="btn pink hidden" id="again"><span class="en">Draw again 🔄</span><span class="zh">再抽一次</span></button>
    </div>`;
  root.appendChild(el);
  const card = el.querySelector<HTMLDivElement>('#card')!;
  const drawBtn = el.querySelector<HTMLButtonElement>('#draw')!;
  const useBtn = el.querySelector<HTMLButtonElement>('#use')!;
  const againBtn = el.querySelector<HTMLButtonElement>('#again')!;

  async function doDraw() {
    drawBtn.disabled = true; againBtn.disabled = true;
    card.classList.add('spin');
    try {
      const pic = await api.drawRandom();
      setTimeout(() => {
        card.classList.remove('spin');
        card.style.fontSize = '0';
        card.innerHTML = `<img src="${fileUrl(pic.path)}" style="width:100%;height:100%;object-fit:cover;border-radius:16px" class="pop"/>`;
        app.round.picture = pic;
        drawBtn.classList.add('hidden'); useBtn.classList.remove('hidden'); againBtn.classList.remove('hidden');
        drawBtn.disabled = false; againBtn.disabled = false;
      }, 700);
    } catch (e) {
      card.classList.remove('spin');
      drawBtn.disabled = false; againBtn.disabled = false;
      if (String(e).toLowerCase().includes('empty')) {
        card.style.fontSize = '22px';
        card.innerHTML = `<div style="text-align:center;padding:20px;color:var(--lav)" class="display">🖼️ No pictures yet!<span style="font-size:16px;display:block;margin-top:6px">Ask a grown-up: ⚙️ Settings → Gallery folder<br>还没有图片，请老师在 ⚙️ 设置 → 图片文件夹 中添加</span></div>`;
      } else {
        app.showError(String(e));
      }
    }
  }
  drawBtn.onclick = doDraw;
  againBtn.onclick = doDraw;
  useBtn.onclick = () => app.go('describe');
}
