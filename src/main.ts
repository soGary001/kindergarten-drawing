import { App, type Screen } from './state';
import { api } from './api';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { renderIdle } from './screens/idle';
import { renderDraw } from './screens/draw';
import { renderDescribe } from './screens/describe';
import { renderGenerating } from './screens/generating';
import { renderCompare } from './screens/compare';
import { mountSettings } from './screens/settings';
import { showError } from './screens/errorOverlay';

function scatterShapes() {
  const host = document.getElementById('shapes')!;
  const svgs = [
    `<svg width="80" height="80"><circle cx="40" cy="40" r="26" fill="#a0e7e5"/></svg>`,
    `<svg width="80" height="80"><polygon points="40,6 72,68 8,68" fill="#c3b6f7"/></svg>`,
    `<svg width="90" height="40"><path d="M5 30 Q20 5 35 30 T70 30" stroke="#ffd23f" stroke-width="8" fill="none"/></svg>`,
    `<svg width="60" height="60"><rect x="10" y="10" width="40" height="40" rx="8" fill="#ff7eb6" transform="rotate(18 30 30)"/></svg>`,
  ];
  const spots = [[5,8],[88,12],[3,70],[90,75],[48,4],[70,88]];
  spots.forEach(([x,y],i)=>{ const d=document.createElement('div'); d.className='shape floaty';
    d.style.left=x+'vw'; d.style.top=y+'vh'; d.style.animationDelay=(i*0.4)+'s';
    d.innerHTML=svgs[i%svgs.length]; host.appendChild(d); });
}

async function boot() {
  scatterShapes();
  const root = document.getElementById('app')!;
  const app = new App(root, {
    idle: renderIdle, draw: renderDraw, describe: renderDescribe,
    generating: renderGenerating, compare: renderCompare,
  } as Record<Screen, any>, (m)=>showError(m, ()=>app.go('idle')));

  // Global "back to home" button — quit any activity and return to the start screen.
  const back = document.createElement('button');
  back.id = 'backHome';
  back.innerHTML = '🏠 <span style="font-size:.8em">返回 / Home</span>';
  back.style.cssText = 'position:fixed;top:10px;left:12px;z-index:40;font-family:var(--font-display);font-weight:800;font-size:16px;padding:8px 16px;border:none;border-radius:999px;background:#fff;color:var(--ink);box-shadow:0 4px 0 var(--shadow-lav);cursor:pointer';
  back.onclick = () => app.goHome();
  document.body.appendChild(back);
  // Hide it on the home screen itself.
  app.setOnRender((screen) => { back.style.display = screen === 'idle' ? 'none' : 'block'; });

  mountSettings(app);
  const s = await api.getSettings();
  if (s.fullscreen) { await getCurrentWindow().setFullscreen(true); }
  app.go('idle');
  api.checkConnectivity().then(ok => { if (!ok) showError("No internet 无网络 — speech & image generation need internet / 语音识别和生成图片都需要联网 🌐", () => app.go('idle')); });
}
boot();
