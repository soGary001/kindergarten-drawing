export function showError(msg: string, onRetry: () => void) {
  document.querySelector('#err')?.remove();
  const o = document.createElement('div'); o.id='err';
  o.style.cssText='position:fixed;inset:0;z-index:50;background:rgba(253,238,242,.96);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:20px';
  o.innerHTML = `<div style="font-size:clamp(44px,10vmin,80px)">🐣</div>
    <h1 class="display" style="color:var(--pink);font-size:clamp(22px,5vmin,40px)">Oops, let's try again!<span class="zh-sub">哎呀，再试一次吧！</span></h1>
    <p class="display" style="font-size:clamp(14px,2.6vmin,22px);color:var(--lav);max-width:60vw;text-align:center">${msg}</p>
    <button class="btn pink" id="retry"><span class="en">Try Again 🔄</span><span class="zh">再试一次</span></button>`;
  document.body.appendChild(o);
  o.querySelector<HTMLButtonElement>('#retry')!.onclick = () => { o.remove(); onRetry(); };
}
