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

    // イベント委譲: ルートに1つだけクリックリスナーを付ける
    let rt2 = rt.clone();
    let dom2 = dom.clone();
    let root_node: Node = root.clone().into();
    let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
        let Some(target) = ev.target().and_then(|t| t.dyn_into::<Node>().ok()) else { return };
        let rn = root_node.clone();
        let hit = click_at(&*dom2, &target, |n| n.parent_node(), |n| n.is_same_node(Some(&rn)));
        if hit {
            flush_all();
        }
    });
    let _ = &rt2;
    root.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
    cb.forget();

    root.set_attribute("data-reaxt-hydrated", if adopted { "adopted" } else { "rebuilt" })?;
    register(rt);
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
        let (rt2, dom2, rn) = (rt.clone(), dom.clone(), root_node.clone());
        let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
            let Some(target) = ev.target().and_then(|t| t.dyn_into::<Node>().ok()) else { return };
            let hit = click_at(&*dom2, &target, |n| n.parent_node(), |n| n.is_same_node(Some(&rn)));
            if hit {
                flush_all();
            }
        });
        let _ = &rt2;
        root_node.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
        if let Some(el) = root_node.dyn_ref::<Element>() {
            el.set_attribute("data-reaxt-hydrated", if rt.borrow().adopted { "adopted" } else { "rebuilt" })?;
        }
        register(rt);
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
