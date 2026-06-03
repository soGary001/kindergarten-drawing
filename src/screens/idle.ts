import type { App } from '../state';
export function renderIdle(root: HTMLElement, app: App) {
  const el = document.createElement('div');
  el.className = 'screen';
  el.innerHTML = `
    <div class="display pop" style="font-size:clamp(44px,10vmin,88px)">🎨🗣️✨</div>
    <h1 class="pop" style="font-size:clamp(26px,6.5vmin,56px);color:var(--pink);text-align:center">English Speaking Stars<span class="zh-sub">英语口语小明星</span></h1>
    <p class="display" style="font-size:clamp(14px,2.8vmin,24px);color:var(--lav)">Draw a picture · Describe it · Watch the magic!<span class="zh-line">抽张图卡 · 用英语描述 · 见证魔法时刻！</span></p>
    <button class="btn pink floaty" id="start" style="margin-top:20px"><span class="en">Start! 🚀</span><span class="zh">开始</span></button>`;
  root.appendChild(el);
  el.querySelector<HTMLButtonElement>('#start')!.onclick = () => { app.resetRound(); app.go('draw'); };
}
