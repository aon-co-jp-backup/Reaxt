//! Preact 風の呼び名(Rust版)。`h` / `Fragment` / `clone_element` / `render_to_string`(preact-render-to-string)/
//! `@preact/signals` 相当の `signal` / `computed` / `effect` / `use_signal`。実体は既存の仕組みと同じで、別の描画系は無い。
//!
//! 対応しないもの(Rustの型と合わない/JS固有): `Component` クラス(関数コンポーネントとフックを使う)、
//! `preact/compat` が提供するReact互換のJS資産の読み込み、`dangerouslySetInnerHTML`(安全のため非対応)。
use super::signals::{self, Effect, Memo, Signal};
use super::vnode::VNode;

/// Preactの `h(tag, props, ...children)`。`on*` 属性はSSR側が出力しないので、イベントは `on_click` 等を使う。
pub fn h<'a>(tag: &str, props: impl IntoIterator<Item = (&'a str, &'a str)>, children: impl IntoIterator<Item = VNode>) -> VNode {
    let mut b = VNode::element(tag);
    for (k, v) in props {
        if k == "key" {
            b = b.key(v);
        } else {
            b = b.attr(k, v);
        }
    }
    b.children(children).build()
}

/// Preactの `Fragment`(`display:contents` の包みで1つにまとめる)。
#[allow(non_snake_case)]
pub fn Fragment(children: impl IntoIterator<Item = VNode>) -> VNode {
    VNode::fragment(children)
}

/// Preactの `cloneElement(vnode, props)`: 属性を足す/上書きした複製を返す(テキストはそのまま)。
pub fn clone_element<'a>(v: &VNode, props: impl IntoIterator<Item = (&'a str, &'a str)>) -> VNode {
    match v {
        VNode::Text(_) | VNode::Component(_) => v.clone(),
        VNode::Element(el) => {
            let mut el = el.clone();
            for (k, val) in props {
                if k == "key" {
                    el.key = Some(val.to_string());
                } else {
                    el.attrs.insert(k.to_string(), val.to_string());
                }
            }
            VNode::Element(el)
        }
    }
}

/// `preact-render-to-string` 相当: VNodeをHTML文字列にする(エスケープ済み)。
pub fn render_to_string(v: &VNode) -> String {
    crate::ssr::render(v)
}

/// `@preact/signals` の `signal(v)`。読む=`.get()`(Preactの`.value`)、書く=`.set()`。
pub fn signal<T: Clone + 'static>(v: T) -> Signal<T> {
    signals::create_signal(v)
}

/// `computed(() => ...)`。
pub fn computed<T: Clone + PartialEq + 'static>(f: impl FnMut() -> T + 'static) -> Memo<T> {
    signals::create_memo(f)
}

/// `effect(() => ...)`。返す `Effect` を `stop()` すると止まる。
pub fn effect(f: impl FnMut() + 'static) -> Effect {
    signals::create_effect(f)
}

/// `useSignal(v)`: コンポーネントの中で1つだけ作られ、再描画をまたいで同じシグナルを返す。
pub fn use_signal<T: Clone + 'static>(init: impl FnOnce() -> T) -> Signal<T> {
    let r = super::hooks::use_ref(|| signals::create_signal(init()));
    let s = r.borrow().clone();
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::react::hooks::{drop_instance, run_component};

    #[test]
    fn h_builds_elements_with_key_and_children() {
        let v = h("li", [("class", "a"), ("key", "k1")], [VNode::text("x")]);
        let VNode::Element(el) = v else { panic!() };
        assert_eq!((el.tag.as_str(), el.key.as_deref()), ("li", Some("k1")));
        assert_eq!(el.attrs.get("class").map(String::as_str), Some("a"));
        assert!(!el.attrs.contains_key("key"));
    }

    #[test]
    fn clone_element_overrides_props_and_render_escapes() {
        let v = h("a", [("href", "/x")], [VNode::text("<t>")]);
        let c = clone_element(&v, [("href", "/y"), ("rel", "n")]);
        assert_eq!(render_to_string(&c), "<a href=\"/y\" rel=\"n\">&lt;t&gt;</a>");
    }

    #[test]
    fn use_signal_is_stable_across_renders_and_signals_work() {
        let id = 40;
        let a = run_component(id, || use_signal(|| 1));
        a.set(5);
        let b = run_component(id, || use_signal(|| 1));
        assert_eq!(b.get(), 5, "同じシグナルが返る");
        let c = computed({
            let b = b.clone();
            move || b.get() * 2
        });
        assert_eq!(c.get(), 10);
        drop_instance(id);
    }
}
