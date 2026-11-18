//! SSR(サーバー)とブラウザ(WASM)で同じ関数を使うカウンター。
//! - `view`: ページ全体をハイドレーションする例(index)。
//! - `counter_island`: 静的ページの一部だけを対話化する例(islands)。
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

/// アイランド: `props`は初期値(文字列)。同じ部品を複数置いても、状態もクリックも独立する。
pub fn counter_island(props: &str) -> VNode {
    let start: i32 = props.parse().unwrap_or(0);
    let (n, set) = use_state(|| start);
    let (k, v) = on_click("inc", move || set.update(|c| c + 1));
    VNode::element("div")
        .attr("class", "ctr")
        .child(VNode::element("span").attr("class", "n").child(VNode::text(format!("n={n}"))).build())
        .child(VNode::element("button").attr("class", "b").attr(k, v).child(VNode::text("+")).build())
        .build()
}

/// islandsページ(静的な見出し+2つのアイランド+静的な段落)。
pub fn islands_page() -> VNode {
    VNode::element("main")
        .child(VNode::element("h1").child(VNode::text("Reaxt islands")).build())
        .child(reaxt::hydrate::island("counter", "5", counter_island))
        .child(reaxt::hydrate::island("counter", "100", counter_island))
        .child(VNode::element("p").attr("id", "static").child(VNode::text("ここは静的HTML(JSは触らない)")).build())
        .build()
}

#[cfg(target_arch = "wasm32")]
mod boot {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        // ページの描画方式で切り替える(サーバーが body に data-reaxt-mode を付ける)
        if reaxt::wasm::page_mode() == "Islands" {
            reaxt::wasm::mount_islands(&[("counter", super::counter_island)]).map(|_| ())
        } else {
            reaxt::wasm::mount("#reaxt-root", 1, super::view).map(|_| ())
        }
    }
}
