export type Screen = 'idle'|'draw'|'describe'|'generating'|'compare';
export interface Round { picture?: { id:string; path:string }; transcript:string; generatedPath?:string; }

type Render = (root: HTMLElement, app: App) => void;

export class App {
  private screen: Screen = 'idle';
  round: Round = { transcript: '' };
  constructor(private root: HTMLElement, private screens: Record<Screen, Render>, private onError:(m:string)=>void) {}
  go(s: Screen) { this.screen = s; this.render(); }
  resetRound() { this.round = { transcript: '' }; }
  showError(m: string) { this.onError(m); }
  render() {
    this.root.innerHTML = '';
    this.screens[this.screen](this.root, this);
  }
}
