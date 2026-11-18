//! ストリーミングSSR(順不同の差し替え)。
//! 先に「シェル」(待ち中の場所にはfallback)を送り、解決した部分を後から `<template>` と小さなスクリプトで
//! 所定の位置へ差し替える(Reactの `renderToPipeableStream` と同じ考え方)。
//! ここでは**チャンク列を作る純関数**までを提供する。HTTP(chunked)への流し込みは、ホスト側(RPoem等)が
//! 返す `Vec<String>` を順に書けばよい。まとめて1回で返しても、結合した文字列は有効なHTMLになる。
use crate::react::VNode;
use crate::ssr;

/// 待ち中の1か所: 解決したら出すVNodeを返す関数。
pub struct Pending {
    resolve: Box<dyn FnOnce() -> VNode>,
}

impl Pending {
    pub fn new(resolve: impl FnOnce() -> VNode + 'static) -> Self {
        Pending { resolve: Box::new(resolve) }
    }
}

/// 差し替え先の印(`fallback`を包む要素のid)。
pub fn slot_id(n: usize) -> String {
    format!("rx-s-{n}")
}

/// `fallback` を `<div id="rx-s-N" style="display:contents">` で包んだVNode(シェルに置く)。
pub fn slot(n: usize, fallback: VNode) -> VNode {
    VNode::element("div").attr("id", slot_id(n)).attr("style", "display:contents").child(fallback).build()
}

/// 解決した1か所の差し替えチャンク。`<template>`に本文を入れ、スクリプトで差し替えて自身を消す。
pub fn resolved_chunk(n: usize, content: &VNode) -> String {
    let html = ssr::render(content);
    format!(
        "<template id=\"rx-r-{n}\">{html}</template><script>(function(){{var t=document.getElementById(\"rx-r-{n}\"),s=document.getElementById(\"{sid}\");if(t&&s){{s.replaceChildren(t.content);}}if(t)t.remove();var me=document.currentScript;if(me)me.remove();}})();</script>",
        sid = slot_id(n)
    )
}

/// チャンク列: [シェルのHTML, 解決1, 解決2, ...]。`pendings[i]` は `slot(i, ..)` の場所に対応する。
/// `shell` は文書の本文に当たるVNode(中に `slot(i, fallback)` を置いておく)。
pub fn chunks(shell: &VNode, pendings: Vec<Pending>) -> Vec<String> {
    let mut out = vec![ssr::render(shell)];
    for (i, p) in pendings.into_iter().enumerate() {
        out.push(resolved_chunk(i, &(p.resolve)()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_first_then_replacements_in_order() {
        let shell = VNode::element("main").child(slot(0, VNode::text("読み込み中"))).child(slot(1, VNode::text("..."))).build();
        let c = chunks(&shell, vec![Pending::new(|| VNode::text("A")), Pending::new(|| VNode::element("b").child(VNode::text("B<")).build())]);
        assert_eq!(c.len(), 3);
        assert!(c[0].contains("id=\"rx-s-0\"") && c[0].contains("読み込み中"), "{}", c[0]);
        assert!(c[1].contains("<template id=\"rx-r-0\">A</template>"));
        assert!(c[2].contains("<template id=\"rx-r-1\"><b>B&lt;</b></template>"), "本文はエスケープされる: {}", c[2]);
        assert!(c[1].contains("rx-s-0"));
    }
}
