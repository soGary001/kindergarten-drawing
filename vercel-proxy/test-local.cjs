// Local smoke test for the proxy handlers — hits DashScope directly (no Vercel needed).
// Run: DASHSCOPE_API_KEY=sk-... node vercel-proxy/test-local.cjs /tmp/sp.wav
const fs = require("fs");
const genImage = require("./api/generate-image.js");
const transcribe = require("./api/transcribe.js");

function mockRes() {
  return {
    _status: 0,
    status(c) { this._status = c; return this; },
    json(o) { console.log("  ->", this._status, JSON.stringify(o).slice(0, 300)); return this; },
  };
}

(async () => {
  const wav = process.argv[2] || "/tmp/sp.wav";
  console.log("== transcribe ==");
  const audio = fs.readFileSync(wav).toString("base64");
  await transcribe({ method: "POST", body: { audio } }, mockRes());

  console.log("== generate-image ==");
  await genImage({ method: "POST", body: { prompt: "a happy yellow duck, cute children book illustration" } }, mockRes());
})();
