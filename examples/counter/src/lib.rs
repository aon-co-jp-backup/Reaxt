//! SSR(サーバー)とブラウザ(WASM)で同じ関数を使うカウンター。
use reaxt::hydrate::on_click;
use reaxt::react::{use_state, VNode};

/// サーバーでもWASMでも同じ純関数。
pub fn view() -> VNode {
    let (n, set) = use_state(|| 0i32);
    let (k, v) = on_click("inc", move || set.update(|c| c + 1));
    VNode::element("div")
        .child(VNode::element("h1").child(VNode::text("Reaxt counter")).build())
        .child(VNode::element("p").attr("id", "count").child(VNode::text(format!("count={n}"))).build())
        .child(VNode::element("button").attr("id", "inc").attr(k, v).child(VNode::text("+1")).build())
        .build()
}

#[cfg(target_arch = "wasm32")]
mod boot {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        reaxt::wasm::mount("#reaxt-root", 1, super::view).map(|_| ())
    }
}
