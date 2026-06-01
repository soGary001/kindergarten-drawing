import type { App } from '../state';
import { api } from '../api';
import { open } from '@tauri-apps/plugin-dialog';
import { getCurrentWindow } from '@tauri-apps/api/window';

export function mountSettings(_app: App) {
  const gear = document.createElement('button');
  gear.textContent = '⚙️';
  gear.style.cssText='position:fixed;top:10px;right:12px;z-index:40;background:none;border:none;font-size:26px;opacity:.5;cursor:pointer';
  document.body.appendChild(gear);
  const openPanel = async () => {
    const s = await api.getSettings();
    const p = document.createElement('div'); p.id='settings';
    p.style.cssText='position:fixed;inset:0;z-index:45;background:rgba(0,0,0,.4);display:flex;align-items:center;justify-content:center';
    p.innerHTML = `<div class="panel" style="width:520px;display:flex;flex-direction:column;gap:14px">
      <h2 style="color:var(--pink)">Operator Settings</h2>
      <label class="display">Child / Round label</label>
      <input id="label" value="${s.child_label}" style="font-size:20px;padding:8px;border-radius:10px;border:2px solid var(--lav)"/>
      <button class="btn mint" id="gallery">📁 Gallery folder</button>
      <div id="gpath" style="font-size:14px;color:#888">${s.gallery_dir ?? '(bundled default)'}</div>
      <button class="btn mint" id="snap">📁 Snapshot folder</button>
      <div id="spath" style="font-size:14px;color:#888">${s.snapshot_dir ?? '(default)'}</div>
      <label class="display"><input type="checkbox" id="fs" ${s.fullscreen?'checked':''}/> Fullscreen</label>
      <div style="display:flex;gap:10px"><button class="btn pink" id="ok">Save</button><button class="btn" id="cancel">Close</button></div>
    </div>`;
    document.body.appendChild(p);
    const next = { ...s };
    p.querySelector<HTMLButtonElement>('#gallery')!.onclick = async () => { const d = await open({ directory:true }); if (d) { next.gallery_dir = d as string; p.querySelector('#gpath')!.textContent = d as string; } };
    p.querySelector<HTMLButtonElement>('#snap')!.onclick = async () => { const d = await open({ directory:true }); if (d) { next.snapshot_dir = d as string; p.querySelector('#spath')!.textContent = d as string; } };
    p.querySelector<HTMLButtonElement>('#cancel')!.onclick = () => p.remove();
    p.querySelector<HTMLButtonElement>('#ok')!.onclick = async () => {
      next.child_label = p.querySelector<HTMLInputElement>('#label')!.value;
      next.fullscreen = p.querySelector<HTMLInputElement>('#fs')!.checked;
      await api.setSettings(next);
      await getCurrentWindow().setFullscreen(next.fullscreen);
      p.remove();
    };
  };
  gear.onclick = openPanel;
  window.addEventListener('keydown', (e)=>{ if (e.key.toLowerCase()==='s' && e.ctrlKey) openPanel(); });
}
