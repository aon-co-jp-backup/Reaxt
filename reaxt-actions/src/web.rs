//! ブラウザ(wasm32)用クライアント: 同一オリジンへfetch。応答のあと全アイランドを再描画する。
//! `x-reaxt: 1` を必ず付ける(カスタムヘッダはクロスオリジンだとCORSの事前確認が要るので、
//! サーバー側の`csrf_guard`と合わせてCSRFを防ぐ)。
use crate::{Client, RawResult};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, RequestInit, Response};

pub struct WebClient;

fn js(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}

async fn fetch(path: &str, body: &str) -> RawResult {
    let win = web_sys::window().ok_or("no window")?;
    let headers = Headers::new().map_err(js)?;
    headers.set("content-type", "application/json").map_err(js)?;
    headers.set("x-reaxt", "1").map_err(js)?;
    let init = RequestInit::new();
    init.set_method("POST");
    init.set_headers(&headers);
    init.set_body(&JsValue::from_str(body));
    let resp = JsFuture::from(win.fetch_with_str_and_init(path, &init)).await.map_err(js)?;
    let resp: Response = resp.dyn_into().map_err(|_| "not a Response".to_string())?;
    let text = JsFuture::from(resp.text().map_err(js)?).await.map_err(js)?.as_string().unwrap_or_default();
    Ok((resp.status(), text))
}

impl Client for WebClient {
    fn post(&self, path: &str, body: String, cb: Box<dyn FnOnce(RawResult)>) {
        let path = path.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            let r = fetch(&path, &body).await;
            cb(r);
            // コールバックが状態を更新しているので、全アイランドを描画し直す
            reaxt::hydrate::request_flush();
        });
    }
}
