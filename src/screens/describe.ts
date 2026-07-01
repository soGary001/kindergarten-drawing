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

// Fix common homophones the recognizer mishears in this picture-describing context.
// (Kids say "sun", ASR often returns "son".) Whole-word, case-preserving.
const HOMOPHONES: Record<string, string> = {
  son: 'sun',
  sons: 'suns',
};
function fixHomophones(text: string): string {
  return text.replace(/[A-Za-z]+/g, (w) => {
    const r = HOMOPHONES[w.toLowerCase()];
    if (!r) return w;
    return w[0] === w[0].toUpperCase() ? r[0].toUpperCase() + r.slice(1) : r;
  });
}

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
    <div id="status" class="display" style="font-size:clamp(12px,2vmin,18px);color:var(--lav);min-height:20px;text-align:center;max-width:84vw"></div>
    <div id="timer" class="display" style="font-size:clamp(26px,5.5vmin,52px);color:var(--mint);min-height:1.1em"></div>
    <div style="display:flex;gap:14px;flex-wrap:wrap;justify-content:center">
      <button class="btn hidden" id="again"><span class="en">🔄 Say again</span><span class="zh">重讲</span></button>
      <button class="btn pink hidden" id="act"></button>
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
  const actBtn = el.querySelector<HTMLButtonElement>('#act')!;
  const modifyBtn = el.querySelector<HTMLButtonElement>('#modify')!;
  const doneBtn = el.querySelector<HTMLButtonElement>('#done')!;
  const overlay = el.querySelector<HTMLDivElement>('#overlay')!;
  const cheerEl = el.querySelector<HTMLHeadingElement>('#cheer')!;

  const DURATION = 30;
  let remaining = DURATION;
  let ticker: number | undefined;
  let cheerTimer: number | undefined;
  let busy = false;
  let finishing = false;
  let ended = false;
  let recording = false;
  let roundWords: string[] = [];
  let partial = '';
  let currentUrl = '';
  let currentPath = '';
  const unlisten: Array<Promise<() => void>> = [];

  function renderWords() {
    const confirmed = roundWords.join(' ');
    wordsEl.innerHTML = '';
    if (!confirmed && !partial) { wordsEl.textContent = recording ? '… 👂' : ''; return; }
    const c = document.createElement('span'); c.textContent = confirmed + (partial ? ' ' : '');
    const p = document.createElement('span'); p.style.opacity = '0.45'; p.textContent = partial;
    wordsEl.appendChild(c); wordsEl.appendChild(p);
    wordsEl.scrollTop = wordsEl.scrollHeight;
  }
  function renderTimer() {
    timerEl.textContent = `⏱ ${remaining}`;
    timerEl.style.color = remaining <= 10 ? 'var(--pink)' : 'var(--mint)';
  }
  // Timer ONLY runs while actively talking; paused otherwise.
  function startTimer() {
    if (ticker !== undefined) return;
    renderTimer();
    ticker = window.setInterval(() => {
      remaining -= 1; renderTimer();
      if (remaining <= 0) { remaining = 0; renderTimer(); onTimeUp(); }
    }, 1000);
  }
  function pauseTimer() { if (ticker !== undefined) { clearInterval(ticker); ticker = undefined; } }

  unlisten.push(onEvent<string>('asr://partial', (s) => { if (recording) { partial = fixHomophones(s); renderWords(); } }));
  unlisten.push(onEvent<string>('asr://final', (s) => { if (recording && s.trim()) { roundWords.push(fixHomophones(s.trim())); partial = ''; renderWords(); } }));
  unlisten.push(onEvent<string>('asr://error', (e) => { statusEl.textContent = '⚠️ ' + e; }));
  function cleanup() { unlisten.forEach((u) => u.then((f) => f())); }

  type Phase = 'talk' | 'idle' | 'none';
  function setButtons(p: Phase) {
    againBtn.classList.toggle('hidden', p !== 'talk');
    actBtn.classList.toggle('hidden', p !== 'talk');
    modifyBtn.classList.toggle('hidden', p !== 'idle');
    doneBtn.classList.toggle('hidden', p !== 'idle');
  }

  // Start a talking round (round 1 = generate; later rounds = edit). Timer runs now.
  async function startTalk() {
    roundWords = []; partial = '';
    setButtons('none');
    statusEl.textContent = '🎤 Starting… 正在开启麦克风…';
    try { await api.asrStart(); }
    catch (e) { app.showError(String(e)); return; }
    recording = true;
    renderWords();
    actBtn.innerHTML = currentUrl
      ? '<span class="en">✅ Done speaking</span><span class="zh">说完了</span>'
      : '<span class="en">✨ Generate</span><span class="zh">生成图片</span>';
    setButtons('talk');
    statusEl.innerHTML = currentUrl
      ? '🔴 说出要怎么改,说完点「说完了」 / Say what to change'
      : '🔴 看图说话吧,说完点「生成图片」 / Describe it';
    startTimer();
  }

  // Image is shown; waiting for the child to decide. Timer PAUSED.
  function enterIdle() {
    recording = false;
    pauseTimer();
    renderWords();
    setButtons('idle');
    statusEl.innerHTML = '想继续改就点「修改图片」开始说,满意就点「完成」 / Tap Modify to keep changing, or Done';
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

  // Stop recording, pause clock, run generate/edit (music + cheers), then idle or finish.
  async function runImage() {
    if (busy) return;
    busy = true;
    pauseTimer();
    recording = false;
    setButtons('none');
    try { await api.asrStop(); } catch { /* ignore */ }
    await new Promise((r) => setTimeout(r, 900)); // catch trailing final sentence(s)
    const text = roundWords.join(' ').trim();
    const isEdit = !!currentUrl;

    if (!text) {
      busy = false;
      if (isEdit) { enterIdle(); statusEl.textContent = '(没听到补充 / nothing new said)'; return; }
      // round 1 with no speech
      statusEl.textContent = "(没听清,再说一次吧 / let's try again)";
      finishing = false;
      startTalk();
      return;
    }
    if (!isEdit) app.round.transcript = text;

    showOverlay(true);
    try {
      const res = isEdit ? await api.editImage(currentUrl, text) : await api.generateImage(text);
      currentUrl = res.url; currentPath = res.path;
      app.round.generatedPath = res.path;
      aibox.style.fontSize = '0';
      aibox.innerHTML = `<img src="${fileUrl(res.path)}" class="pop" style="width:100%;height:100%;object-fit:cover;border-radius:12px"/>`;
      showOverlay(false);
      playCelebration();
      busy = false;
      if (finishing) { finalize(); return; }
      enterIdle();
    } catch (e) {
      showOverlay(false);
      busy = false;
      app.showError(String(e));
    }
  }

  function onTimeUp() {
    // Only fires while talking. Do one final generate/edit, then finish.
    pauseTimer();
    finishing = true;
    if (busy) return;
    runImage();
  }

  let tornDown = false;
  function teardown() {
    if (tornDown) return; tornDown = true;
    pauseTimer();
    if (cheerTimer !== undefined) { clearInterval(cheerTimer); cheerTimer = undefined; }
    stopEncourageMusic();
    recording = false;
    api.asrStop().catch(() => {});
    cleanup();
  }
  // Runs on ANY navigation away (incl. the global Home button): stop mic/timer/music.
  app.setLeaveHook(teardown);

  function finalize() {
    if (ended) return; ended = true;
    if (!app.round.generatedPath && currentPath) app.round.generatedPath = currentPath;
    app.go('compare'); // leaveHook → teardown cleans up
  }

  againBtn.onclick = () => { roundWords = []; partial = ''; renderWords(); statusEl.innerHTML = '🔄 重新说吧(时间继续) / say it again'; };
  actBtn.onclick = () => runImage();
  modifyBtn.onclick = () => startTalk();
  doneBtn.onclick = () => finalize();

  renderWords();
  startTalk(); // round 1 auto-starts (right after drawing the card)
}
