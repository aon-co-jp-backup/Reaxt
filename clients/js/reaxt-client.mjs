// Reaxt の JS/TS クライアント(依存なし)。Connect の単純呼び出し(POST /pkg.Service/Method、JSON)を叩く。
// ブラウザ・Node 18+・Deno・Bun で動く(fetch が使えればよい)。
export class ReaxtError extends Error {
  constructor(status, code, message) {
    super(message);
    this.name = "ReaxtError";
    this.status = status;
    this.code = code;
  }
}

/** @param {{baseUrl?: string, headers?: Record<string,string>, fetch?: typeof fetch}} [opts] */
export function createClient(opts = {}) {
  const base = (opts.baseUrl ?? "").replace(/\/$/, "");
  const f = opts.fetch ?? globalThis.fetch.bind(globalThis);
  async function call(service, method, input = null) {
    const res = await f(`${base}/${service}/${method}`, {
      method: "POST",
      // x-reaxt はCSRF対策のヘッダー(ブラウザの単純リクエストでは付けられない)
      headers: { "content-type": "application/json", "x-reaxt": "1", ...(opts.headers ?? {}) },
      body: JSON.stringify(input),
    });
    const text = await res.text();
    let json = null;
    try { json = text ? JSON.parse(text) : null; } catch { /* JSONでない応答 */ }
    if (!res.ok) throw new ReaxtError(res.status, json?.code ?? "unknown", json?.message ?? text);
    return json;
  }
  return {
    call,
    /** 公開されているメソッドの一覧(GET /_reaxt/manifest.json) */
    async manifest() {
      const r = await f(`${base}/_reaxt/manifest.json`);
      if (!r.ok) throw new ReaxtError(r.status, "unknown", await r.text());
      return (await r.json()).actions;
    },
    /** 型付きの関数を作る(TSでは defineAction<I,O>(...) の型引数が効く) */
    defineAction(service, method) { return (input) => call(service, method, input); },
  };
}
