//! aruaru-admin Web の Reaxt 版(移植の実験)。枠・状態表示・ダッシュボード・コミット履歴・ブランチ。
//! 元は React+TypeScript+Tailwind。ここはRustのみ。データはServer Action経由でaruaru-serverのGraphQLへ。
//! JSONはRS-JSONの`light`(依存ゼロ、serde/serde_json無し)で読み書きする。
use reaxt::hydrate::on_click;
use reaxt::react::{use_state, StateSetter, VNode};
use reaxt_actions::{bool_field, obj_json, quote_json, str_field, Action, Wire};
use rust_json::LightValue;
pub mod ops;
use std::cell::RefCell;

pub struct GqlIn {
    pub query: String,
}
impl Wire for GqlIn {
    fn to_wire(&self) -> String {
        obj_json(&[("query", quote_json(&self.query))])
    }
    fn from_wire(v: &LightValue) -> Result<Self, String> {
        Ok(GqlIn { query: str_field(v, "query")? })
    }
}
pub struct GqlOut {
    pub body: String,
}
impl Wire for GqlOut {
    fn to_wire(&self) -> String {
        obj_json(&[("body", quote_json(&self.body))])
    }
    fn from_wire(v: &LightValue) -> Result<Self, String> {
        Ok(GqlOut { body: str_field(v, "body")? })
    }
}
/// 画面→サーバー(BFF)→aruaru-server の GraphQL(問い合わせ専用)。
pub const GQL: Action<GqlIn, GqlOut> = Action::new("aruaru_admin.v1.Proxy", "Graphql");

pub struct SqlIn {
    pub sql: String,
    pub auto_commit: bool,
}
impl Wire for SqlIn {
    fn to_wire(&self) -> String {
        obj_json(&[("sql", quote_json(&self.sql)), ("auto_commit", self.auto_commit.to_string())])
    }
    fn from_wire(v: &LightValue) -> Result<Self, String> {
        Ok(SqlIn { sql: str_field(v, "sql")?, auto_commit: bool_field(v, "auto_commit")? })
    }
}
/// SQL実行(書き込み)。サーバー側で`execSql`のmutationを組み立てて中継する。
pub const SQL: Action<SqlIn, GqlOut> = Action::new("aruaru_admin.v1.Proxy", "ExecSql");

/// `execSql` の GraphQL mutation を作る(SQL文字列はJSON文字列と同じ規則でエスケープ)。
pub fn exec_sql_mutation(sql: &str, auto_commit: bool) -> Result<String, String> {
    let s = quote_json(sql);
    Ok(format!("mutation {{ execSql(sql: {s}, autoCommit: {auto_commit}, commitMessage: \"aruaru-admin\") {{ success commitId message }} }}"))
}

pub const QUERY: &str = "query { currentBranch branches { name headCommitId isCurrent } log(limit: 50) { shortId author message timestamp } }";

#[derive(Clone, Default, Debug)]
pub struct Branch {
    pub name: String,
    pub head: String,
    pub current: bool,
}
#[derive(Clone, Default, Debug)]
pub struct Commit {
    pub short: String,
    pub author: String,
    pub message: String,
    pub timestamp: String,
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

fn s(v: &LightValue, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

/// GraphQL応答のJSON(RS-JSONで上限つき読み込み)から画面用の状態を作る。
pub fn parse_snap(body: &str) -> Result<Snap, String> {
    let r = rust_json::parse_light_secure(body, &rust_json::LightLimits::small()).map_err(|e| e.to_string())?;
    let d = r.get("data").filter(|d| d.as_object().is_some()).ok_or("data がありません")?;
    let branches = d.get("branches").and_then(|x| x.as_array()).unwrap_or_default().iter().map(|b| Branch { name: s(b, "name"), head: s(b, "headCommitId"), current: b.get("isCurrent").and_then(|x| x.as_bool()).unwrap_or(false) }).collect();
    let commits = d.get("log").and_then(|x| x.as_array()).unwrap_or_default().iter().map(|c| Commit { short: s(c, "shortId"), author: s(c, "author"), message: s(c, "message"), timestamp: s(c, "timestamp") }).collect();
    Ok(Snap { online: Some(true), current: s(d, "currentBranch"), branches, commits, err: String::new() })
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

/// クエリ画面の実行。応答(または失敗)を`set_out`へ。
#[cfg(target_arch = "wasm32")]
fn run_query(mode: &str, text: String, auto: bool, set_out: StateSetter<String>) {
    use reaxt_actions::web::WebClient;
    set_out.set("実行中…".into());
    let done = move |r: Result<GqlOut, reaxt_actions::ActionError>| {
        set_out.set(match r {
            Ok(o) => o.body,
            Err(reaxt_actions::ActionError::Server { message, .. }) => format!("エラー: {message}"),
            Err(e) => format!("{e:?}"),
        })
    };
    if mode == "sql" {
        reaxt_actions::call(&WebClient, &SQL, &SqlIn { sql: text, auto_commit: auto }, done);
    } else {
        reaxt_actions::call(&WebClient, &GQL, &GqlIn { query: text }, done);
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn run_query(_: &str, _: String, _: bool, _: StateSetter<String>) {}

struct QueryForm {
    mode: String,
    text: String,
    auto: bool,
    out: String,
    set_mode: StateSetter<String>,
    set_text: StateSetter<String>,
    set_auto: StateSetter<bool>,
    set_out: StateSetter<String>,
}

fn query_page(f: &QueryForm) -> VNode {
    use reaxt::hydrate::{on_input, on_submit};
    let mode_btn = |m: &'static str, label: &str| {
        let set = f.set_mode.clone();
        let (k, v) = on_click(&format!("mode-{m}"), move || set.set(m.to_string()));
        VNode::element("button").attr("type", "button").attr("id", format!("mode-{m}")).attr("class", if f.mode == m { "tab active" } else { "tab" }).attr(k, v).child(t(label)).build()
    };
    let set_text = f.set_text.clone();
    let (ik, iv) = on_input("q-text", move |v| set_text.set(v));
    let set_auto = f.set_auto.clone();
    let (ak, av) = on_input("q-auto", move |v| set_auto.set(v == "true"));
    let (mode, text, auto, set_out) = (f.mode.clone(), f.text.clone(), f.auto, f.set_out.clone());
    let (sk, sv) = on_submit("q-run", move || run_query(&mode, text.clone(), auto, set_out.clone()));
    let placeholder = if f.mode == "sql" { "INSERT INTO ... / CREATE TABLE ..." } else { "query { branches { name } }" };
    let mut row = vec![mode_btn("sql", "SQL"), mode_btn("graphql", "GraphQL")];
    if f.mode == "sql" {
        row.push(node(
            "label",
            "chk",
            vec![VNode::element("input").attr("type", "checkbox").attr("id", "q-auto").attr(ak, av).build(), t("自動コミット")],
        ));
    }
    let mut form = VNode::element("form").attr("class", "qform").attr(sk, sv);
    form = form.child(node("div", "row", row));
    form = form.child(VNode::element("textarea").attr("id", "q-text").attr("rows", "8").attr("spellcheck", "false").attr("placeholder", placeholder).attr(ik, iv).build());
    form = form.child(VNode::element("button").attr("type", "submit").attr("id", "q-run").attr("class", "run").child(t("実行")).build());
    let mut kids = vec![node("h2", "", vec![t("クエリ")]), form.build()];
    if !f.out.is_empty() {
        kids.push(VNode::element("pre").attr("id", "q-out").child(t(f.out.clone())).build());
    }
    node("section", "", kids)
}

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

fn page_body(page: &str, s: &Snap, q: &QueryForm, ops: &ops::Ops, set_ops: &StateSetter<ops::Ops>) -> VNode {
    match page {
        "query" => query_page(q),
        "migrate" => ops::migrate(set_ops, ops),
        "backup" => ops::backup(set_ops, ops),
        "registry" => ops::registry(set_ops, ops),
        "parallel" => ops::parallel(set_ops, ops),
        "federation" => ops::federation(set_ops, ops),
        "cluster" => ops::cluster(set_ops, ops),
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
    // フックの数と順序を変えないよう、クエリ画面の状態も常に作る
    let (mode, set_mode) = use_state(|| "sql".to_string());
    let (text, set_text) = use_state(String::new);
    let (auto, set_auto) = use_state(|| false);
    let (out, set_out) = use_state(String::new);
    let q = QueryForm { mode, text, auto, out, set_mode, set_text, set_auto, set_out };
    let (opsv, set_ops) = use_state(ops::Ops::default);
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
            nav_button("query", "クエリ", page == "query", set_page.clone()),
            node("div", "group", vec![t("管理")]),
            nav_button("migrate", "お引越し", page == "migrate", set_page.clone()),
            nav_button("backup", "バックアップ", page == "backup", set_page.clone()),
            nav_button("registry", "レジストリ", page == "registry", set_page.clone()),
            nav_button("parallel", "並列化", page == "parallel", set_page.clone()),
            nav_button("federation", "統合", page == "federation", set_page.clone()),
            nav_button("cluster", "クラスタ", page == "cluster", set_page.clone()),
            node("div", "status", vec![node("span", dot, vec![]), VNode::element("span").attr("id", "status").child(t(status)).build()]),
        ],
    );
    node("div", "app", vec![side, node("main", "main", vec![page_body(&page, &snap, &q, &opsv, &set_ops)])])
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
        let html_of = |p: &'static str| {
            let s = s.clone();
            let h = reaxt::react::hooks::run_component(1, move || {
                let (mode, set_mode) = use_state(|| "sql".to_string());
                let (text, set_text) = use_state(String::new);
                let (auto, set_auto) = use_state(|| false);
                let (out, set_out) = use_state(|| "<i>out</i>".to_string());
                let q = QueryForm { mode, text, auto, out, set_mode, set_text, set_auto, set_out };
                let (o, so) = use_state(ops::Ops::default);
                reaxt::ssr::render(&page_body(p, &s, &q, &o, &so))
            });
            reaxt::react::hooks::drop_instance(1);
            h
        };
        for p in ["dashboard", "commits", "branches", "query", "migrate", "backup", "registry", "parallel", "federation", "cluster"] {
            let html = html_of(p);
            assert!(!html.contains("<b>x") && !html.contains("<i>out"), "{p}: 値はエスケープされる");
        }
        let q = html_of("query");
        assert!(q.contains("data-rx-submit=\"q-run\"") && q.contains("data-rx-input=\"q-text\"") && q.contains("data-rx-input=\"q-auto\""));
        assert!(!q.contains("onsubmit") && !q.contains("oninput"), "インラインJSは使わない");
    }

    #[test]
    fn exec_sql_mutation_escapes_the_sql() {
        let m = exec_sql_mutation("INSERT INTO t VALUES ('a\"b')\n", true).unwrap();
        assert!(m.starts_with("mutation { execSql(sql: \"INSERT INTO t VALUES ('a\\\"b')\\n\", autoCommit: true"), "{m}");
    }
}
