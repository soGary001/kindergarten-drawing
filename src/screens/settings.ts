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
    p.innerHTML = `<div class="panel" style="width:min(520px,92vw);max-height:90vh;overflow-y:auto;display:flex;flex-direction:column;gap:14px">
      <h2 style="color:var(--pink)">Operator Settings<span class="zh-sub">操作设置</span></h2>
      <label class="display">Child / Round label 孩子 / 轮次 标签</label>
      <input id="label" style="font-size:20px;padding:8px;border-radius:10px;border:2px solid var(--lav)"/>
      <button class="btn mint" id="gallery"><span class="en">📁 Gallery folder</span><span class="zh">图片文件夹</span></button>
      <div id="gpath" style="font-size:14px;color:#888">${s.gallery_dir ?? '(默认内置 default)'}</div>
      <button class="btn mint" id="snap"><span class="en">📁 Snapshot folder</span><span class="zh">截图保存文件夹</span></button>
      <div id="spath" style="font-size:14px;color:#888">${s.snapshot_dir ?? '(默认 default)'}</div>
      <label class="display"><input type="checkbox" id="fs" ${s.fullscreen?'checked':''}/> Fullscreen 全屏</label>
      <div style="display:flex;gap:10px"><button class="btn pink" id="ok"><span class="en">Save</span><span class="zh">保存</span></button><button class="btn" id="cancel"><span class="en">Close</span><span class="zh">关闭</span></button></div>
    </div>`;
    document.body.appendChild(p);
    p.querySelector<HTMLInputElement>('#label')!.value = s.child_label;
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
