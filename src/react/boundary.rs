//! Fragment・Suspense・エラー境界。
//!
//! 設計メモ:
//! - **Fragment**: 要素の中の複数の子は `children(..)`/`fragment(..)` でそのまま平らに並ぶ。
//!   ルートなど「1つのVNodeが要る場所」では、レイアウトに影響しない `display:contents` の包みを使う。
//! - **Suspense**: 描画関数が `Err(Pending)` を返したら代わりに `fallback` を出す。SSRは同期なので
//!   「待つ」のではなく、未解決の間は常にfallbackを出す(ストリーミングSSRは未実装)。
//! - **エラー境界**: 描画関数が `Err` を返したら `fallback(メッセージ)` を出す。panicは捕まえない
//!   (releaseは `panic="abort"` で捕まえられず、途中で止まるとフックの状態が崩れるため)。

use super::vnode::VNode;
use std::fmt::Display;

impl VNode {
    /// 複数のVNodeを1つにまとめる(`<>...</>`)。レイアウトに影響しない `display:contents` の `div` で包む。
    pub fn fragment(children: impl IntoIterator<Item = VNode>) -> VNode {
        VNode::element("div").attr("style", "display:contents").attr("data-rx-fragment", "").children(children).build()
    }
}

/// まだ値が揃っていない印(Suspenseが受け取る)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pending;

/// `render` が `Err(Pending)` を返す間は `fallback` を出す。
pub fn suspense(fallback: VNode, render: impl FnOnce() -> Result<VNode, Pending>) -> VNode {
    render().unwrap_or(fallback)
}

/// `render` が `Err(e)` を返したら `fallback(e の文字列)` を出す(エラー境界)。
pub fn error_boundary<E: Display>(render: impl FnOnce() -> Result<VNode, E>, fallback: impl FnOnce(String) -> VNode) -> VNode {
    match render() {
        Ok(v) => v,
        Err(e) => fallback(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_wraps_without_layout() {
        let f = VNode::fragment([VNode::text("a"), VNode::element("b").build()]);
        let VNode::Element(el) = f else { panic!() };
        assert_eq!(el.attrs.get("style").map(String::as_str), Some("display:contents"));
        assert_eq!(el.children.len(), 2);
    }

    #[test]
    fn suspense_shows_fallback_until_ready() {
        let fb = || VNode::text("loading");
        assert_eq!(suspense(fb(), || Err(Pending)), VNode::text("loading"));
        assert_eq!(suspense(fb(), || Ok(VNode::text("done"))), VNode::text("done"));
    }

    #[test]
    fn error_boundary_catches_err_and_passes_message() {
        let ok = error_boundary(|| Ok::<_, String>(VNode::text("ok")), |m| VNode::text(m));
        assert_eq!(ok, VNode::text("ok"));
        let bad = error_boundary(|| Err::<VNode, _>("boom"), |m| VNode::text(format!("err:{m}")));
        assert_eq!(bad, VNode::text("err:boom"));
    }
}
