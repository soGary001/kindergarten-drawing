import type { App } from '../state';
import { api, fileUrl, onEvent } from '../api';
import { startEncourageMusic, stopEncourageMusic, playCelebration } from '../sound';

const CHEERS = [
  '画得真棒! / Great job!',
  '马上好啦! / Almost there!',
  '你的想法太有趣了! / What a fun idea!',
  '小画家加油! / Keep going, little artist!',
  '魔法正在发生… / Magic is happening…',
  '再等一下下~ / Just a moment more~',
];

export function renderDescribe(root: HTMLElement, app: App) {
  const el = document.createElement('div'); el.className = 'screen';
  const origUrl = app.round.picture ? fileUrl(app.round.picture.path) : '';
  el.innerHTML = `
    <div style="display:flex;gap:18px;align-items:center;width:92vw;justify-content:center">
      <div style="display:flex;flex-direction:column;align-items:center;gap:4px">
        <div class="display" style="font-size:clamp(11px,1.8vmin,15px);color:var(--lav)">原图 / Card</div>
        <div class="panel" style="width:min(26vw, calc(22vh * 16 / 9));aspect-ratio:16/9"><img src="${origUrl}" style="width:100%;height:100%;object-fit:cover;border-radius:12px"/></div>
      </div>
      <div class="display" style="font-size:clamp(20px,4vmin,36px);color:var(--pink)">➜</div>
      <div style="display:flex;flex-direction:column;align-items:center;gap:4px">
        <div class="display" style="font-size:clamp(11px,1.8vmin,15px);color:#11625e">你的 AI 作品 / Your AI picture</div>
        <div class="panel gen" id="aibox" style="width:min(48vw, calc(40vh * 16 / 9));aspect-ratio:16/9;display:flex;align-items:center;justify-content:center;font-size:clamp(30px,8vmin,64px)">🖼️</div>
      </div>
    </div>
    <div id="words" style="font-family:var(--font-display);font-weight:700;font-size:clamp(15px,2.6vmin,26px);color:var(--ink);max-width:84vw;max-height:16vh;overflow-y:auto;min-height:1.3em;text-align:center;line-height:1.3"></div>
    <div id="status" class="display" style="font-size:clamp(12px,2vmin,18px);color:var(--lav);min-height:20px;text-align:center"></div>
    <div id="timer" class="display" style="font-size:clamp(26px,5.5vmin,52px);color:var(--mint);min-height:1.1em"></div>
    <div style="display:flex;gap:14px;flex-wrap:wrap;justify-content:center">
      <button class="btn hidden" id="again"><span class="en">🔄 Say again</span><span class="zh">重讲</span></button>
      <button class="btn pink hidden" id="gen"><span class="en">✨ Generate</span><span class="zh">生成图片</span></button>
      <button class="btn pink hidden" id="modify"><span class="en">🔧 Modify</span><span class="zh">修改图片</span></button>
      <button class="btn mint hidden" id="done"><span class="en">✅ Done</span><span class="zh">完成</span></button>
    </div>
    <div id="overlay" class="hidden" style="position:fixed;inset:0;z-index:30;background:rgba(253,238,242,.97);display:flex;flex-direction:column;align-items:center;justify-content:center;gap:20px">
      <div class="spin" style="font-size:clamp(60px,14vmin,110px)">🎨</div>
      <h1 class="display" id="cheer" style="font-size:clamp(26px,5vmin,46px);color:var(--pink);text-align:center"></h1>
      <p class="display" style="font-size:clamp(15px,2.6vmin,24px);color:var(--lav)">AI is painting… 正在画画…</p>
    </div>`;
  root.appendChild(el);

  const aibox = el.querySelector<HTMLDivElement>('#aibox')!;
  const wordsEl = el.querySelector<HTMLDivElement>('#words')!;
  const statusEl = el.querySelector<HTMLDivElement>('#status')!;
  const timerEl = el.querySelector<HTMLDivElement>('#timer')!;
  const againBtn = el.querySelector<HTMLButtonElement>('#again')!;
  const genBtn = el.querySelector<HTMLButtonElement>('#gen')!;
  const modifyBtn = el.querySelector<HTMLButtonElement>('#modify')!;
  const doneBtn = el.querySelector<HTMLButtonElement>('#done')!;
  const overlay = el.querySelector<HTMLDivElement>('#overlay')!;
  const cheerEl = el.querySelector<HTMLHeadingElement>('#cheer')!;

  const DURATION = 60;
  let remaining = DURATION;
  let ticker: number | undefined;
  let cheerTimer: number | undefined;
  let busy = false;       // generating/editing in progress
  let finishing = false;  // time up → finish after this round
  let ended = false;
  let roundWords: string[] = [];
  let partial = '';
  let currentUrl = '';    // remote url of current image (base for next edit)
  let currentPath = '';   // local path of current image (for display + compare)
  const unlisten: Array<Promise<() => void>> = [];

  function renderWords() {
    const confirmed = roundWords.join(' ');
    wordsEl.innerHTML = '';
    if (!confirmed && !partial) { wordsEl.textContent = '… 👂'; return; }
    const c = document.createElement('span'); c.textContent = confirmed + (partial ? ' ' : '');
    const p = document.createElement('span'); p.style.opacity = '0.45'; p.textContent = partial;
    wordsEl.appendChild(c); wordsEl.appendChild(p);
    wordsEl.scrollTop = wordsEl.scrollHeight;
  }
  function renderTimer() {
    timerEl.textContent = `⏱ ${remaining}`;
    timerEl.style.color = remaining <= 10 ? 'var(--pink)' : 'var(--mint)';
  }
  function startTimer() {
    if (ticker !== undefined) return;
    renderTimer();
    ticker = window.setInterval(() => {
      remaining -= 1; renderTimer();
      if (remaining <= 0) { remaining = 0; renderTimer(); onTimeUp(); }
    }, 1000);
  }
  function pauseTimer() { if (ticker !== undefined) { clearInterval(ticker); ticker = undefined; } }

  unlisten.push(onEvent<string>('asr://partial', (s) => { partial = s; renderWords(); }));
  unlisten.push(onEvent<string>('asr://final', (s) => { if (s.trim()) roundWords.push(s.trim()); partial = ''; renderWords(); }));
  unlisten.push(onEvent<string>('asr://error', (e) => { statusEl.textContent = '⚠️ ' + e; }));
  function cleanup() { unlisten.forEach((u) => u.then((f) => f())); }

  function setButtons(mode: 'first' | 'refine' | 'none') {
    againBtn.classList.toggle('hidden', mode === 'none');
    genBtn.classList.toggle('hidden', mode !== 'first');
    modifyBtn.classList.toggle('hidden', mode !== 'refine');
    doneBtn.classList.toggle('hidden', mode !== 'refine');
  }

  // Begin (or resume) listening for a new round of description.
  async function listen() {
    roundWords = []; partial = ''; renderWords();
    statusEl.textContent = '🎤 Starting… 正在开启麦克风…';
    setButtons('none');
    try { await api.asrStart(); }
    catch (e) { app.showError(String(e)); return; }
    setButtons(currentUrl ? 'refine' : 'first');
    statusEl.innerHTML = currentUrl
      ? '🔴 想改什么就继续说,然后点「修改图片」 / Say what to change, then tap Modify'
      : '🔴 看图说话吧!说完点「生成图片」 / Describe it, then tap Generate';
    startTimer();
  }

  function showOverlay(on: boolean) {
    overlay.classList.toggle('hidden', !on);
    if (on) {
      let i = 0; cheerEl.textContent = CHEERS[0];
      cheerTimer = window.setInterval(() => { i++; cheerEl.textContent = CHEERS[i % CHEERS.length]; }, 2400);
      startEncourageMusic();
    } else {
      if (cheerTimer !== undefined) { clearInterval(cheerTimer); cheerTimer = undefined; }
      stopEncourageMusic();
    }
  }

  // Stop recording, pause clock, run gen/edit with music + cheers, then continue or finish.
  async function runImage(kind: 'gen' | 'edit') {
    if (busy) return;
    busy = true;
    pauseTimer();
    setButtons('none');
    try { await api.asrStop(); } catch { /* ignore */ }
    await new Promise((r) => setTimeout(r, 900)); // catch trailing final sentence(s)
    const text = roundWords.join(' ').trim();

    if (kind === 'edit' && !text) {
      // nothing new said — just keep current image, resume listening
      busy = false;
      if (finishing) { finalize(); return; }
      statusEl.textContent = '(没听到补充,继续说说看吧 / say what to change)';
      listen();
      return;
    }
    if (kind === 'gen' && !text) {
      busy = false;
      statusEl.textContent = "(没听清,再说一次吧 / let's try again)";
      if (finishing) { finishing = false; } // give them another chance
      listen();
      return;
    }

    if (kind === 'gen') app.round.transcript = text; // the original description

    showOverlay(true);
    try {
      const res = kind === 'gen'
        ? await api.generateImage(text)
        : await api.editImage(currentUrl, text);
      currentUrl = res.url; currentPath = res.path;
      app.round.generatedPath = res.path;
      aibox.style.fontSize = '0';
      aibox.innerHTML = `<img src="${fileUrl(res.path)}" class="pop" style="width:100%;height:100%;object-fit:cover;border-radius:12px"/>`;
      showOverlay(false);
      playCelebration();
      busy = false;
      if (finishing || remaining <= 0) { finalize(); return; }
      statusEl.textContent = '✨ 看!可以继续补充修改,或点「完成」 / Like it? Add more, or tap Done';
      listen();
    } catch (e) {
      showOverlay(false);
      busy = false;
      app.showError(String(e));
    }
  }

  function onTimeUp() {
    pauseTimer();
    if (busy) { finishing = true; return; } // finish right after current gen
    finishing = true;
    if (!currentUrl) runImage('gen');                       // round 1 ran out → final generate
    else if (roundWords.join('').trim()) runImage('edit');  // pending edit → apply then finish
    else finalize();                                        // nothing new → finish with current image
  }

  async function finalize() {
    if (ended) return; ended = true;
    pauseTimer();
    showOverlay(false);
    try { await api.asrStop(); } catch { /* ignore */ }
    cleanup();
    if (!app.round.generatedPath && currentPath) app.round.generatedPath = currentPath;
    app.go('compare');
  }

  againBtn.onclick = () => { roundWords = []; partial = ''; renderWords(); statusEl.innerHTML = '🔄 重新说吧(时间继续) / say it again'; };
  genBtn.onclick = () => runImage('gen');
  modifyBtn.onclick = () => runImage('edit');
  doneBtn.onclick = () => finalize();

  renderWords();
  listen();
}
