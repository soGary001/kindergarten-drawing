import type { App } from '../state';
import { api, fileUrl, onEvent } from '../api';
import { MicStreamer } from '../audio';

export function renderDescribe(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  const picUrl = app.round.picture ? fileUrl(app.round.picture.path) : '';
  el.innerHTML = `
    <div class="panel" style="width:58vw;aspect-ratio:16/9"><img src="${picUrl}" style="width:100%;height:100%;object-fit:cover;border-radius:16px"/></div>
    <div id="transcript" class="display" style="font-size:30px;min-height:44px;color:var(--ink);text-align:center;max-width:70vw">🎤 Tap the mic and tell me what you see!</div>
    <div style="display:flex;gap:18px">
      <button class="btn pink" id="mic">🎤 Start Talking</button>
      <button class="btn mint hidden" id="gen">Make it! ✨</button>
    </div>`;
  root.appendChild(el);
  const tEl = el.querySelector<HTMLDivElement>('#transcript')!;
  const mic = el.querySelector<HTMLButtonElement>('#mic')!;
  const gen = el.querySelector<HTMLButtonElement>('#gen')!;

  let streamer: MicStreamer | null = null;
  let recording = false;
  let finalText = '';
  const unlisten: Array<Promise<any>> = [];

  unlisten.push(onEvent<string>('asr://partial', (s) => { tEl.textContent = finalText + ' ' + s; }));
  unlisten.push(onEvent<string>('asr://final', (s) => { finalText = (finalText + ' ' + s).trim(); tEl.textContent = finalText; app.round.transcript = finalText; }));
  unlisten.push(onEvent<string>('asr://error', (e) => app.showError(e)));

  mic.onclick = async () => {
    if (!recording) {
      try {
        finalText=''; app.round.transcript='';
        await api.asrStart();
        streamer = new MicStreamer((bytes)=>api.asrSendAudio(bytes));
        await streamer.start();
        recording = true; mic.textContent = '⏹ Stop'; mic.classList.add('mint');
      } catch (e) { app.showError(String(e)); }
    } else {
      await streamer?.stop(); await api.asrStop();
      recording = false; mic.textContent = '🎤 Talk again';
      gen.classList.remove('hidden');
    }
  };
  gen.onclick = () => { unlisten.forEach(u=>u.then(f=>f())); app.go('generating'); };
}
