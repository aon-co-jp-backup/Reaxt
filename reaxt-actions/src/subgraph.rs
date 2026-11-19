//! WunderGraph Cosmo(GraphQL連合)向けのサブグラフ入口。
//! Reaxtが公開するConnectメソッドを、GraphQLのMutationフィールド(`pkg_Service_Method(input: JSON): JSON`)として見せる。
//! Cosmo Routerはこのサブグラフを他のサブグラフと合成し、`POST /_reaxt/graphql`へ問い合わせる。
//! 対応する問い合わせの形: ルートのフィールドを並べ、引数は `input: $変数` か `input: リテラル`(オブジェクト・配列・文字列・数値・真偽・null・列挙・入れ子の`$変数`)。
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

/// 読み取り用のメソッドか(名前が `Get` / `List` / `Read` で始まる)。GraphQLでは Query、それ以外は Mutation として見せる。
pub fn is_read_method(method: &str) -> bool {
    ["Get", "List", "Read"].iter().any(|p| method.strip_prefix(p).is_some_and(|r| r.is_empty() || r.starts_with(|c: char| c.is_ascii_uppercase())))
}

/// `(サービス, メソッド)` に分けたパス一覧(読み取り用, 変更用)。
fn split_paths(manifest_paths: &[String]) -> (Vec<(&str, &str)>, Vec<(&str, &str)>) {
    let (mut reads, mut writes) = (Vec::new(), Vec::new());
    for p in manifest_paths {
        if let Some((s, m)) = p.strip_prefix('/').and_then(|x| x.rsplit_once('/')) {
            if is_read_method(m) {
                reads.push((s, m));
            } else {
                writes.push((s, m));
            }
        }
    }
    (reads, writes)
}

/// 連合(Federation v2)対応のSDL。`Get*`/`List*`/`Read*` は Query、それ以外は Mutation。
pub fn sdl(manifest_paths: &[String]) -> String {
    let (reads, writes) = split_paths(manifest_paths);
    let mut o = String::from(
        "extend schema @link(url: \"https://specs.apollo.dev/federation/v2.0\", import: [\"@shareable\"])\n\nscalar JSON\n\ntype Query {\n  reaxt_ready: Boolean! @shareable\n",
    );
    for (s, m) in &reads {
        o.push_str(&format!("  {}(input: JSON): JSON\n", field_name(s, m)));
    }
    o.push_str("}\n");
    if !writes.is_empty() {
        o.push_str("\ntype Mutation {\n");
        for (s, m) in &writes {
            o.push_str(&format!("  {}(input: JSON): JSON\n", field_name(s, m)));
        }
        o.push_str("}\n");
    }
    o
}

struct Root {
    alias: String,
    field: String,
    /// `input:` の右辺(GraphQLの値の文字列)。実行時にJSONへ変換する。
    input: Option<String>,
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
        let mut input_src = None;
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
                match rest {
                    Some(v) if !v.is_empty() => input_src = Some(v.to_string()),
                    _ => return Err("引数は input: 値 の形だけ対応です".into()),
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
        roots.push(Root { alias, field, input: input_src });
    }
    if roots.is_empty() {
        return Err("ルートのフィールドがありません".into());
    }
    Ok(roots)
}

/// GraphQLの値(リテラル・`$変数`)をJSONの文字列へ変換する。列挙値は文字列、オブジェクトのキーは引用符つきにする。
/// 変数が無い・未定義なら `null`。ブロック文字列(`"""`)は未対応。
fn gql_value_to_json(src: &str, vars: Option<&LightValue>) -> Result<String, String> {
    struct P<'a> {
        b: Vec<char>,
        i: usize,
        vars: Option<&'a LightValue>,
    }
    impl P<'_> {
        fn ws(&mut self) {
            while self.i < self.b.len() && (self.b[self.i].is_whitespace() || self.b[self.i] == ',') {
                self.i += 1;
            }
        }
        fn name(&mut self) -> String {
            let st = self.i;
            while self.i < self.b.len() && (self.b[self.i].is_ascii_alphanumeric() || matches!(self.b[self.i], '_' | '.' | '+' | '-')) {
                self.i += 1;
            }
            self.b[st..self.i].iter().collect()
        }
        fn value(&mut self, depth: usize) -> Result<String, String> {
            if depth > 32 {
                return Err("引数の入れ子が深すぎます".into());
            }
            self.ws();
            let Some(&c) = self.b.get(self.i) else { return Err("値がありません".into()) };
            match c {
                '$' => {
                    self.i += 1;
                    let n = self.name();
                    Ok(self.vars.and_then(|m| m.get(&n)).map(LightValue::to_json).unwrap_or_else(|| "null".into()))
                }
                '"' => {
                    if self.b[self.i..].starts_with(&['"', '"', '"']) {
                        return Err("ブロック文字列(\"\"\")は未対応です".into());
                    }
                    self.i += 1;
                    let mut o = String::from("\"");
                    while self.i < self.b.len() {
                        let ch = self.b[self.i];
                        self.i += 1;
                        match ch {
                            '\\' => {
                                o.push('\\');
                                if let Some(&n) = self.b.get(self.i) {
                                    o.push(n);
                                    self.i += 1;
                                }
                            }
                            '"' => {
                                o.push('"');
                                return Ok(o);
                            }
                            _ => o.push(ch),
                        }
                    }
                    Err("文字列が閉じていません".into())
                }
                '[' => {
                    self.i += 1;
                    let mut items = Vec::new();
                    loop {
                        self.ws();
                        match self.b.get(self.i) {
                            Some(']') => {
                                self.i += 1;
                                return Ok(format!("[{}]", items.join(",")));
                            }
                            None => return Err("配列が閉じていません".into()),
                            _ => items.push(self.value(depth + 1)?),
                        }
                    }
                }
                '{' => {
                    self.i += 1;
                    let mut items = Vec::new();
                    loop {
                        self.ws();
                        match self.b.get(self.i) {
                            Some('}') => {
                                self.i += 1;
                                return Ok(format!("{{{}}}", items.join(",")));
                            }
                            None => return Err("オブジェクトが閉じていません".into()),
                            _ => {
                                let k = self.name();
                                if k.is_empty() {
                                    return Err("オブジェクトのキーが読めません".into());
                                }
                                self.ws();
                                if self.b.get(self.i) != Some(&':') {
                                    return Err("オブジェクトに : がありません".into());
                                }
                                self.i += 1;
                                let v = self.value(depth + 1)?;
                                items.push(format!("{}:{v}", quote_json(&k)));
                            }
                        }
                    }
                }
                _ => {
                    let t = self.name();
                    if t.is_empty() {
                        return Err(format!("解釈できない文字 {c:?}"));
                    }
                    if matches!(t.as_str(), "null" | "true" | "false") {
                        Ok(t)
                    } else if t.chars().next().is_some_and(|x| x.is_ascii_digit() || x == '-') {
                        t.parse::<f64>().map(|_| t.clone()).map_err(|_| format!("数値が不正です: {t}"))
                    } else {
                        Ok(quote_json(&t))
                    }
                }
            }
        }
    }
    let mut p = P { b: src.chars().collect(), i: 0, vars };
    let out = p.value(0)?;
    p.ws();
    if p.i < p.b.len() {
        return Err("引数の後ろに余分な文字があります".into());
    }
    Ok(out)
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
    // open-cosmo のゲートウェイは、ルートのフィールド名をこの問い合わせで探す(`remote_subgraphs.rs`)。
    // 型や引数までは返さない最小の応答(`queryType`/`mutationType` の `fields { name }` だけ)。
    if query.contains("__schema") {
        let (reads, writes) = split_paths(paths);
        let list = |v: &[(&str, &str)]| v.iter().map(|(s, m)| format!("{{\"name\":{}}}", quote_json(&field_name(s, m)))).collect::<Vec<_>>().join(",");
        let mut q = String::from("{\"name\":\"reaxt_ready\"}");
        if !reads.is_empty() {
            q.push(',');
            q.push_str(&list(&reads));
        }
        return (
            200,
            format!(
                "{{\"data\":{{\"__schema\":{{\"queryType\":{{\"fields\":[{q}]}},\"mutationType\":{{\"fields\":[{}]}}}}}}}}",
                list(&writes)
            ),
        );
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
        if r.field == "reaxt_ready" {
            data.push(format!("{}:true", quote_json(&r.alias)));
            continue;
        }
        let target = paths.iter().find(|p| p.strip_prefix('/').and_then(|x| x.rsplit_once('/')).map_or(false, |(s, m)| field_name(s, m) == r.field));
        let Some(path) = target else {
            errors.push(gql_error(&format!("未知のフィールド {}", r.field), "not_found", 404));
            data.push(format!("{}:null", quote_json(&r.alias)));
            continue;
        };
        let input = match &r.input {
            Some(src) => match gql_value_to_json(src, vars) {
                Ok(j) => j,
                Err(e) => {
                    errors.push(gql_error(&e, "invalid_argument", 400));
                    data.push(format!("{}:null", quote_json(&r.alias)));
                    continue;
                }
            },
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
        Response { status, content_type: "application/json; charset=utf-8", headers: vec![("cache-control".into(), "no-store".into())], bytes: None, chunks: None, streaming: None, body }
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
    fn answers_the_minimal_introspection_the_open_cosmo_gateway_uses() {
        let q = "{\"query\":\"{ __schema { queryType { fields { name } } mutationType { fields { name } } } }\"}";
        let (st, out) = execute(q, &paths(), &fake);
        assert_eq!(st, 200);
        assert!(out.contains("\"queryType\":{\"fields\":[{\"name\":\"reaxt_ready\"}]}"), "{out}");
        assert!(out.contains("\"mutationType\":{\"fields\":[{\"name\":\"demo_v1_Counter_Add\"}"), "{out}");
    }

    #[test]
    fn accepts_literal_arguments() {
        let q = "{\"query\":\"mutation($n: Int) { a: demo_v1_Counter_Add(input: {by: 1, tags: [\\\"x\\\", ok], deep: {n: $n, f: -2.5, t: true, z: null}}) }\",\"variables\":{\"n\":7}}";
        let (_, out) = execute(q, &paths(), &fake);
        assert!(out.contains("\"a\":{\"echo\":{\"by\":1,\"tags\":[\"x\",\"ok\"],\"deep\":{\"n\":7,\"f\":-2.5,\"t\":true,\"z\":null}}}"), "{out}");
        // 壊れたリテラルはエラーになり、他のフィールドには影響しない
        let (_, out) = execute("{\"query\":\"mutation { demo_v1_Counter_Add(input: {by: ) }\"}", &paths(), &fake);
        assert!(out.contains("invalid_argument"), "{out}");
    }

    #[test]
    fn rejects_unknown_fields_and_bad_json() {
        let (_, out) = execute("{\"query\":\"mutation { demo_v1_Counter_Add(other: 1) }\"}", &paths(), &fake);
        assert!(out.contains("input: 値"), "{out}");
        let (_, out) = execute("{\"query\":\"mutation { nope }\"}", &paths(), &fake);
        assert!(out.contains("not_found"), "{out}");
        let (st, _) = execute("{bad", &paths(), &fake);
        assert_eq!(st, 400);
    }

    #[test]
    fn read_methods_become_query_fields() {
        assert!(is_read_method("Get") && is_read_method("GetVisits") && is_read_method("ListAll") && is_read_method("ReadOne"));
        assert!(!is_read_method("Getter") && !is_read_method("Add") && !is_read_method("Settle"));
        let p = vec!["/demo.v1.Counter/Add".to_string(), "/demo.v1.Counter/GetTotal".to_string()];
        let s = sdl(&p);
        assert!(s.contains("type Query {\n  reaxt_ready: Boolean! @shareable\n  demo_v1_Counter_GetTotal(input: JSON): JSON"), "{s}");
        assert!(s.contains("type Mutation {\n  demo_v1_Counter_Add(input: JSON): JSON"), "{s}");
        // 変更用が1つも無ければ Mutation 型は出さない
        assert!(!sdl(&["/demo.v1.Counter/Get".to_string()]).contains("type Mutation"));
        // 読み取りは Query として実行でき、reaxt_ready と並べても動く
        let call = |path: &str, _b: &str| -> (u16, String) { (200, format!("\"{path}\"")) };
        let (_, out) = execute("{\"query\":\"{ reaxt_ready demo_v1_Counter_GetTotal }\"}", &p, &call);
        assert!(out.contains("\"reaxt_ready\":true") && out.contains("\"demo_v1_Counter_GetTotal\":\"/demo.v1.Counter/GetTotal\""), "{out}");
        let (_, out) = execute("{\"query\":\"{ __schema { queryType { fields { name } } } }\"}", &p, &call);
        assert!(out.contains("demo_v1_Counter_GetTotal") && out.contains("reaxt_ready"), "{out}");
    }

    #[test]
    fn answers_federation_sdl() {
        let (_, out) = execute("{\"query\":\"query { _service { sdl } }\"}", &paths(), &fake);
        assert!(out.contains("_service") && out.contains("scalar JSON"), "{out}");
    }
}
