//! VNode→HTML(SSR)。テキスト/属性値は必ずエスケープする。空要素は閉じタグ無し。
use crate::react::VNode;

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr",
];

pub fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
}

fn safe_name(n: &str) -> bool {
    !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':'))
}

pub fn render(node: &VNode) -> String {
    let mut out = String::new();
    write(node, &mut out);
    out
}

fn write(node: &VNode, out: &mut String) {
    match node {
        VNode::Text(t) => escape(t, out),
        VNode::Element(el) => {
            // 不正なタグ名・属性名(注入の足場)は出力しない
            if !safe_name(&el.tag) {
                return;
            }
            out.push('<');
            out.push_str(&el.tag);
            for (k, v) in &el.attrs {
                // on*属性(インラインJS)は出さない: 動作はWASM側で結ぶ
                if !safe_name(k) || k.to_ascii_lowercase().starts_with("on") {
                    continue;
                }
                out.push(' ');
                out.push_str(k);
                out.push_str("=\"");
                escape(v, out);
                out.push('"');
            }
            out.push('>');
            if VOID.contains(&el.tag.as_str()) {
                return;
            }
            for c in &el.children {
                write(c, out);
            }
            out.push_str("</");
            out.push_str(&el.tag);
            out.push('>');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escapes_and_blocks_injection() {
        let n = VNode::element("p")
            .attr("title", "a\"b")
            .attr("onclick", "x()")
            .attr("bad name", "x")
            .child(VNode::text("<script>1</script>"))
            .build();
        assert_eq!(render(&n), "<p title=\"a&quot;b\">&lt;script&gt;1&lt;/script&gt;</p>");
        assert_eq!(render(&VNode::element("br").build()), "<br>");
        assert_eq!(render(&VNode::element("a b").build()), "");
    }
}
