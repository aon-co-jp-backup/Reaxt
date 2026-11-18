//! 残り6画面(Migrate/Backup/Registry/Parallel/Federation/Cluster)。
//! 状態は1つの`Ops`にまとめ(フックの数を固定)、結果はJSONを汎用の表/キー値で描く。
use crate::{node, t};
use reaxt::hydrate::{on_click, on_input};
use reaxt::react::{StateSetter, VNode};
use reaxt_actions::{obj_json, quote_json, str_field, Action, Wire};
use rust_json::LightValue;
use std::collections::BTreeMap;

pub struct AdmIn {
    pub method: String,
    pub body: String,
}
/// 管理操作(Connect)の中継。トークンはサーバー側が付ける。
impl Wire for AdmIn {
    fn to_wire(&self) -> String {
        obj_json(&[("method", quote_json(&self.method)), ("body", quote_json(&self.body))])
    }
    fn from_wire(v: &LightValue) -> Result<Self, String> {
        Ok(AdmIn { method: str_field(v, "method")?, body: str_field(v, "body")? })
    }
}
pub const ADM: Action<AdmIn, crate::GqlOut> = Action::new("aruaru_admin.v1.Proxy", "Admin");

#[derive(Clone, Default)]
pub struct Ops {
    pub f: BTreeMap<String, String>,
    pub outs: BTreeMap<String, String>,
}
impl Ops {
    fn g(&self, k: &str) -> String {
        self.f.get(k).cloned().unwrap_or_default()
    }
    fn gd(&self, k: &str, d: &str) -> String {
        self.f.get(k).cloned().unwrap_or_else(|| d.to_string())
    }
}

pub fn q(s: &str) -> String {
    quote_json(s)
}

/// 画面ごとの実行要求(送る先と本文)。ネイティブでもテストできる純粋関数で組み立てる。
pub enum Req {
    Adm(&'static str, String),
    Gql(&'static str),
}
pub fn build(op: &str, o: &Ops) -> Option<Req> {
    Some(match op {
        "mig-test" => Req::Adm("TestSourceConnection", format!("{{\"source\":{},\"uri\":{}}}", q(&o.gd("m-source", "postgres")), q(&o.g("m-uri")))),
        "mig-run" => Req::Adm(
            "RunMigration",
            format!("{{\"input\":{{\"source\":{},\"sourceUri\":{},\"commitMessage\":{},\"includeTables\":null}}}}", q(&o.gd("m-source", "postgres")), q(&o.g("m-uri")), q(&o.gd("m-msg", "Migration import"))),
        ),
        "mig-move" => Req::Adm(
            "RunMigration",
            format!("{{\"input\":{{\"source\":\"aruaru\",\"sourceUri\":{},\"commitMessage\":\"Instance migration\",\"includeTables\":null}}}}", q(&o.g("m-target"))),
        ),
        "bk-list" => Req::Gql("query { backups { id createdAt branch commitId kind sizeMb path status } }"),
        "bk-create" => Req::Adm(
            "CreateBackup",
            format!("{{\"config\":{{\"branch\":{},\"kind\":{}}}}}", if o.g("b-branch").is_empty() { "null".to_string() } else { q(&o.g("b-branch")) }, q(&o.gd("b-kind", "full"))),
        ),
        "bk-restore" => Req::Adm("RestoreBackup", format!("{{\"input\":{{\"backupId\":{},\"targetBranch\":null}}}}", q(&o.g("b-id")))),
        "reg-list" => Req::Gql("query { registry { id name category wire status rank score updatedAt } }"),
        "reg-sum" => Req::Gql("query { registrySummary { total connectable ga beta pgCompatible planned } }"),
        "reg-crawl" => Req::Adm("CrawlRegistry", "{}".to_string()),
        "par-cfg" => Req::Gql("query { parallelConfig { enabled maxWorkers chunkSize strategy } }"),
        "par-jobs" => Req::Gql("query { parallelJobs { jobId sql status workers elapsedMs rowsProcessed startedAt } }"),
        "par-explain" => Req::Adm("ExplainDistributed", format!("{{\"sql\":{}}}", q(&o.gd("p-sql", "SELECT 1")))),
        "fed-list" => Req::Gql("query { federatedSources { name kind uri status tables } }"),
        "fed-add" => Req::Adm("RegisterFederatedSource", format!("{{\"input\":{{\"name\":{},\"kind\":{},\"uri\":{}}}}}", q(&o.g("f-name")), q(&o.gd("f-kind", "postgres")), q(&o.g("f-uri")))),
        "fed-test" => Req::Adm("TestSourceConnection", format!("{{\"source\":{},\"uri\":{}}}", q(&o.gd("f-kind", "postgres")), q(&o.g("f-uri")))),
        "fed-drop" => Req::Adm("DropFederatedSource", format!("{{\"name\":{}}}", q(&o.g("f-name")))),
        "fed-query" => Req::Adm("FederatedQuery", format!("{{\"sql\":{}}}", q(&o.g("f-sql")))),
        "cl-status" => Req::Gql("query { clusterStatus { stats { totalNodes healthyNodes totalRanges totalRows tableCount replicationFactor underReplicated } nodes { nodeId addr role alive commitIndex appliedIndex ranges diskUsedGb } ranges { rangeId startKey endKey leaderNode replicas sizeMb } } }"),
        "cl-add" => Req::Adm("ClusterNodeOp", format!("{{\"input\":{{\"action\":\"add\",\"nodeId\":{},\"addr\":{}}}}}", o.gd("c-id", "4").parse::<u64>().unwrap_or(0), q(&o.g("c-addr")))),
        "cl-rebalance" => Req::Adm("RebalanceCluster", "{}".to_string()),
        _ => return None,
    })
}

#[cfg(target_arch = "wasm32")]
pub fn run(op: &'static str, o: &Ops, set: StateSetter<Ops>) {
    use crate::{GqlIn, GQL};
    use reaxt_actions::web::WebClient;
    let Some(req) = build(op, o) else { return };
    let put = move |s: String| {
        set.update(|old| {
            let mut n = old.clone();
            n.outs.insert(op.to_string(), s.clone());
            n
        })
    };
    let fin = move |r: Result<crate::GqlOut, reaxt_actions::ActionError>| {
        put(match r {
            Ok(v) => v.body,
            Err(reaxt_actions::ActionError::Server { message, .. }) => format!("{{\"エラー\":{}}}", q(&message)),
            Err(e) => format!("{{\"エラー\":{}}}", q(&format!("{e:?}"))),
        })
    };
    match req {
        Req::Adm(m, b) => reaxt_actions::call(&WebClient, &ADM, &AdmIn { method: m.into(), body: b }, fin),
        Req::Gql(g) => reaxt_actions::call(&WebClient, &GQL, &GqlIn { query: g.into() }, fin),
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub fn run(_: &'static str, _: &Ops, _: StateSetter<Ops>) {}

fn set_field(set: &StateSetter<Ops>, key: &'static str, val: String) {
    set.update(move |old| {
        let mut n = old.clone();
        n.f.insert(key.to_string(), val.clone());
        n
    })
}

fn field(set: &StateSetter<Ops>, o: &Ops, key: &'static str, ph: &str, area: bool) -> VNode {
    let s = set.clone();
    let (k, v) = on_input(key, move |val| set_field(&s, key, val));
    let tag = if area { "textarea" } else { "input" };
    let mut b = VNode::element(tag).attr("id", key).attr("placeholder", ph).attr(k, v);
    if area {
        b = b.attr("rows", "4").child(t(o.g(key)));
    } else {
        b = b.attr("type", "text").attr("class", "txt").attr("value", o.g(key));
    }
    b.build()
}

fn pick(set: &StateSetter<Ops>, o: &Ops, key: &'static str, val: &'static str, default: &str) -> VNode {
    let s = set.clone();
    let (k, v) = on_click(&format!("pick-{key}-{val}"), move || set_field(&s, key, val.to_string()));
    let on = o.gd(key, default) == val;
    VNode::element("button").attr("type", "button").attr("id", format!("pick-{key}-{val}")).attr("class", if on { "tab active" } else { "tab" }).attr(k, v).child(t(val)).build()
}

fn btn(set: &StateSetter<Ops>, o: &Ops, op: &'static str, label: &str) -> VNode {
    let (s, oo) = (set.clone(), o.clone());
    let (k, v) = on_click(&format!("op-{op}"), move || run(op, &oo, s.clone()));
    VNode::element("button").attr("type", "button").attr("id", format!("op-{op}")).attr("class", "run").attr(k, v).child(t(label)).build()
}

/// JSON文字列を汎用表示にする(オブジェクトの配列=表、オブジェクト=キー値カード、他=整形文字列)。
pub fn json_view(raw: &str) -> VNode {
    let v = match rust_json::parse_light_secure(raw, &rust_json::LightLimits::small()) {
        Ok(v) => v,
        Err(_) => return VNode::element("pre").child(t(raw.to_string())).build(),
    };
    let v = v.get("data").cloned().unwrap_or(v);
    // GraphQLの data は {フィールド名: 値}。1フィールドなら値だけ取り出す
    let v = match v.as_object() {
        Some([(_, only)]) => only.clone(),
        _ => v,
    };
    render(&v)
}

fn cell(v: &LightValue) -> String {
    match v {
        LightValue::Str(s) => s.clone(),
        LightValue::Null => String::new(),
        other => other.to_json(),
    }
}

fn render(v: &LightValue) -> VNode {
    if let Some(arr) = v.as_array() {
        if arr.is_empty() {
            return node("p", "dim", vec![t("(0件)")]);
        }
        if arr.iter().all(|x| x.as_object().is_some()) {
            let cols: Vec<String> = arr[0].as_object().map(|m| m.iter().map(|(k, _)| k.clone()).collect()).unwrap_or_default();
            let mut rows = vec![node("tr", "", cols.iter().map(|c| node("th", "", vec![t(c.clone())])).collect())];
            for r in arr {
                rows.push(node("tr", "", cols.iter().map(|c| node("td", "mono", vec![t(r.get(c).map(cell).unwrap_or_default())])).collect()));
            }
            return node("table", "tbl", rows);
        }
    }
    if let Some(m) = v.as_object() {
        let mut kids = vec![];
        let mut cards = vec![];
        for (k, x) in m {
            if x.as_object().is_some() || x.as_array().is_some() {
                kids.push(node("h3", "", vec![t(k.clone())]));
                kids.push(render(x));
            } else {
                cards.push(node("div", "card", vec![node("div", "label", vec![t(k.clone())]), node("div", "value", vec![t(cell(x))])]));
            }
        }
        let mut all = vec![node("div", "cards", cards)];
        all.extend(kids);
        return node("div", "", all);
    }
    VNode::element("pre").child(t(v.to_json())).build()
}

fn out(o: &Ops, op: &str) -> Vec<VNode> {
    match o.outs.get(op) {
        Some(s) => vec![node("div", "res", vec![json_view(s)])],
        None => vec![],
    }
}

fn sec(title: &str, kids: Vec<VNode>) -> VNode {
    let mut v = vec![node("h3", "", vec![t(title)])];
    v.extend(kids);
    node("section", "", v)
}
fn row(kids: Vec<VNode>) -> VNode {
    node("div", "row", kids)
}
fn page(title: &str, secs: Vec<VNode>) -> VNode {
    let mut v = vec![node("h2", "", vec![t(title)])];
    v.extend(secs);
    node("section", "", v)
}

pub fn migrate(set: &StateSetter<Ops>, o: &Ops) -> VNode {
    let mut a = vec![
        row(["postgres", "cockroach", "snowflake", "mysql", "csv", "parquet"].iter().map(|s| pick(set, o, "m-source", s, "postgres")).collect()),
        field(set, o, "m-uri", "接続URI / ファイルパス", false),
        field(set, o, "m-msg", "コミットメッセージ", false),
        row(vec![btn(set, o, "mig-test", "接続テスト"), btn(set, o, "mig-run", "取り込み開始")]),
    ];
    a.extend(out(o, "mig-test"));
    a.extend(out(o, "mig-run"));
    let mut b = vec![field(set, o, "m-target", "移植先クラスタ URI (aruaru://host:5432/main)", false), row(vec![btn(set, o, "mig-move", "まるごと移植")])];
    b.extend(out(o, "mig-move"));
    page("お引越し(移行・移植)", vec![sec("取り込み: 他DB → aruaru", a), sec("まるごと移植: aruaru → 別クラスタ", b)])
}

pub fn backup(set: &StateSetter<Ops>, o: &Ops) -> VNode {
    let mut a = vec![row(vec![btn(set, o, "bk-list", "一覧を取得")])];
    a.extend(out(o, "bk-list"));
    let mut b = vec![
        row(vec![pick(set, o, "b-kind", "full", "full"), pick(set, o, "b-kind", "incremental", "full")]),
        field(set, o, "b-branch", "ブランチ(空=現在)", false),
        row(vec![btn(set, o, "bk-create", "バックアップ作成")]),
    ];
    b.extend(out(o, "bk-create"));
    let mut c = vec![field(set, o, "b-id", "バックアップID", false), row(vec![btn(set, o, "bk-restore", "復元")])];
    c.extend(out(o, "bk-restore"));
    page("バックアップ", vec![sec("一覧", a), sec("作成", b), sec("復元", c)])
}

pub fn registry(set: &StateSetter<Ops>, o: &Ops) -> VNode {
    let mut a = vec![row(vec![btn(set, o, "reg-sum", "集計"), btn(set, o, "reg-list", "一覧"), btn(set, o, "reg-crawl", "再クロール")])];
    for op in ["reg-sum", "reg-list", "reg-crawl"] {
        a.extend(out(o, op));
    }
    page("対応DBレジストリ", vec![node("div", "", a)])
}

pub fn parallel(set: &StateSetter<Ops>, o: &Ops) -> VNode {
    let mut a = vec![row(vec![btn(set, o, "par-cfg", "設定を取得"), btn(set, o, "par-jobs", "ジョブ一覧")]), node("p", "dim", vec![t("設定の変更は aruaru.yaml の query.parallel で行います(画面からは変更不可)。")])];
    a.extend(out(o, "par-cfg"));
    a.extend(out(o, "par-jobs"));
    let mut b = vec![field(set, o, "p-sql", "SELECT region, SUM(amount) FROM orders GROUP BY region", true), row(vec![btn(set, o, "par-explain", "EXPLAIN (分散)")])];
    b.extend(out(o, "par-explain"));
    page("分散並列化", vec![sec("設定・ジョブ", a), sec("実行プラン", b)])
}

pub fn federation(set: &StateSetter<Ops>, o: &Ops) -> VNode {
    let mut a = vec![row(vec![btn(set, o, "fed-list", "ソース一覧")])];
    a.extend(out(o, "fed-list"));
    let mut b = vec![
        row(["aruaru", "postgres", "cockroach", "snowflake", "mysql"].iter().map(|s| pick(set, o, "f-kind", s, "postgres")).collect()),
        field(set, o, "f-name", "ソース名", false),
        field(set, o, "f-uri", "URI", false),
        row(vec![btn(set, o, "fed-test", "接続テスト"), btn(set, o, "fed-add", "登録"), btn(set, o, "fed-drop", "名前で削除")]),
    ];
    for op in ["fed-test", "fed-add", "fed-drop"] {
        b.extend(out(o, op));
    }
    let mut c = vec![field(set, o, "f-sql", "SELECT * FROM local.users LIMIT 10", true), row(vec![btn(set, o, "fed-query", "横断クエリ実行")])];
    c.extend(out(o, "fed-query"));
    page("分散DB統合(フェデレーション)", vec![sec("統合ソース", a), sec("登録", b), sec("横断クエリ", c)])
}

pub fn cluster(set: &StateSetter<Ops>, o: &Ops) -> VNode {
    let mut a = vec![row(vec![btn(set, o, "cl-status", "状態を取得")])];
    a.extend(out(o, "cl-status"));
    let mut b = vec![field(set, o, "c-id", "新ノードID (4)", false), field(set, o, "c-addr", "host:port", false), row(vec![btn(set, o, "cl-add", "ノード追加"), btn(set, o, "cl-rebalance", "リバランス")])];
    b.extend(out(o, "cl-add"));
    b.extend(out(o, "cl-rebalance"));
    page("クラスタ管理", vec![sec("状態", a), sec("操作", b)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_requests_with_escaping() {
        let mut o = Ops::default();
        o.f.insert("m-uri".into(), "pg://u:p\"x@h/db".into());
        match build("mig-run", &o) {
            Some(Req::Adm("RunMigration", b)) => assert!(b.contains("\"sourceUri\":\"pg://u:p\\\"x@h/db\"") && b.contains("\"source\":\"postgres\""), "{b}"),
            _ => panic!(),
        }
        assert!(build("nope", &o).is_none());
        for op in ["bk-list", "reg-list", "fed-list", "cl-status", "par-jobs"] {
            assert!(matches!(build(op, &o), Some(Req::Gql(_))));
        }
    }

    #[test]
    fn json_view_renders_tables_and_escapes() {
        let h = reaxt::ssr::render(&json_view(r#"{"data":{"backups":[{"id":"<b>1","kind":"full"}]}}"#));
        assert!(h.contains("<table") && !h.contains("<b>1") && h.contains("&lt;b&gt;1"), "{h}");
        let h = reaxt::ssr::render(&json_view(r#"{"ok":true,"message":"x"}"#));
        assert!(h.contains("message"), "{h}");
        let h = reaxt::ssr::render(&json_view("not json"));
        assert!(h.contains("not json"));
    }
}
