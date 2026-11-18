//! WunderGraph Cosmo(GraphQL連合)向けのサブグラフ入口。
//! Reaxtが公開するConnectメソッドを、GraphQLのMutationフィールド(`pkg_Service_Method(input: JSON): JSON`)として見せる。
//! Cosmo Routerはこのサブグラフを他のサブグラフと合成し、`POST /_reaxt/graphql`へ問い合わせる。
//! 対応する問い合わせの形: ルートのフィールドを並べ、引数は `input: $変数` だけ(リテラルは未対応)。
//! 戻り値はJSONスカラーなので下位の選択は不要。認証・`x-reaxt`ヘッダーはRouter側のヘッダー伝播で付ける。
use reaxt::app::{App, Gateway, Request, Response};
use rust_json::{parse_light_secure, quote_json, LightLimits, LightValue};
use std::sync::Arc;

pub const PATH: &str = "/_reaxt/graphql";

/// `pkg.v1.Service` + `Method` → GraphQLのフィールド名。
pub fn field_name(service: &str, method: &str) -> String {
    let mut s: String = service.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    s.push('_');
    s.push_str(method);
    s
}

/// 連合(Federation v2)対応のSDL。
pub fn sdl(manifest_paths: &[String]) -> String {
    let mut o = String::from(
        "extend schema @link(url: \"https://specs.apollo.dev/federation/v2.0\", import: [\"@shareable\"])\n\nscalar JSON\n\ntype Query {\n  reaxt_ready: Boolean! @shareable\n}\n\ntype Mutation {\n",
    );
    for p in manifest_paths {
        if let Some((s, m)) = p.strip_prefix('/').and_then(|x| x.rsplit_once('/')) {
            o.push_str(&format!("  {}(input: JSON): JSON\n", field_name(s, m)));
        }
    }
    o.push_str("}\n");
    o
}

struct Root {
    alias: String,
    field: String,
    var: Option<String>,
}

/// 問い合わせ文字列から、ルートのフィールド(別名・名前・`input: $v`)を取り出す。
fn parse_roots(q: &str) -> Result<Vec<Root>, String> {
    let b: Vec<char> = q.chars().collect();
    let mut i = 0;
    // コメントを除く
    let mut clean: Vec<char> = Vec::with_capacity(b.len());
    while i < b.len() {
        if b[i] == '#' {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
        } else {
            clean.push(b[i]);
            i += 1;
        }
    }
    let b = clean;
    // 最初の`{`(操作の本体)まで進む。変数定義の`(...)`は読み飛ばす
    let mut i = 0;
    let mut paren = 0;
    while i < b.len() {
        match b[i] {
            '(' => paren += 1,
            ')' => paren -= 1,
            '{' if paren == 0 => break,
            _ => {}
        }
        i += 1;
    }
    if i >= b.len() {
        return Err("問い合わせに { がありません".into());
    }
    i += 1;
    let mut roots = Vec::new();
    let ident = |i: &mut usize, b: &[char]| -> String {
        while *i < b.len() && (b[*i].is_whitespace() || b[*i] == ',') {
            *i += 1;
        }
        let st = *i;
        while *i < b.len() && (b[*i].is_ascii_alphanumeric() || b[*i] == '_') {
            *i += 1;
        }
        b[st..*i].iter().collect()
    };
    loop {
        while i < b.len() && (b[i].is_whitespace() || b[i] == ',') {
            i += 1;
        }
        if i >= b.len() {
            return Err("問い合わせが閉じていません".into());
        }
        if b[i] == '}' {
            break;
        }
        let first = ident(&mut i, &b);
        if first.is_empty() {
            return Err(format!("解釈できない文字 {:?}", b[i]));
        }
        while i < b.len() && b[i].is_whitespace() {
            i += 1;
        }
        let (alias, field) = if i < b.len() && b[i] == ':' {
            i += 1;
            let f = ident(&mut i, &b);
            (first, f)
        } else {
            (first.clone(), first)
        };
        while i < b.len() && b[i].is_whitespace() {
            i += 1;
        }
        let mut var = None;
        if i < b.len() && b[i] == '(' {
            let st = i + 1;
            let mut d = 0;
            while i < b.len() {
                match b[i] {
                    '(' => d += 1,
                    ')' => {
                        d -= 1;
                        if d == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            let args: String = b[st..i.min(b.len())].iter().collect();
            i += 1;
            let args = args.trim();
            if !args.is_empty() {
                let rest = args.strip_prefix("input").map(str::trim_start).and_then(|r| r.strip_prefix(':')).map(str::trim);
                match rest.and_then(|r| r.strip_prefix('$')) {
                    Some(name) if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !name.is_empty() => var = Some(name.to_string()),
                    _ => return Err("引数は input: $変数 の形だけ対応です(リテラル不可)".into()),
                }
            }
        }
        // 下位の選択(あっても無視)
        while i < b.len() && b[i].is_whitespace() {
            i += 1;
        }
        if i < b.len() && b[i] == '{' {
            let mut d = 0;
            while i < b.len() {
                match b[i] {
                    '{' => d += 1,
                    '}' => {
                        d -= 1;
                        if d == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
        roots.push(Root { alias, field, var });
    }
    if roots.is_empty() {
        return Err("ルートのフィールドがありません".into());
    }
    Ok(roots)
}

fn gql_error(msg: &str, code: &str, status: u16) -> String {
    format!("{{\"message\":{},\"extensions\":{{\"code\":{},\"status\":{status}}}}}", quote_json(msg), quote_json(code))
}

/// 1回のGraphQL要求(本文JSON)を処理して、GraphQL応答のJSONを返す。
pub fn execute(body: &str, paths: &[String], call: &dyn Fn(&str, &str) -> (u16, String)) -> (u16, String) {
    let v = match parse_light_secure(body, &LightLimits::small()) {
        Ok(v) => v,
        Err(e) => return (400, format!("{{\"errors\":[{}]}}", gql_error(&e.to_string(), "invalid_argument", 400))),
    };
    let query = v.get("query").and_then(|q| q.as_str()).unwrap_or("");
    let vars = v.get("variables");
    // 連合のSDL問い合わせ
    if query.contains("_service") {
        return (200, format!("{{\"data\":{{\"_service\":{{\"sdl\":{}}}}}}}", quote_json(&sdl(paths))));
    }
    if query.contains("reaxt_ready") && !query.contains("mutation") {
        return (200, "{\"data\":{\"reaxt_ready\":true}}".into());
    }
    let roots = match parse_roots(query) {
        Ok(r) => r,
        Err(e) => return (200, format!("{{\"data\":null,\"errors\":[{}]}}", gql_error(&e, "invalid_argument", 400))),
    };
    let mut data = Vec::new();
    let mut errors = Vec::new();
    for r in roots {
        if r.field == "__typename" {
            data.push(format!("{}:\"Mutation\"", quote_json(&r.alias)));
            continue;
        }
        let target = paths.iter().find(|p| p.strip_prefix('/').and_then(|x| x.rsplit_once('/')).map_or(false, |(s, m)| field_name(s, m) == r.field));
        let Some(path) = target else {
            errors.push(gql_error(&format!("未知のフィールド {}", r.field), "not_found", 404));
            data.push(format!("{}:null", quote_json(&r.alias)));
            continue;
        };
        let input = match &r.var {
            Some(n) => vars.and_then(|m| m.get(n)).map(LightValue::to_json).unwrap_or_else(|| "null".into()),
            None => "null".into(),
        };
        let (status, out) = call(path, &input);
        if status == 200 {
            data.push(format!("{}:{}", quote_json(&r.alias), out));
        } else {
            let (code, msg) = match parse_light_secure(&out, &LightLimits::small()) {
                Ok(e) => (e.get("code").and_then(|x| x.as_str()).unwrap_or("unknown").to_string(), e.get("message").and_then(|x| x.as_str()).unwrap_or("").to_string()),
                Err(_) => ("unknown".into(), out),
            };
            errors.push(gql_error(&msg, &code, status));
            data.push(format!("{}:null", quote_json(&r.alias)));
        }
    }
    let errs = if errors.is_empty() { String::new() } else { format!(",\"errors\":[{}]", errors.join(",")) };
    (200, format!("{{\"data\":{{{}}}{errs}}}", data.join(",")))
}

/// `App`に`POST /_reaxt/graphql`(Cosmo向けサブグラフ)を足す。メソッド一覧は呼び出し時点の登録分。
pub fn install(app: App) -> App {
    let paths = app.method_paths();
    let g: Gateway = Arc::new(move |req: &Request, call: &dyn Fn(&str, &str) -> (u16, String)| {
        let (status, body) = execute(req.body, &paths, call);
        Response { status, content_type: "application/json; charset=utf-8", headers: vec![("cache-control".into(), "no-store".into())], bytes: None, chunks: None, body }
    });
    app.gateway(PATH, g)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> Vec<String> {
        vec!["/demo.v1.Counter/Add".into(), "/demo.v1.Counter/Fail".into()]
    }
    fn fake(path: &str, body: &str) -> (u16, String) {
        match path {
            "/demo.v1.Counter/Add" => (200, format!("{{\"echo\":{body}}}")),
            _ => (403, "{\"code\":\"permission_denied\",\"message\":\"no\"}".into()),
        }
    }

    #[test]
    fn sdl_lists_fields() {
        let s = sdl(&paths());
        assert!(s.contains("demo_v1_Counter_Add(input: JSON): JSON"));
        assert!(s.contains("scalar JSON"));
    }

    #[test]
    fn executes_with_variables_alias_and_errors() {
        let q = "{\"query\":\"mutation M($a: JSON, $b: JSON) { x: demo_v1_Counter_Add(input: $a) demo_v1_Counter_Fail(input: $b) { y } }\",\"variables\":{\"a\":{\"by\":3},\"b\":null}}";
        let (st, out) = execute(q, &paths(), &fake);
        assert_eq!(st, 200);
        assert!(out.contains("\"x\":{\"echo\":{\"by\":3}}"), "{out}");
        assert!(out.contains("\"demo_v1_Counter_Fail\":null"), "{out}");
        assert!(out.contains("permission_denied"), "{out}");
    }

    #[test]
    fn rejects_literals_unknown_fields_and_bad_json() {
        let (_, out) = execute("{\"query\":\"mutation { demo_v1_Counter_Add(input: {by: 1}) }\"}", &paths(), &fake);
        assert!(out.contains("リテラル不可"), "{out}");
        let (_, out) = execute("{\"query\":\"mutation { nope }\"}", &paths(), &fake);
        assert!(out.contains("not_found"), "{out}");
        let (st, _) = execute("{bad", &paths(), &fake);
        assert_eq!(st, 400);
    }

    #[test]
    fn answers_federation_sdl() {
        let (_, out) = execute("{\"query\":\"query { _service { sdl } }\"}", &paths(), &fake);
        assert!(out.contains("_service") && out.contains("scalar JSON"), "{out}");
    }
}
