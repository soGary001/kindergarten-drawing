// Vercel serverless function: proxy DashScope qwen3-asr-flash speech-to-text (non-realtime).
// The app records the child's full utterance, then POSTs it here as base64 WAV.
// POST { audio: <base64 wav, or data URI> }  ->  200 { text }  |  4xx/5xx { error }

const EP = "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation";

module.exports = async function handler(req, res) {
  if (req.method !== "POST") return res.status(405).json({ error: "POST only" });
  const key = process.env.DASHSCOPE_API_KEY;
  if (!key) return res.status(500).json({ error: "server missing DASHSCOPE_API_KEY" });

  let { audio } = req.body || {};
  if (!audio || typeof audio !== "string") return res.status(400).json({ error: "missing audio" });
  if (!audio.startsWith("data:")) audio = `data:audio/wav;base64,${audio}`;

  const body = {
    model: "qwen3-asr-flash",
    input: { messages: [{ role: "user", content: [{ audio }] }] },
  };

  try {
    const r = await fetch(EP, {
      method: "POST",
      headers: { Authorization: `Bearer ${key}`, "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const j = await r.json();
    if (j.code) return res.status(502).json({ error: `${j.code}: ${j.message}` });
    const text = j?.output?.choices?.[0]?.message?.content?.find((c) => c.text)?.text || "";
    return res.status(200).json({ text });
  } catch (e) {
    return res.status(502).json({ error: `upstream error: ${e}` });
  }
};

module.exports.config = { maxDuration: 30 };
