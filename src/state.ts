export type Screen = 'idle'|'draw'|'describe'|'generating'|'compare';
export interface Round { picture?: { id:string; path:string }; transcript:string; generatedPath?:string; }

type Render = (root: HTMLElement, app: App) => void;

export class App {
  private screen: Screen = 'idle';
  round: Round = { transcript: '' };
  private leaveHook?: () => void;
  private onRender?: (s: Screen) => void;
  constructor(private root: HTMLElement, private screens: Record<Screen, Render>, private onError:(m:string)=>void) {}

  /** A screen can register cleanup that runs when navigating away (stop mic, timers, music…). */
  setLeaveHook(fn?: () => void) { this.leaveHook = fn; }
  setOnRender(fn: (s: Screen) => void) { this.onRender = fn; }
  current(): Screen { return this.screen; }

  go(s: Screen) {
    if (this.leaveHook) { try { this.leaveHook(); } catch { /* ignore */ } this.leaveHook = undefined; }
    this.screen = s;
    this.render();
  }
  /** Quit the current activity and return to the home screen. */
  goHome() { this.resetRound(); this.go('idle'); }
  resetRound() { this.round = { transcript: '' }; }
  showError(m: string) { this.onError(m); }
  render() {
    this.root.innerHTML = '';
    this.screens[this.screen](this.root, this);
    if (this.onRender) this.onRender(this.screen);
  }
}
