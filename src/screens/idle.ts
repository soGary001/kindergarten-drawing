import type { App } from '../state';
export function renderIdle(root: HTMLElement, app: App) {
  const el = document.createElement('div');
  el.className = 'screen';
  el.innerHTML = `
    <div class="display pop" style="font-size:88px">🎨🗣️✨</div>
    <h1 class="pop" style="font-size:56px;color:var(--pink);text-align:center">English Speaking Stars</h1>
    <p class="display" style="font-size:24px;color:var(--lav)">Draw a picture · Describe it · Watch the magic!</p>
    <button class="btn pink floaty" id="start" style="margin-top:20px">Start! 🚀</button>`;
  root.appendChild(el);
  el.querySelector<HTMLButtonElement>('#start')!.onclick = () => { app.resetRound(); app.go('draw'); };
}
