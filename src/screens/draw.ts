import type { App } from '../state';
import { api, fileUrl } from '../api';

export function renderDraw(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  el.innerHTML = `
    <h1 style="color:var(--pink);font-size:44px">Lucky Draw 🎴</h1>
    <div id="card" class="panel" style="width:62vw;aspect-ratio:16/9;display:flex;align-items:center;justify-content:center;font-size:120px">🎴</div>
    <div style="display:flex;gap:18px">
      <button class="btn" id="draw">Draw a Card!</button>
      <button class="btn mint hidden" id="use">Use this one ✅</button>
      <button class="btn pink hidden" id="again">Draw again 🔄</button>
    </div>`;
  root.appendChild(el);
  const card = el.querySelector<HTMLDivElement>('#card')!;
  const drawBtn = el.querySelector<HTMLButtonElement>('#draw')!;
  const useBtn = el.querySelector<HTMLButtonElement>('#use')!;
  const againBtn = el.querySelector<HTMLButtonElement>('#again')!;

  async function doDraw() {
    card.classList.add('spin');
    try {
      const pic = await api.drawRandom();
      setTimeout(() => {
        card.classList.remove('spin');
        card.style.fontSize = '0';
        card.innerHTML = `<img src="${fileUrl(pic.path)}" style="width:100%;height:100%;object-fit:cover;border-radius:16px" class="pop"/>`;
        app.round.picture = pic;
        drawBtn.classList.add('hidden'); useBtn.classList.remove('hidden'); againBtn.classList.remove('hidden');
      }, 700);
    } catch (e) { card.classList.remove('spin'); app.showError(String(e)); }
  }
  drawBtn.onclick = doDraw;
  againBtn.onclick = doDraw;
  useBtn.onclick = () => app.go('describe');
}
