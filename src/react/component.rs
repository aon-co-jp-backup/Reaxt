//! 部品(Component)と自動id(v2 第1段)。`DESIGN-REVIEW.md` の A1・A4 の解消。
//!
//! 部品は `fn(&Props) -> VNode`。親が `VNode::component(f, props)` で子として置くと、描画器(`resolve_root`)が
//! **木の位置+key+関数の種類**から安定した `ComponentId` を自動で振り、その id のフック状態で `f` を実行する。
//! - 別の部品・別の位置は別の id になる(id の取り違えで状態が混ざらない)。
//! - `key` を付けた子は位置ではなく key で識別する(並べ替えても状態が付いて行く)。
//! - 木から消えた部品の状態は、次の描画で自動的に破棄される(`drop_instance`)。
//! - 解決後の木には `VNode::Component` が残らない(SSR・diff・DOM適用は解決済みの木だけを見る)。
//!
//! 互換: 従来の `run_component(id, ..)`(手書きのid)はそのまま使える。自動idは最上位ビットを立てて名前空間を分ける。
//! 現段階の制約: `provide_context` は親の描画関数の中でしか有効でなく、子の部品には届かない(第6段で木に載せる)。

use super::hooks::{drop_instance, run_component, ComponentId};
use super::vnode::VNode;
use std::cell::RefCell;
use std::collections::HashMap;

const AUTO: u64 = 1 << 63;

fn mix(a: u64, b: u64) -> u64 {
    // FNV風の決定的な混ぜ方(プロセスをまたいでも同じ結果)
    let mut h = a ^ 0xcbf2_9ce4_8422_2325;
    for byte in b.to_le_bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn hash_str(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3))
}

thread_local! {
    /// 根ごとの、直前の描画で生きていた自動idの一覧(消えた部品の状態を破棄するため)。
    static LIVE: RefCell<HashMap<u64, Vec<ComponentId>>> = RefCell::new(HashMap::new());
}

/// `tree` の中の `VNode::Component` を、自動idで描画して展開する。何度呼んでも結果は同じ(展開済みなら素通し)。
/// `root_key` は根の識別子(`App` なら `root_id`)。前回の描画で生きていて今回いない部品の状態は破棄する。
pub fn resolve_root(root_key: u64, tree: VNode) -> VNode {
    let mut seen: Vec<ComponentId> = Vec::new();
    let out = resolve_node(tree, mix(root_key, 0), &mut seen);
    LIVE.with(|l| {
        let prev = l.borrow_mut().insert(root_key, seen.clone()).unwrap_or_default();
        for id in prev {
            if !seen.contains(&id) {
                drop_instance(id);
            }
        }
    });
    out
}

/// 根の部品をすべて破棄する(SSRの1要求の終わりなど)。
pub fn release_root(root_key: u64) {
    let prev = LIVE.with(|l| l.borrow_mut().remove(&root_key)).unwrap_or_default();
    for id in prev {
        drop_instance(id);
    }
}

fn resolve_node(node: VNode, path: u64, seen: &mut Vec<ComponentId>) -> VNode {
    match node {
        VNode::Text(_) => node,
        VNode::Element(mut el) => {
            el.children = resolve_children(el.children, path, seen);
            VNode::Element(el)
        }
        VNode::Component(c) => {
            let id = mix(path, c.kind as u64) | AUTO;
            seen.push(id);
            // 子の部品は、この部品の描画の中で展開する(描画後の副作用は一番外側の終わりにまとめて流れる)
            run_component(id, || {
                let out = (c.render)();
                resolve_node(out, id, seen)
            })
        }
    }
}

fn resolve_children(children: Vec<VNode>, parent: u64, seen: &mut Vec<ComponentId>) -> Vec<VNode> {
    children
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            // key があれば位置ではなく key で識別する(並べ替えても状態が付いて行く)
            let child_path = match c.key() {
                Some(k) => mix(parent, hash_str(k) | 1),
                None => mix(parent, (i as u64) << 1),
            };
            resolve_node(c, child_path, seen)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::react::hooks::use_state;
    use crate::react::vnode::VNode;

    fn counter(label: &String) -> VNode {
        let (n, set) = use_state(|| 0i32);
        let s2 = set.clone();
        COUNTERS.with(|c| c.borrow_mut().insert(label.clone(), Box::new(move |v| s2.set(v))));
        VNode::element("b").child(VNode::text(format!("{label}={n}"))).build()
    }
    fn other(_: &()) -> VNode {
        let (n, _) = use_state(|| 100i32);
        VNode::text(format!("other={n}"))
    }
    thread_local! {
        static COUNTERS: RefCell<HashMap<String, Box<dyn Fn(i32)>>> = RefCell::new(HashMap::new());
    }
    fn bump(label: &str, v: i32) {
        COUNTERS.with(|c| (c.borrow()[label])(v));
    }
    fn text(v: &VNode) -> String {
        match v {
            VNode::Text(t) => t.clone(),
            VNode::Element(e) => e.children.iter().map(text).collect::<Vec<_>>().join(","),
            VNode::Component(_) => "<unresolved>".into(),
        }
    }
    fn page(items: &[&str]) -> VNode {
        VNode::element("div")
            .children(items.iter().map(|l| VNode::component_keyed(l.to_string(), counter, l.to_string())))
            .build()
    }

    #[test]
    fn different_components_and_positions_get_different_ids_without_manual_ids() {
        let tree = || {
            VNode::element("div")
                .child(VNode::component(counter, "a".to_string()))
                .child(VNode::component(counter, "b".to_string()))
                .child(VNode::component(other, ()))
                .build()
        };
        let r = resolve_root(1, tree());
        assert_eq!(text(&r), "a=0,b=0,other=100");
        bump("a", 7);
        let r = resolve_root(1, tree());
        assert_eq!(text(&r), "a=7,b=0,other=100", "aの状態がb・otherに漏れない(旧設計のA1)");
        release_root(1);
    }

    #[test]
    fn keyed_children_keep_state_when_reordered_and_unmounted_state_is_dropped() {
        resolve_root(2, page(&["x", "y"]));
        bump("x", 5);
        let r = resolve_root(2, page(&["y", "x"])); // 並べ替え
        assert_eq!(text(&r), "y=0,x=5", "keyで状態が付いて行く");
        let r = resolve_root(2, page(&["y"])); // xが消える
        assert_eq!(text(&r), "y=0");
        let r = resolve_root(2, page(&["y", "x"])); // 再び現れたxは新しい状態
        assert_eq!(text(&r), "y=0,x=0", "消えた部品の状態は破棄される");
        release_root(2);
    }

    #[test]
    fn nested_components_have_their_own_state() {
        fn outer(_: &()) -> VNode {
            let (n, _) = use_state(|| 1i32);
            VNode::element("section")
                .child(VNode::text(format!("outer={n}")))
                .child(VNode::component(counter, "inner".to_string()))
                .build()
        }
        let r = resolve_root(3, VNode::component(outer, ()));
        assert_eq!(text(&r), "outer=1,inner=0");
        bump("inner", 9);
        let r = resolve_root(3, VNode::component(outer, ()));
        assert_eq!(text(&r), "outer=1,inner=9");
        release_root(3);
    }

    #[test]
    fn resolve_is_idempotent_on_resolved_trees() {
        let r = resolve_root(4, VNode::component(other, ()));
        let r2 = resolve_root(4, r.clone());
        assert_eq!(r, r2);
        release_root(4);
    }
}
