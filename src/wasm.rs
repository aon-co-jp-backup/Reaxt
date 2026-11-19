//! ブラウザ(wasm32)用の `Dom` 実装(web-sys)と `mount`。ロジックは `hydrate` に集約済み。
use crate::hydrate::{click_at, Dom, Runtime};
use crate::react::{ComponentId, VNode};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Node};

pub struct WebDom {
    doc: Document,
}

impl WebDom {
    pub fn new() -> Self {
        WebDom { doc: web_sys::window().expect("window").document().expect("document") }
    }
}

impl Default for WebDom {
    fn default() -> Self {
        Self::new()
    }
}

fn safe(name: &str) -> bool {
    !name.is_empty()
        && !name.to_ascii_lowercase().starts_with("on")
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
}

impl Dom for WebDom {
    type Node = Node;

    fn create(&self, v: &VNode) -> Node {
        match v {
            VNode::Text(t) => self.doc.create_text_node(t).into(),
            VNode::Component(_) => self.doc.create_text_node("").into(), // 解決済みの木には現れない
            VNode::Element(e) => {
                let el = match self.doc.create_element(&e.tag) {
                    Ok(el) => el,
                    // 不正なタグ名は空のspanにして描画を続ける(SSR側は出力しない)
                    Err(_) => self.doc.create_element("span").expect("span"),
                };
                for (k, val) in &e.attrs {
                    if safe(k) {
                        let _ = el.set_attribute(k, val);
                    }
                }
                for c in &e.children {
                    let _ = el.append_child(&self.create(c));
                }
                el.into()
            }
        }
    }

    fn children(&self, n: &Node) -> Vec<Node> {
        let list = n.child_nodes();
        (0..list.length()).filter_map(|i| list.item(i)).collect()
    }

    fn set_children(&self, n: &Node, kids: Vec<Node>) {
        let cur = self.children(n);
        // 並びも中身も同じなら触らない(フォーカスやスクロールを保つ)
        if cur.len() == kids.len() && cur.iter().zip(&kids).all(|(a, b)| a.is_same_node(Some(b))) {
            return;
        }
        for c in &cur {
            let _ = n.remove_child(c);
        }
        for k in &kids {
            let _ = n.append_child(k);
        }
    }

    fn set_text(&self, n: &Node, t: &str) {
        n.set_text_content(Some(t));
    }

    fn set_attr(&self, n: &Node, k: &str, v: &str) {
        if let (true, Some(el)) = (safe(k), n.dyn_ref::<Element>()) {
            let _ = el.set_attribute(k, v);
        }
    }

    fn remove_attr(&self, n: &Node, k: &str) {
        if let Some(el) = n.dyn_ref::<Element>() {
            let _ = el.remove_attribute(k);
        }
    }

    fn replace(&self, old: &Node, new: &Node) {
        if let Some(p) = old.parent_node() {
            let _ = p.replace_child(new, old);
        }
    }

    fn tag(&self, n: &Node) -> Option<String> {
        n.dyn_ref::<Element>().map(|e| e.tag_name())
    }

    fn text(&self, n: &Node) -> Option<String> {
        (n.node_type() == Node::TEXT_NODE).then(|| n.text_content().unwrap_or_default())
    }

    fn attr(&self, n: &Node, k: &str) -> Option<String> {
        n.dyn_ref::<Element>().and_then(|e| e.get_attribute(k))
    }
}

type Rt = Rc<RefCell<Runtime<WebDom>>>;

thread_local! {
    static ALL: RefCell<Vec<Rt>> = const { RefCell::new(Vec::new()) };
}

fn register(rt: Rt) {
    ALL.with(|a| a.borrow_mut().push(rt));
    // クリック以外(fetch応答など)の更新から再描画できるよう、全体再描画をフックに登録
    crate::hydrate::set_flush_hook(flush_all);
}

/// 更新(dirty)があれば、マウント済みの全アプリ/アイランドを再レンダー→差分適用する。
/// 更新フラグは全体で1つなので、1つが消費して他が取りこぼすことが無いよう全部を回す。
pub fn flush_all() {
    if !crate::react::hooks::any_dirty() {
        return;
    }
    let all: Vec<Rt> = ALL.with(|a| a.borrow().clone());
    for rt in &all {
        rt.borrow_mut().flush_forced();
    }
    crate::react::hooks::clear_all_dirty();
}

/// 入力要素の現在の値(input/textarea/select)。
fn value_of(n: &Node) -> Option<String> {
    if let Some(e) = n.dyn_ref::<web_sys::HtmlInputElement>() {
        return Some(if e.type_() == "checkbox" { e.checked().to_string() } else { e.value() });
    }
    if let Some(e) = n.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        return Some(e.value());
    }
    n.dyn_ref::<web_sys::HtmlSelectElement>().map(|e| e.value())
}

/// ルート要素にイベント委譲のリスナー(click・input・change・submit)を付ける。
fn listen(dom: &Rc<WebDom>, root: Node) -> Result<(), JsValue> {
    for name in ["click", "input", "change", "submit"] {
        let (dom2, rn) = (dom.clone(), root.clone());
        let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
            let Some(target) = ev.target().and_then(|t| t.dyn_into::<Node>().ok()) else { return };
            let parent = |n: &Node| n.parent_node();
            let is_root = |n: &Node| n.is_same_node(Some(&rn));
            let hit = match ev.type_().as_str() {
                "click" => click_at(&*dom2, &target, parent, is_root),
                "submit" => {
                    let h = crate::hydrate::submit_at(&*dom2, &target, parent, is_root);
                    if h {
                        ev.prevent_default();
                    }
                    h
                }
                // input と change のどちらでも値を渡す(selectやcheckboxはchangeだけのことがある)
                _ => match value_of(&target) {
                    Some(v) => crate::hydrate::input_at(&*dom2, &target, &v, parent, is_root),
                    None => false,
                },
            };
            if hit {
                flush_all();
            }
        });
        root.add_event_listener_with_callback(name, cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    Ok(())
}

/// `selector`(例 `#reaxt-root`)の要素へアプリをマウントする。
/// 中身がSSR出力と一致すればそのまま採用(ハイドレーション)、空/不一致なら描画し直す。
/// 戻り値は採用できたか。
pub fn mount(selector: &str, id: ComponentId, render: impl Fn() -> VNode + 'static) -> Result<bool, JsValue> {
    let dom = Rc::new(WebDom::new());
    let root: Element = dom
        .doc
        .query_selector(selector)?
        .ok_or_else(|| JsValue::from_str("reaxt: root element not found"))?;
    let rt = Rc::new(RefCell::new(Runtime::mount(dom.clone(), root.clone().into(), id, render)));
    let adopted = rt.borrow().adopted;

    // イベント委譲: ルートに click/input/submit のリスナーを1つずつ付ける
    listen(&dom, root.clone().into())?;

    root.set_attribute("data-reaxt-hydrated", if adopted { "adopted" } else { "rebuilt" })?;
    register(rt);
    flush_all(); // マウント直後の副作用(use_effect)が状態を更新していれば、すぐ描画に反映する
    Ok(adopted)
}

/// 印(`data-rx-island`)の付いた全要素を探し、登録簿にある名前のものだけをマウントする(Islands)。
/// 各アイランドのルートにだけクリックリスナーを付け、押されたアイランドだけを再描画する。
/// 戻り値はマウントできた数。
pub fn mount_islands(registry: &[(&str, fn(&str) -> VNode)]) -> Result<usize, JsValue> {
    let dom = Rc::new(WebDom::new());
    let list = dom.doc.query_selector_all(&format!("[{}]", crate::hydrate::ISLAND_ATTR))?;
    let roots: Vec<Node> = (0..list.length()).filter_map(|i| list.item(i)).collect();
    let rts = crate::hydrate::mount_islands(dom.clone(), roots, registry);
    let n = rts.len();
    for rt in rts {
        let root_node = rt.root_node();
        let rt = Rc::new(RefCell::new(rt));
        listen(&dom, root_node.clone())?;
        if let Some(el) = root_node.dyn_ref::<Element>() {
            el.set_attribute("data-reaxt-hydrated", if rt.borrow().adopted { "adopted" } else { "rebuilt" })?;
        }
        register(rt);
        flush_all(); // マウント直後の副作用(use_effect)が状態を更新していれば、すぐ描画に反映する
    }
    Ok(n)
}

/// `ms`ミリ秒ごとに`f`を呼ぶ(setInterval)。呼んだあと、状態更新があれば全体を再描画する。
pub fn every(ms: i32, mut f: impl FnMut() + 'static) {
    let cb = Closure::<dyn FnMut()>::new(move || {
        f();
        flush_all();
    });
    if let Some(w) = web_sys::window() {
        let _ = w.set_interval_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), ms);
    }
    cb.forget();
}

/// サーバーが`<body data-reaxt-mode="...">`に付けた描画方式("Ssr"/"SsrHydrate"/"Csr"/"Islands")。
pub fn page_mode() -> String {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
        .and_then(|b| b.get_attribute("data-reaxt-mode"))
        .unwrap_or_default()
}

/// Portal: `selector` の要素(例 `#modal-root`)へ、別の木として描画する。
/// 要素の位置はページの外側でも構わない(モーダル・トースト等)。クリック・入力のリスナーはその要素に付き、
/// 状態更新は他の木と同じく `flush_all` で再描画される。
///
/// Reactの Portal との違い: 親の木の子ではなく独立した木なので、`provide_context` の値は引き継がれない
/// (共有したい値はシグナルかコンテキストを子の描画関数の中で提供し直す)。SSRには出ない(クライアントだけ)。
/// 戻り値は既存DOMを採用できたか(通常は空なので `false`)。
pub fn portal(selector: &str, id: ComponentId, render: impl Fn() -> VNode + 'static) -> Result<bool, JsValue> {
    mount(selector, id, render)
}
