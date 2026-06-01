export function showError(msg: string, onRetry: () => void) {
  document.querySelector('#err')?.remove();
  const o = document.createElement('div'); o.id='err';
  o.style.cssText='position:fixed;inset:0;z-index:50;background:rgba(253,238,242,.96);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:20px';
  o.innerHTML = `<div style="font-size:80px">🐣</div>
    <h1 class="display" style="color:var(--pink);font-size:40px">Oops, let's try again!</h1>
    <p class="display" style="font-size:22px;color:var(--lav);max-width:60vw;text-align:center">${msg}</p>
    <button class="btn pink" id="retry">Try Again 🔄</button>`;
  document.body.appendChild(o);
  o.querySelector<HTMLButtonElement>('#retry')!.onclick = () => { o.remove(); onRetry(); };
}
