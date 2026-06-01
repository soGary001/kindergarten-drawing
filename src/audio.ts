// Captures mic, resamples to 16kHz mono PCM16, calls onChunk with byte arrays (~100ms).
export class MicStreamer {
  private ctx?: AudioContext; private stream?: MediaStream; private node?: ScriptProcessorNode; private src?: MediaStreamAudioSourceNode;
  constructor(private onChunk: (bytes: number[]) => void) {}
  async start() {
    this.stream = await navigator.mediaDevices.getUserMedia({ audio: { channelCount: 1, echoCancellation: true, noiseSuppression: true } });
    this.ctx = new AudioContext();
    this.src = this.ctx.createMediaStreamSource(this.stream);
    this.node = this.ctx.createScriptProcessor(4096, 1, 1);
    const inRate = this.ctx.sampleRate;
    this.src.connect(this.node); this.node.connect(this.ctx.destination);
    this.node.onaudioprocess = (e) => {
      const input = e.inputBuffer.getChannelData(0);
      const down = downsample(input, inRate, 16000);
      const pcm = floatToPCM16(down);
      this.onChunk(Array.from(new Uint8Array(pcm.buffer)));
    };
  }
  async stop() {
    this.node?.disconnect(); this.src?.disconnect();
    this.stream?.getTracks().forEach(t => t.stop());
    await this.ctx?.close();
  }
}
function downsample(buf: Float32Array, inRate: number, outRate: number): Float32Array {
  if (outRate >= inRate) return buf;
  const ratio = inRate / outRate; const len = Math.floor(buf.length / ratio); const out = new Float32Array(len);
  for (let i=0;i<len;i++) out[i] = buf[Math.floor(i*ratio)];
  return out;
}
function floatToPCM16(buf: Float32Array): Int16Array {
  const out = new Int16Array(buf.length);
  for (let i=0;i<buf.length;i++){ const s=Math.max(-1,Math.min(1,buf[i])); out[i]= s<0 ? s*0x8000 : s*0x7fff; }
  return out;
}
