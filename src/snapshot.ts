// Compose original + generated side by side with the transcript caption into one PNG.
export async function composeSnapshot(origUrl: string, genUrl: string, caption: string, label: string): Promise<string> {
  const [a, b] = await Promise.all([loadImg(origUrl), loadImg(genUrl)]);
  const W = 1600, H = 760, pad = 30, gap = 30;
  const cw = (W - pad*2 - gap) / 2, ch = cw * 9/16;
  const c = document.createElement('canvas'); c.width = W; c.height = H;
  const ctx = c.getContext('2d')!;
  ctx.fillStyle = '#fdeef2'; ctx.fillRect(0,0,W,H);
  drawCover(ctx, a, pad, 80, cw, ch);
  drawCover(ctx, b, pad+cw+gap, 80, cw, ch);
  ctx.fillStyle = '#ff7eb6'; ctx.font = '800 40px "Baloo 2", sans-serif'; ctx.textAlign='center';
  ctx.fillText('English Speaking Stars ⭐', W/2, 56);
  ctx.fillStyle = '#2b2b3a'; ctx.font = '500 30px "Fredoka", sans-serif';
  ctx.fillText(`"${caption}"`, W/2, 80+ch+50);
  if (label) { ctx.fillStyle='#c3b6f7'; ctx.font='600 26px "Fredoka",sans-serif'; ctx.fillText(label, W/2, 80+ch+90); }
  return c.toDataURL('image/png');
}
function loadImg(src: string){ return new Promise<HTMLImageElement>((res,rej)=>{const i=new Image(); i.crossOrigin='anonymous'; i.onload=()=>res(i); i.onerror=rej; i.src=src;}); }
function drawCover(ctx:CanvasRenderingContext2D,img:HTMLImageElement,x:number,y:number,w:number,h:number){
  const r=Math.max(w/img.width,h/img.height); const iw=img.width*r, ih=img.height*r;
  ctx.save(); ctx.beginPath(); roundRect(ctx,x,y,w,h,16); ctx.clip();
  ctx.drawImage(img, x+(w-iw)/2, y+(h-ih)/2, iw, ih); ctx.restore();
}
function roundRect(ctx:CanvasRenderingContext2D,x:number,y:number,w:number,h:number,r:number){
  ctx.moveTo(x+r,y); ctx.arcTo(x+w,y,x+w,y+h,r); ctx.arcTo(x+w,y+h,x,y+h,r); ctx.arcTo(x,y+h,x,y,r); ctx.arcTo(x,y,x+w,y,r); ctx.closePath();
}
