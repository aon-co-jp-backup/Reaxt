//! aruaru-admin Web の Reaxt 版(移植の実験)。枠・状態表示・ダッシュボード・コミット履歴・ブランチ。
//! 元は React+TypeScript+Tailwind。ここはRustのみ。データはServer Action経由でaruaru-serverのGraphQLへ。
use reaxt::hydrate::on_click;
use reaxt::react::{use_state, StateSetter, VNode};
use reaxt_actions::Action;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

#[derive(Serialize, Deserialize)]
pub struct GqlIn {
    pub query: String,
}
#[derive(Serialize, Deserialize)]
pub struct GqlOut {
    pub body: String,
}
/// 画面→サーバー(BFF)→aruaru-server の GraphQL(問い合わせ専用)。
pub const GQL: Action<GqlIn, GqlOut> = Action::new("aruaru_admin.v1.Proxy", "Graphql");

pub const QUERY: &str = "query { currentBranch branches { name headCommitId isCurrent } log(limit: 50) { shortId author message timestamp } }";

#[derive(Deserialize, Clone, Default, Debug)]
pub struct Branch {
    pub name: String,
    #[serde(rename = "headCommitId")]
    pub head: String,
    #[serde(rename = "isCurrent")]
    pub current: bool,
}
#[derive(Deserialize, Clone, Default, Debug)]
pub struct Commit {
    #[serde(rename = "shortId")]
    pub short: String,
    pub author: String,
    pub message: String,
    pub timestamp: String,
}
#[derive(Deserialize, Default)]
struct Data {
    #[serde(default, rename = "currentBranch")]
    current: String,
    #[serde(default)]
    branches: Vec<Branch>,
    #[serde(default)]
    log: Vec<Commit>,
}
#[derive(Deserialize)]
struct Resp {
    data: Option<Data>,
}

#[derive(Clone, Default, Debug)]
pub struct Snap {
    /// None=確認前 / Some(true)=接続中 / Some(false)=切断
    pub online: Option<bool>,
    pub current: String,
    pub branches: Vec<Branch>,
    pub commits: Vec<Commit>,
    pub err: String,
}

/// GraphQL応答のJSON(RS-JSONで上限つき読み込み)から画面用の状態を作る。
pub fn parse_snap(body: &str) -> Result<Snap, String> {
    let r: Resp = rust_json::from_str_secure(body, &rust_json::Limits::small()).map_err(|e| e.to_string())?;
    let d = r.data.ok_or("data がありません")?;
    Ok(Snap { online: Some(true), current: d.current, branches: d.branches, commits: d.log, err: String::new() })
}

thread_local! {
    static SETTER: RefCell<Option<StateSetter<Snap>>> = const { RefCell::new(None) };
}

fn publish(s: Snap) {
    SETTER.with(|c| {
        if let Some(set) = c.borrow().as_ref() {
            set.set(s);
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub fn refresh() {
    use reaxt_actions::web::WebClient;
    reaxt_actions::call(&WebClient, &GQL, &GqlIn { query: QUERY.into() }, |r| {
        // 失敗しても直前のデータは残し、接続状態とエラーだけ変える
        let failed = |err: String| SETTER.with(|c| {
            if let Some(set) = c.borrow().as_ref() {
                set.update(|old| Snap { online: Some(false), err: err.clone(), ..old.clone() });
            }
        });
        match r {
            Ok(o) => match parse_snap(&o.body) {
                Ok(s) => publish(s),
                Err(e) => failed(e),
            },
            Err(e) => failed(format!("{e:?}")),
        }
    });
}
#[cfg(not(target_arch = "wasm32"))]
pub fn refresh() {}

fn node(tag: &str, class: &str, kids: Vec<VNode>) -> VNode {
    let mut b = VNode::element(tag);
    if !class.is_empty() {
        b = b.attr("class", class);
    }
    for k in kids {
        b = b.child(k);
    }
    b.build()
}
fn t(s: impl Into<String>) -> VNode {
    VNode::text(s.into())
}

fn nav_button(id: &str, label: &str, active: bool, set: StateSetter<String>) -> VNode {
    let to = id.to_string();
    let (k, v) = on_click(&format!("nav-{id}"), move || set.set(to.clone()));
    VNode::element("button").attr("class", if active { "nav active" } else { "nav" }).attr("id", format!("nav-{id}")).attr(k, v).child(t(label)).build()
}

fn card(label: &str, value: String) -> VNode {
    node("div", "card", vec![node("div", "label", vec![t(label)]), node("div", "value", vec![t(value)])])
}

fn commit_table(commits: &[Commit]) -> VNode {
    let head = node("tr", "", vec![node("th", "", vec![t("ID")]), node("th", "", vec![t("作者")]), node("th", "", vec![t("メッセージ")]), node("th", "", vec![t("日時")])]);
    let mut rows = vec![head];
    for c in commits {
        rows.push(node(
            "tr",
            "",
            vec![
                node("td", "mono", vec![t(c.short.clone())]),
                node("td", "", vec![t(c.author.clone())]),
                node("td", "", vec![t(c.message.clone())]),
                node("td", "dim", vec![t(c.timestamp.clone())]),
            ],
        ));
    }
    node("table", "tbl", rows)
}

fn page_body(page: &str, s: &Snap) -> VNode {
    match page {
        "commits" => node("section", "", vec![node("h2", "", vec![t("コミット履歴")]), commit_table(&s.commits)]),
        "branches" => {
            let mut rows = vec![node("tr", "", vec![node("th", "", vec![t("ブランチ")]), node("th", "", vec![t("HEAD")]), node("th", "", vec![t("")])])];
            for b in &s.branches {
                let short: String = b.head.chars().take(12).collect();
                rows.push(node(
                    "tr",
                    "",
                    vec![node("td", "", vec![t(b.name.clone())]), node("td", "mono", vec![t(short)]), node("td", "cur", vec![t(if b.current { "現在" } else { "" })])],
                ));
            }
            node("section", "", vec![node("h2", "", vec![t("ブランチ")]), node("table", "tbl", rows)])
        }
        _ => {
            let recent: Vec<Commit> = s.commits.iter().take(5).cloned().collect();
            node(
                "section",
                "",
                vec![
                    node("h2", "", vec![t("ダッシュボード")]),
                    node(
                        "div",
                        "cards",
                        vec![
                            card("接続", match s.online { Some(true) => "接続中".into(), Some(false) => "切断".into(), None => "確認中".into() }),
                            card("現在のブランチ", s.current.clone()),
                            card("ブランチ数", s.branches.len().to_string()),
                            card("コミット数(直近50)", s.commits.len().to_string()),
                        ],
                    ),
                    node("h3", "", vec![t("直近のコミット")]),
                    commit_table(&recent),
                ],
            )
        }
    }
}

/// アプリ全体(ブラウザでマウントする根)。
pub fn view() -> VNode {
    let (page, set_page) = use_state(|| "dashboard".to_string());
    let (snap, set_snap) = use_state(Snap::default);
    SETTER.with(|c| *c.borrow_mut() = Some(set_snap));
    let dot = match snap.online {
        Some(true) => "dot on",
        Some(false) => "dot off",
        None => "dot",
    };
    let status = match snap.online {
        Some(true) => format!("接続中 ({})", snap.current),
        Some(false) => format!("切断: {}", snap.err),
        None => "確認中…".to_string(),
    };
    let side = node(
        "aside",
        "side",
        vec![
            node("div", "brand", vec![t("aruaru-admin")]),
            node("div", "group", vec![t("データ")]),
            nav_button("dashboard", "ダッシュボード", page == "dashboard", set_page.clone()),
            nav_button("commits", "コミット", page == "commits", set_page.clone()),
            nav_button("branches", "ブランチ", page == "branches", set_page.clone()),
            node("div", "status", vec![node("span", dot, vec![]), VNode::element("span").attr("id", "status").child(t(status)).build()]),
        ],
    );
    node("div", "app", vec![side, node("main", "main", vec![page_body(&page, &snap)])])
}

#[cfg(target_arch = "wasm32")]
mod boot {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        reaxt::wasm::mount("#reaxt-root", 1, super::view)?;
        super::refresh();
        reaxt::wasm::every(5000, super::refresh);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_graphql_response() {
        let body = r#"{"data":{"currentBranch":"main","branches":[{"name":"main","headCommitId":"abc","isCurrent":true}],"log":[{"shortId":"1","author":"a","message":"m","timestamp":"t","extra":1}]}}"#;
        let s = parse_snap(body).unwrap();
        assert_eq!(s.current, "main");
        assert_eq!(s.branches.len(), 1);
        assert_eq!(s.commits[0].message, "m");
        assert!(parse_snap("{\"errors\":[]}").is_err());
        assert!(parse_snap("nope").is_err());
    }

    #[test]
    fn renders_pages_to_html() {
        let s = parse_snap(r#"{"data":{"currentBranch":"main","branches":[],"log":[{"shortId":"1","author":"a","message":"<b>x</b>","timestamp":"t"}]}}"#).unwrap();
        for p in ["dashboard", "commits", "branches"] {
            let html = reaxt::ssr::render(&page_body(p, &s));
            assert!(!html.contains("<b>x"), "{p}: メッセージはエスケープされる");
        }
    }
}
