//! 段階2: ハイドレーション/クライアント描画の核。ブラウザ非依存(`Dom`トレイトの背後に隠す)。
//!
//! - `apply`: `react::Patch` を実DOMへ適用(ReactDOMの更新反映相当)。
//! - `matches_dom`: SSRで出たDOMがVNode木と構造一致するかの検査(不一致なら作り直す)。
//! - イベント: 属性 `data-rx-click="<id>"` + `on_click(id, f)` の登録簿(インラインJS無し・CSP互換)。
//! - `Runtime`: マウント→(ハイドレート or 描画)→クリック→再レンダー→パッチ適用の一連。
//!
//! ブラウザ実装(web-sys)は `wasm` モジュール(wasm32のみ)。ここはネイティブでテストできる。
use crate::react::diff::{AttrsPatch, ChildPatch, Patch};
use crate::react::{App, ComponentId, VNode};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// ブラウザDOMに相当する最小の操作(web-sys版と、テスト用のメモリ版がある)。
pub trait Dom {
    type Node: Clone;
    fn create(&self, v: &VNode) -> Self::Node;
    fn children(&self, n: &Self::Node) -> Vec<Self::Node>;
    /// 子を丸ごと指定の並びにする(既存の子は外して付け直す)。
    fn set_children(&self, n: &Self::Node, kids: Vec<Self::Node>);
    fn set_text(&self, n: &Self::Node, t: &str);
    fn set_attr(&self, n: &Self::Node, k: &str, v: &str);
    fn remove_attr(&self, n: &Self::Node, k: &str);
    /// 親の中で`old`を`new`に差し替える。
    fn replace(&self, old: &Self::Node, new: &Self::Node);
    /// 要素ならタグ名、テキストなら`None`。
    fn tag(&self, n: &Self::Node) -> Option<String>;
    fn text(&self, n: &Self::Node) -> Option<String>;
    fn attr(&self, n: &Self::Node, k: &str) -> Option<String>;
}

/// `patch`を`node`へ適用する。差し替えが起きたら新しいノードを返す(それ以外は`node`のまま)。
pub fn apply<D: Dom>(d: &D, node: &D::Node, patch: &Patch) -> D::Node {
    match patch {
        Patch::NoOp => node.clone(),
        Patch::Replace(v) => {
            let new = d.create(v);
            d.replace(node, &new);
            new
        }
        Patch::UpdateText(t) => {
            d.set_text(node, t);
            node.clone()
        }
        Patch::UpdateElement { attrs, children } => {
            if let Some(AttrsPatch { set, remove }) = attrs {
                for (k, v) in set {
                    d.set_attr(node, k, v);
                }
                for k in remove {
                    d.remove_attr(node, k);
                }
            }
            if let Some(ps) = children {
                apply_children(d, node, ps);
            }
            node.clone()
        }
    }
}

fn apply_children<D: Dom>(d: &D, parent: &D::Node, patches: &[ChildPatch]) {
    let old = d.children(parent);
    let mut inserts: HashMap<usize, &VNode> = HashMap::new();
    let mut updates: HashMap<usize, (usize, &Patch)> = HashMap::new();
    let mut removed: HashSet<usize> = HashSet::new();
    for p in patches {
        match p {
            ChildPatch::Insert { index, node } => {
                inserts.insert(*index, node);
            }
            ChildPatch::Remove { index } => {
                removed.insert(*index);
            }
            ChildPatch::Update { old_index, new_index, patch } => {
                updates.insert(*new_index, (*old_index, patch.as_ref()));
            }
        }
    }
    let final_len = old.len() + inserts.len() - removed.len();
    let mut out = Vec::with_capacity(final_len);
    for i in 0..final_len {
        if let Some(v) = inserts.get(&i) {
            out.push(d.create(v));
        } else if let Some((oi, p)) = updates.get(&i) {
            out.push(apply(d, &old[*oi], p));
        } else {
            out.push(old[i].clone());
        }
    }
    d.set_children(parent, out);
}

/// SSRで出たDOMがVNode木と構造一致するか(タグ・テキスト・属性。`on*`は出していないので無関係)。
pub fn matches_dom<D: Dom>(d: &D, n: &D::Node, v: &VNode) -> bool {
    match v {
        VNode::Text(t) => d.text(n).as_deref() == Some(t.as_str()),
        VNode::Element(el) => {
            if d.tag(n).map(|t| t.eq_ignore_ascii_case(&el.tag)) != Some(true) {
                return false;
            }
            for (k, val) in &el.attrs {
                if k.to_ascii_lowercase().starts_with("on") {
                    continue;
                }
                if d.attr(n, k).as_deref() != Some(val.as_str()) {
                    return false;
                }
            }
            let kids = d.children(n);
            kids.len() == el.children.len()
                && kids.iter().zip(&el.children).all(|(k, c)| matches_dom(d, k, c))
        }
    }
}

// ---- イベント登録簿 ----

pub const CLICK_ATTR: &str = "data-rx-click";

thread_local! {
    static CLICKS: RefCell<HashMap<String, Rc<dyn Fn()>>> = RefCell::new(HashMap::new());
}

/// クリックハンドラを登録し、要素に付ける属性(名前, 値)を返す。
/// 例: `VNode::element("button").attr(attr.0, attr.1)`。
pub fn on_click(id: &str, f: impl Fn() + 'static) -> (&'static str, String) {
    // アイランド内ではidに`<scope>:`を前置して、同じidの部品が複数あっても衝突させない
    let scope = SCOPE.with(|s| s.get());
    let key = if scope == 0 { id.to_string() } else { format!("{scope}:{id}") };
    CLICKS.with(|c| c.borrow_mut().insert(key.clone(), Rc::new(f)));
    (CLICK_ATTR, key)
}

// ---- Islands(静的HTMLの一部だけを対話化) ----

pub const ISLAND_ATTR: &str = "data-rx-island";
pub const PROPS_ATTR: &str = "data-rx-props";
pub const SCOPE_ATTR: &str = "data-rx-scope";

thread_local! {
    static SCOPE: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static NEXT_ISLAND: std::cell::Cell<u32> = const { std::cell::Cell::new(1) };
}

/// `f`の間だけスコープ(アイランド番号)を切り替える。
pub fn with_scope<T>(scope: u32, f: impl FnOnce() -> T) -> T {
    let prev = SCOPE.with(|s| s.replace(scope));
    let r = f();
    SCOPE.with(|s| s.set(prev));
    r
}

/// ページの描画の頭で呼ぶ(アイランド番号を1から振り直す。サーバーとブラウザで同じ並びになる)。
pub fn reset_islands() {
    NEXT_ISLAND.with(|n| n.set(1));
}

/// アイランドのコンポーネントID(スコープから決まる)。
pub fn island_component_id(scope: u32) -> ComponentId {
    1_000_000 + scope as ComponentId
}

/// サーバー側: アイランドを描画して、印(`data-rx-island`/`-props`/`-scope`)付きのdivで包む。
/// ブラウザ側は印を見て、その部分だけをマウントする。`props`は任意の文字列(通常はJSON)。
pub fn island(name: &str, props: &str, render: fn(&str) -> VNode) -> VNode {
    let scope = NEXT_ISLAND.with(|n| n.replace(n.get() + 1));
    let id = island_component_id(scope);
    let tree = with_scope(scope, || crate::react::run_component(id, || render(props)));
    crate::react::hooks::drop_instance(id);
    VNode::element("div")
        .attr(ISLAND_ATTR, name)
        .attr(PROPS_ATTR, props)
        .attr(SCOPE_ATTR, scope.to_string())
        .child(tree)
        .build()
}

/// ブラウザ側: 印の付いた各ルートをマウントする(登録簿に無い名前は無視)。
pub fn mount_islands<D: Dom>(
    dom: Rc<D>,
    roots: Vec<D::Node>,
    registry: &[(&str, fn(&str) -> VNode)],
) -> Vec<Runtime<D>> {
    let mut out = Vec::new();
    for root in roots {
        let (Some(name), Some(scope)) = (dom.attr(&root, ISLAND_ATTR), dom.attr(&root, SCOPE_ATTR)) else { continue };
        let Some(scope) = scope.parse::<u32>().ok().filter(|s| *s > 0) else { continue };
        let Some((_, render)) = registry.iter().find(|(n, _)| *n == name) else { continue };
        let render = *render;
        let props = dom.attr(&root, PROPS_ATTR).unwrap_or_default();
        out.push(Runtime::mount_scoped(dom.clone(), root, island_component_id(scope), scope, move || render(&props)));
    }
    out
}

/// 登録済みハンドラを実行する。存在すれば`true`。
pub fn dispatch_click(id: &str) -> bool {
    let f = CLICKS.with(|c| c.borrow().get(id).cloned());
    match f {
        Some(f) => {
            f();
            true
        }
        None => false,
    }
}

/// 要素から祖先へ辿って`data-rx-click`を探す(イベント委譲)。見つかったらそのidでハンドラを実行。
pub fn click_at<D: Dom>(d: &D, target: &D::Node, parent_of: impl Fn(&D::Node) -> Option<D::Node>, root_is: impl Fn(&D::Node) -> bool) -> bool {
    let mut cur = Some(target.clone());
    while let Some(n) = cur {
        if let Some(id) = d.attr(&n, CLICK_ATTR) {
            return dispatch_click(&id);
        }
        if root_is(&n) {
            break;
        }
        cur = parent_of(&n);
    }
    false
}

/// マウント済みのクライアント側アプリ。
pub struct Runtime<D: Dom> {
    dom: Rc<D>,
    root: D::Node,
    app: App<Box<dyn Fn() -> VNode>>,
    /// ハイドレーションで既存DOMをそのまま採用できたか。
    pub adopted: bool,
}

impl<D: Dom> Runtime<D> {
    /// `root`は`#reaxt-root`相当。中身がSSR出力と一致すれば採用(`adopted`)、
    /// 空または不一致なら作り直す(CSR/不一致時のフォールバック)。
    pub fn mount(dom: Rc<D>, root: D::Node, id: ComponentId, render: impl Fn() -> VNode + 'static) -> Self {
        Self::mount_scoped(dom, root, id, 0, render)
    }

    /// アイランド用: 再レンダーのたびに`scope`を有効にする(`on_click`のid名前空間)。
    pub fn mount_scoped(dom: Rc<D>, root: D::Node, id: ComponentId, scope: u32, render: impl Fn() -> VNode + 'static) -> Self {
        let wrapped = move || with_scope(scope, &render);
        let app = App::mount(id, Box::new(wrapped) as Box<dyn Fn() -> VNode>);
        let kids = dom.children(&root);
        let adopted = kids.len() == 1 && matches_dom(&*dom, &kids[0], app.tree());
        if !adopted {
            let fresh = dom.create(app.tree());
            dom.set_children(&root, vec![fresh]);
        }
        Runtime { dom, root, app, adopted }
    }

    /// 状態が更新されていれば再レンダー→差分→適用。適用したら`true`。
    pub fn flush(&mut self) -> bool {
        match self.app.tick() {
            Some(patch) => {
                let kids = self.dom.children(&self.root);
                if let Some(top) = kids.first() {
                    let n = apply(&*self.dom, top, &patch);
                    self.dom.set_children(&self.root, vec![n]);
                }
                true
            }
            None => false,
        }
    }

    /// マウント先のルートノード。
    pub fn root_node(&self) -> D::Node {
        self.root.clone()
    }

    /// 現在のツリー(テスト・デバッグ用)。
    pub fn tree(&self) -> &VNode {
        self.app.tree()
    }
}

#[cfg(test)]
pub(crate) mod mem {
    //! テスト用のメモリDOM。
    use super::*;
    use std::collections::BTreeMap;

    pub struct M {
        pub tag: Option<String>,
        pub text: String,
        pub attrs: BTreeMap<String, String>,
        pub kids: Vec<N>,
    }
    pub type N = Rc<RefCell<M>>;
    pub struct MemDom;

    pub fn el(tag: &str) -> N {
        Rc::new(RefCell::new(M { tag: Some(tag.into()), text: String::new(), attrs: BTreeMap::new(), kids: vec![] }))
    }

    pub fn html(n: &N) -> String {
        let m = n.borrow();
        match &m.tag {
            None => m.text.clone(),
            Some(t) => {
                let a: String = m.attrs.iter().map(|(k, v)| format!(" {k}=\"{v}\"")).collect();
                let k: String = m.kids.iter().map(html).collect();
                format!("<{t}{a}>{k}</{t}>")
            }
        }
    }

    impl Dom for MemDom {
        type Node = N;
        fn create(&self, v: &VNode) -> N {
            match v {
                VNode::Text(t) => Rc::new(RefCell::new(M { tag: None, text: t.clone(), attrs: BTreeMap::new(), kids: vec![] })),
                VNode::Element(e) => {
                    let n = el(&e.tag);
                    for (k, val) in &e.attrs {
                        n.borrow_mut().attrs.insert(k.clone(), val.clone());
                    }
                    let kids = e.children.iter().map(|c| self.create(c)).collect();
                    n.borrow_mut().kids = kids;
                    n
                }
            }
        }
        fn children(&self, n: &N) -> Vec<N> {
            n.borrow().kids.clone()
        }
        fn set_children(&self, n: &N, kids: Vec<N>) {
            n.borrow_mut().kids = kids;
        }
        fn set_text(&self, n: &N, t: &str) {
            n.borrow_mut().text = t.into();
        }
        fn set_attr(&self, n: &N, k: &str, v: &str) {
            n.borrow_mut().attrs.insert(k.into(), v.into());
        }
        fn remove_attr(&self, n: &N, k: &str) {
            n.borrow_mut().attrs.remove(k);
        }
        fn replace(&self, old: &N, new: &N) {
            // メモリ版は親ポインタを持たないので、中身ごと差し替える(同一Rcを維持)。
            let (o, nn) = (old.clone(), new.clone());
            let taken = std::mem::replace(&mut *nn.borrow_mut(), M { tag: None, text: String::new(), attrs: BTreeMap::new(), kids: vec![] });
            *o.borrow_mut() = taken;
        }
        fn tag(&self, n: &N) -> Option<String> {
            n.borrow().tag.clone()
        }
        fn text(&self, n: &N) -> Option<String> {
            let m = n.borrow();
            m.tag.is_none().then(|| m.text.clone())
        }
        fn attr(&self, n: &N, k: &str) -> Option<String> {
            n.borrow().attrs.get(k).cloned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mem::*;
    use super::*;
    use crate::react::hooks::use_state;
    use crate::react::diff::diff;
    use crate::ssr;

    fn counter_view() -> VNode {
        let (n, set) = use_state(|| 0i32);
        let (k, v) = on_click("inc", move || set.update(|c| c + 1));
        VNode::element("div")
            .child(VNode::element("p").child(VNode::text(format!("count={n}"))).build())
            .child(VNode::element("button").attr(k, v).child(VNode::text("+")).build())
            .build()
    }

    #[test]
    fn hydrate_adopts_matching_ssr_dom_and_click_updates_in_place() {
        // SSRで出たDOM相当を用意(ハイドレーションは既存DOMを採用する)
        let d = Rc::new(MemDom);
        let root = el("div");
        let ssr_tree = {
            let a = App::mount(9001, counter_view);
            a.tree().clone()
        };
        crate::react::hooks::drop_instance(9001);
        root.borrow_mut().kids = vec![d.create(&ssr_tree)];
        let existing_btn = root.borrow().kids[0].borrow().kids[1].clone();

        let mut rt = Runtime::mount(d.clone(), root.clone(), 9002, counter_view);
        assert!(rt.adopted, "一致するので既存DOMを採用する");
        assert!(Rc::ptr_eq(&root.borrow().kids[0].borrow().kids[1], &existing_btn), "ボタン要素は作り直されない");

        assert!(dispatch_click("inc"));
        assert!(rt.flush());
        assert!(html(&root).contains("count=1"));
        assert!(dispatch_click("inc"));
        assert!(rt.flush());
        assert_eq!(html(&root), format!("<div><div><p>count=2</p><button data-rx-click=\"inc\">+</button></div></div>"));
        assert!(!rt.flush(), "状態が変わらなければ何もしない");
        crate::react::hooks::drop_instance(9002);
    }

    fn island_counter(props: &str) -> VNode {
        let start: i32 = props.parse().unwrap_or(0);
        let (n, set) = use_state(|| start);
        let (k, v) = on_click("inc", move || set.update(|c| c + 1));
        VNode::element("div")
            .child(VNode::element("span").child(VNode::text(format!("n={n}"))).build())
            .child(VNode::element("button").attr(k, v).child(VNode::text("+")).build())
            .build()
    }

    #[test]
    fn islands_are_independent_even_with_the_same_handler_id() {
        // サーバー: 静的ページに2つのアイランド(propsが違う)
        reset_islands();
        let page = VNode::element("main")
            .child(VNode::element("h1").child(VNode::text("static")).build())
            .child(island("counter", "5", island_counter))
            .child(island("counter", "10", island_counter))
            .build();
        let html_ssr = ssr::render(&page);
        assert!(html_ssr.contains("data-rx-scope=\"1\"") && html_ssr.contains("data-rx-scope=\"2\""));
        assert!(html_ssr.contains("data-rx-click=\"1:inc\"") && html_ssr.contains("data-rx-click=\"2:inc\""));

        // ブラウザ相当: SSRのDOMから、印のついた要素だけを取ってマウント
        let d = Rc::new(MemDom);
        let page_dom = d.create(&page);
        let islands: Vec<N> = page_dom.borrow().kids.iter().skip(1).cloned().collect();
        let buttons: Vec<N> = islands.iter().map(|i| i.borrow().kids[0].borrow().kids[1].clone()).collect();
        let mut rts = mount_islands(d.clone(), islands.clone(), &[("counter", island_counter)]);
        assert_eq!(rts.len(), 2);
        assert!(rts.iter().all(|r| r.adopted), "どちらも既存DOMを採用");
        assert!(Rc::ptr_eq(&islands[0].borrow().kids[0].borrow().kids[1], &buttons[0]));

        // 2番目だけを押す → 2番目だけが変わる(1番目はそのまま)
        assert!(dispatch_click("2:inc"));
        // ブラウザでは押したアイランドのリスナーだけがflushする(既知の制約: 更新フラグは全体で1つ)
        assert!(rts[1].flush());
        assert!(!rts[0].flush(), "もう更新は無い");
        assert!(html(&islands[1]).contains("n=11"));
        assert!(html(&islands[0]).contains("n=5"));
        // 静的部分は一切触られない
        assert_eq!(html(&page_dom.borrow().kids[0]), "<h1>static</h1>");
        // 未登録の名前は無視される
        assert!(mount_islands(d, islands, &[("other", island_counter)]).is_empty());
        for s in 1..=2 {
            crate::react::hooks::drop_instance(island_component_id(s));
        }
    }

    #[test]
    fn mismatch_or_empty_root_is_rebuilt() {
        let d = Rc::new(MemDom);
        let root = el("div");
        root.borrow_mut().kids = vec![d.create(&VNode::element("span").child(VNode::text("old")).build())];
        let rt = Runtime::mount(d.clone(), root.clone(), 9003, || VNode::element("b").child(VNode::text("new")).build());
        assert!(!rt.adopted);
        assert_eq!(html(&root), "<div><b>new</b></div>");
        let empty = el("div");
        let rt2 = Runtime::mount(d, empty.clone(), 9004, || VNode::element("i").build());
        assert!(!rt2.adopted);
        assert_eq!(html(&empty), "<div><i></i></div>");
        crate::react::hooks::drop_instance(9003);
        crate::react::hooks::drop_instance(9004);
    }

    #[test]
    fn apply_matches_full_render_for_list_changes() {
        // 追加・削除・置換・属性変更が混在しても、適用結果が新しい木のレンダリングと一致する
        let li = |k: &str, t: &str| VNode::element("li").key(k).child(VNode::text(t)).build();
        let old = VNode::element("ul").attr("class", "a").children([li("a", "A"), li("b", "B"), li("c", "C")]).build();
        let new = VNode::element("ul").attr("id", "x").children([li("c", "C!"), li("a", "A"), li("d", "D")]).build();
        let d = MemDom;
        let root = d.create(&old);
        let p = diff(&old, &new);
        let top = apply(&d, &root, &p);
        assert_eq!(html(&top), html(&d.create(&new)));
        // SSRの文字列とも矛盾しない(属性の並び順を除き、要素構造が同じ)
        assert!(ssr::render(&new).starts_with("<ul id=\"x\">"));
    }
}
