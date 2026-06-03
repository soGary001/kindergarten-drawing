// Vercel serverless function: proxy DashScope qwen-image-2.0-pro image generation.
// The DashScope API key lives ONLY in the Vercel env var DASHSCOPE_API_KEY — never in the app.
// POST { prompt: string, size?: "1280*720" }  ->  200 { url }  |  4xx/5xx { error }

const EP = "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation";

module.exports = async function handler(req, res) {
  if (req.method !== "POST") return res.status(405).json({ error: "POST only" });
  const key = process.env.DASHSCOPE_API_KEY;
  if (!key) return res.status(500).json({ error: "server missing DASHSCOPE_API_KEY" });

  const { prompt, size } = req.body || {};
  if (!prompt || typeof prompt !== "string") return res.status(400).json({ error: "missing prompt" });

  const body = {
    model: "qwen-image-2.0-pro",
    input: { messages: [{ role: "user", content: [{ text: prompt }] }] },
    parameters: { size: size || "1280*720", n: 1 },
  };

  try {
    const r = await fetch(EP, {
      method: "POST",
      headers: { Authorization: `Bearer ${key}`, "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const j = await r.json();
    if (j.code) return res.status(502).json({ error: `${j.code}: ${j.message}` });
    const url = j?.output?.choices?.[0]?.message?.content?.find((c) => c.image)?.image;
    if (!url) return res.status(502).json({ error: "no image in upstream response" });
    return res.status(200).json({ url });
  } catch (e) {
    return res.status(502).json({ error: `upstream error: ${e}` });
  }
};

// On Vercel Pro you can raise this; image generation takes ~10-15s.
module.exports.config = { maxDuration: 60 };
