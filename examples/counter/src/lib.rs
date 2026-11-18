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

/// Server Action: サーバーが持つカウンターを1増やして新しい値を返す(入力なし)。
pub const INC: reaxt_actions::Action<(), u32> = reaxt_actions::Action::new("demo.v1.Counter", "Inc");

/// アイランド: 初期値(SSR時点のサーバーの値)をpropsで受け、クリックでServer Actionを呼ぶ。
pub fn server_counter(props: &str) -> VNode {
    let start: u32 = props.parse().unwrap_or(0);
    let (n, set) = use_state(|| start);
    let (msg, set_msg) = use_state(String::new);
    let (k, v) = on_click("inc", move || inc_on_server(set.clone(), set_msg.clone()));
    VNode::element("div")
        .attr("class", "sc")
        .child(VNode::element("span").attr("class", "n").child(VNode::text(format!("server={n}"))).build())
        .child(VNode::element("button").attr("class", "b").attr(k, v).child(VNode::text("call server")).build())
        .child(VNode::element("em").attr("class", "m").child(VNode::text(msg)).build())
        .build()
}

#[cfg(target_arch = "wasm32")]
fn inc_on_server(set: reaxt::react::StateSetter<u32>, set_msg: reaxt::react::StateSetter<String>) {
    use reaxt_actions::web::WebClient;
    reaxt_actions::call(&WebClient, &INC, &(), move |r| match r {
        Ok(n) => {
            set.set(n);
            set_msg.set(String::new());
        }
        Err(e) => set_msg.set(format!("error: {e:?}")),
    });
}

/// サーバー側(SSR)ではクリックは起きないので何もしない。
#[cfg(not(target_arch = "wasm32"))]
fn inc_on_server(_set: reaxt::react::StateSetter<u32>, _set_msg: reaxt::react::StateSetter<String>) {}

/// actionsページ(静的な見出し+Server Actionを呼ぶアイランド)。`current`はSSR時点のサーバーの値。
pub fn actions_page(current: u32) -> VNode {
    VNode::element("main")
        .child(VNode::element("h1").child(VNode::text("Reaxt server actions")).build())
        .child(reaxt::hydrate::island("server_counter", &current.to_string(), server_counter))
        .build()
}

#[cfg(target_arch = "wasm32")]
mod boot {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        // Portalのデモ: /portal では本体の外に #modal-root を作り、そこへ別の木を描く
        let loc = web_sys::window().and_then(|w| w.location().pathname().ok()).unwrap_or_default();
        if loc == "/portal" {
            let doc = web_sys::window().unwrap().document().unwrap();
            let div = doc.create_element("div")?;
            div.set_id("modal-root");
            doc.body().unwrap().append_child(&div)?;
            reaxt::wasm::mount("#reaxt-root", 1, super::view)?;
            return reaxt::wasm::portal("#modal-root", 2, super::modal_view).map(|_| ());
        }
        // ページの描画方式で切り替える(サーバーが body に data-reaxt-mode を付ける)
        if reaxt::wasm::page_mode() == "Islands" {
            reaxt::wasm::mount_islands(&[
                ("counter", super::counter_island),
                ("server_counter", super::server_counter),
            ])
            .map(|_| ())
        } else {
            reaxt::wasm::mount("#reaxt-root", 1, super::view).map(|_| ())
        }
    }
}

/// Portal+シグナル+batchのデモ(クライアントだけ)。`#modal-root`(本体の外)へ描かれ、
/// 1回のクリックで2つのシグナルを `batch` で更新→副作用は1回だけ走り、合計が表示に反映される。
pub fn modal_view() -> VNode {
    use reaxt::react::{batch, create_effect, create_signal, use_effect, use_ref, Cleanup};
    let sigs = use_ref(|| (create_signal(1i32), create_signal(1i32)));
    let runs = use_ref(|| 0u32);
    let (sum, set_sum) = use_state(|| 2i32);
    let (runs_shown, set_runs) = use_state(|| 0u32);
    {
        let (a, b) = sigs.borrow().clone();
        let runs = runs.clone();
        use_effect((), move || {
            let e = create_effect(move || {
                *runs.borrow_mut() += 1;
                set_sum.set(a.get() + b.get());
                set_runs.set(*runs.borrow());
            });
            Some(Box::new(move || e.stop()) as Cleanup)
        });
    }
    let (a, b) = sigs.borrow().clone();
    let (k, v) = on_click("bump", move || batch(|| {
        a.set(a.get() + 10);
        b.set(b.get() + 10);
    }));
    VNode::element("div")
        .attr("id", "modal")
        .child(VNode::element("span").attr("id", "sum").child(VNode::text(format!("sum={sum}"))).build())
        .child(VNode::element("span").attr("id", "runs").child(VNode::text(format!("runs={runs_shown}"))).build())
        .child(VNode::element("button").attr("id", "bump").attr(k, v).child(VNode::text("bump")).build())
        .build()
}
