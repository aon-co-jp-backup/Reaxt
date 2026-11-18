//! Reaxt のデータベース連携。
//! - `Param`/`bind`: `?` プレースホルダーへ値を安全に束縛する(文字列は`'`を二重化、NUL拒否、`?`は引用符の外だけ)。
//! - `Db`: 同期のトレイト(ハンドラは同期の`Fn`)。`DbHandle`に包んでDI(`ctx.get::<DbHandle>()`)で受け取る。
//! - `AruaruDb`: aruaru-server のGraphQL(`sql`問い合わせ・`execSql`変更)を叩くクライアント(標準ライブラリのみ)。
//! 管理トークン(`x-admin-token`)はサーバー側の設定で渡す。ブラウザへは出さない。
use rust_json::{parse_light_secure, quote_json, LightLimits, LightValue};
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;
pub mod pg;
pub mod scram;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbError(pub String);
impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for DbError {}
fn err<T>(m: impl Into<String>) -> Result<T, DbError> {
    Err(DbError(m.into()))
}

/// SQLへ束縛する値。
#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    Null,
    Int(i64),
    Bool(bool),
    Text(String),
}
impl From<i64> for Param {
    fn from(v: i64) -> Self {
        Param::Int(v)
    }
}
impl From<i32> for Param {
    fn from(v: i32) -> Self {
        Param::Int(v as i64)
    }
}
impl From<bool> for Param {
    fn from(v: bool) -> Self {
        Param::Bool(v)
    }
}
impl From<&str> for Param {
    fn from(v: &str) -> Self {
        Param::Text(v.into())
    }
}
impl From<String> for Param {
    fn from(v: String) -> Self {
        Param::Text(v)
    }
}

/// 対応するデータベースの方言(文字列の引用規則・真偽値の書き方が違う)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// PostgreSQL互換: PostgreSQL・CockroachDB・YugabyteDB・Redshift・aruaru-db(標準の`''`二重化)
    Postgres,
    /// MySQL・MariaDB・TiDB・Snowflake・BigQuery(`\`も文字列内の制御文字なので二重化が要る)
    MySql,
    /// Oracle(真偽値型が無いので1/0。`''`二重化)
    Oracle,
    /// SQL Server(Unicode文字列は`N'...'`、真偽値は1/0)
    SqlServer,
    /// SQLite
    Sqlite,
}
impl Dialect {
    fn backslash_escapes(self) -> bool {
        matches!(self, Dialect::MySql)
    }
}

fn literal(d: Dialect, p: &Param) -> Result<String, DbError> {
    Ok(match p {
        Param::Null => "NULL".into(),
        Param::Int(n) => n.to_string(),
        Param::Bool(b) => match d {
            Dialect::Oracle | Dialect::SqlServer | Dialect::Sqlite => if *b { "1" } else { "0" }.into(),
            _ => if *b { "TRUE" } else { "FALSE" }.into(),
        },
        Param::Text(s) => {
            if s.contains('\0') {
                return err("文字列にNULは使えません");
            }
            let mut body = s.replace('\'', "''");
            if d.backslash_escapes() {
                body = body.replace('\\', "\\\\");
            }
            match d {
                Dialect::SqlServer => format!("N'{body}'"),
                _ => format!("'{body}'"),
            }
        }
    })
}

/// PostgreSQL方言で束縛する(`bind_for(Dialect::Postgres, ..)`と同じ)。
pub fn bind(sql: &str, params: &[Param]) -> Result<String, DbError> {
    bind_for(Dialect::Postgres, sql, params)
}

/// `?` を順に値へ置き換える。文字列`'...'`・識別子`"..."`・コメント(`-- ...`、`/* ... */`)の中の`?`は置き換えない。
/// 個数が合わなければエラー。MySQL系は文字列内の`\`による引用符の打ち消しも解釈する。
pub fn bind_for(d: Dialect, sql: &str, params: &[Param]) -> Result<String, DbError> {
    let c: Vec<char> = sql.chars().collect();
    let mut out = String::with_capacity(sql.len() + 16);
    let mut it = params.iter();
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            q @ ('\'' | '"') => {
                out.push(q);
                i += 1;
                loop {
                    if i >= c.len() {
                        return err("引用符が閉じていません");
                    }
                    let ch = c[i];
                    out.push(ch);
                    i += 1;
                    if ch == '\\' && d.backslash_escapes() && q == '\'' {
                        if i < c.len() {
                            out.push(c[i]);
                            i += 1;
                        }
                    } else if ch == q {
                        if i < c.len() && c[i] == q {
                            out.push(c[i]); // 二重化された引用符
                            i += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
            '-' if i + 1 < c.len() && c[i + 1] == '-' => {
                while i < c.len() && c[i] != '\n' {
                    out.push(c[i]);
                    i += 1;
                }
            }
            '/' if i + 1 < c.len() && c[i + 1] == '*' => {
                out.push_str("/*");
                i += 2;
                loop {
                    if i + 1 >= c.len() {
                        return err("コメントが閉じていません");
                    }
                    if c[i] == '*' && c[i + 1] == '/' {
                        out.push_str("*/");
                        i += 2;
                        break;
                    }
                    out.push(c[i]);
                    i += 1;
                }
            }
            '?' => match it.next() {
                Some(p) => {
                    out.push_str(&literal(d, p)?);
                    i += 1;
                }
                None => return err("プレースホルダーが値より多いです"),
            },
            ch => {
                out.push(ch);
                i += 1;
            }
        }
    }
    if it.next().is_some() {
        return err("値がプレースホルダーより多いです");
    }
    Ok(out)
}

/// 問い合わせ結果(値は文字列)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub trait Db: Send + Sync {
    /// このデータベースの方言(束縛の規則を決める)。既定はPostgreSQL互換。
    fn dialect(&self) -> Dialect {
        Dialect::Postgres
    }
    /// 束縛済みのSQLを実行して行を返す(SELECT)。
    fn query_raw(&self, sql: &str) -> Result<Rows, DbError>;
    /// 束縛済みのSQLを実行する(INSERT/UPDATE/DELETE/DDL)。`commit_message`があれば自動コミットする。
    fn exec_raw(&self, sql: &str, commit_message: Option<&str>) -> Result<(), DbError>;

    fn query(&self, sql: &str, params: &[Param]) -> Result<Rows, DbError> {
        self.query_raw(&bind_for(self.dialect(), sql, params)?)
    }
    fn execute(&self, sql: &str, params: &[Param]) -> Result<(), DbError> {
        self.exec_raw(&bind_for(self.dialect(), sql, params)?, None)
    }
    /// 実行して、成功したらバージョン管理にコミットする(aruaru-dbの履歴に残る)。
    fn execute_committed(&self, sql: &str, params: &[Param], message: &str) -> Result<(), DbError> {
        self.exec_raw(&bind_for(self.dialect(), sql, params)?, Some(message))
    }
}

/// DI用の包み(`Module::provide(DbHandle(...))` → `ctx.get::<DbHandle>()`)。
#[derive(Clone)]
pub struct DbHandle(pub Arc<dyn Db>);
impl DbHandle {
    pub fn new(db: impl Db + 'static) -> Self {
        DbHandle(Arc::new(db))
    }
}

/// aruaru-server(GraphQL `/graphql`)のクライアント。
pub struct AruaruDb {
    pub addr: String,
    pub admin_token: Option<String>,
}

impl AruaruDb {
    pub fn new(addr: impl Into<String>, admin_token: Option<String>) -> Self {
        AruaruDb { addr: addr.into(), admin_token }
    }
    /// `ARUARU_GQL`(既定 127.0.0.1:4100)と`ARUARU_DB_ADMIN_TOKEN`から作る。
    pub fn from_env() -> Self {
        AruaruDb::new(std::env::var("ARUARU_GQL").unwrap_or_else(|_| "127.0.0.1:4100".into()), std::env::var("ARUARU_DB_ADMIN_TOKEN").ok())
    }

    fn gql(&self, query: &str) -> Result<LightValue, DbError> {
        let body = format!("{{\"query\":{}}}", quote_json(query));
        let tok = self.admin_token.as_ref().map(|t| format!("x-admin-token: {t}\r\n")).unwrap_or_default();
        let req = format!("POST /graphql HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n{tok}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", self.addr, body.len());
        let mut s = std::net::TcpStream::connect(&self.addr).map_err(|e| DbError(format!("接続できません {}: {e}", self.addr)))?;
        s.set_read_timeout(Some(Duration::from_secs(15))).ok();
        s.write_all(req.as_bytes()).map_err(|e| DbError(e.to_string()))?;
        let mut raw = Vec::new();
        s.read_to_end(&mut raw).map_err(|e| DbError(e.to_string()))?;
        let text = String::from_utf8_lossy(&raw).into_owned();
        let (head, rest) = text.split_once("\r\n\r\n").ok_or_else(|| DbError("不正な応答".into()))?;
        let first = head.lines().next().unwrap_or("");
        if !first.contains(" 200") {
            return err(format!("上流が {first}"));
        }
        let payload = if head.to_ascii_lowercase().contains("transfer-encoding: chunked") { dechunk(rest)? } else { rest.to_string() };
        let v = parse_light_secure(&payload, &LightLimits { max_bytes: 32 << 20, max_depth: 16 }).map_err(|e| DbError(e.to_string()))?;
        if let Some(es) = v.get("errors").and_then(|e| e.as_array()) {
            if let Some(m) = es.first().and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                return err(m.to_string());
            }
        }
        Ok(v)
    }
}

fn dechunk(rest: &str) -> Result<String, DbError> {
    let mut out = String::new();
    let mut r = rest;
    loop {
        let (len, tail) = r.split_once("\r\n").ok_or_else(|| DbError("chunk".into()))?;
        let n = usize::from_str_radix(len.trim(), 16).map_err(|e| DbError(e.to_string()))?;
        if n == 0 {
            return Ok(out);
        }
        out.push_str(tail.get(..n).ok_or_else(|| DbError("chunk長".into()))?);
        r = tail.get(n + 2..).ok_or_else(|| DbError("chunk終端".into()))?;
    }
}

impl Db for AruaruDb {
    fn query_raw(&self, sql: &str) -> Result<Rows, DbError> {
        let v = self.gql(&format!("query {{ sql(query: {}) {{ columns rows }} }}", quote_json(sql)))?;
        let r = v.get("data").and_then(|d| d.get("sql")).ok_or_else(|| DbError("応答にdata.sqlがありません".into()))?;
        let strs = |a: &LightValue| -> Vec<String> { a.as_array().map(|x| x.iter().map(|s| s.as_str().unwrap_or("").to_string()).collect()).unwrap_or_default() };
        Ok(Rows {
            columns: r.get("columns").map(strs).unwrap_or_default(),
            rows: r.get("rows").and_then(|x| x.as_array()).map(|rows| rows.iter().map(strs).collect()).unwrap_or_default(),
        })
    }
    fn exec_raw(&self, sql: &str, commit_message: Option<&str>) -> Result<(), DbError> {
        let msg = commit_message.map(|m| format!(", commitMessage: {}", quote_json(m))).unwrap_or_default();
        let v = self.gql(&format!("mutation {{ execSql(sql: {}, autoCommit: {}{msg}) {{ success message }} }}", quote_json(sql), commit_message.is_some()))?;
        let r = v.get("data").and_then(|d| d.get("execSql")).ok_or_else(|| DbError("応答にdata.execSqlがありません".into()))?;
        if r.get("success").and_then(|s| s.as_bool()) == Some(true) {
            Ok(())
        } else {
            err(r.get("message").and_then(|m| m.as_str()).unwrap_or("失敗").to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Fake(Mutex<Vec<String>>);
    impl Db for Fake {
        fn query_raw(&self, sql: &str) -> Result<Rows, DbError> {
            self.0.lock().unwrap().push(sql.into());
            Ok(Rows::default())
        }
        fn exec_raw(&self, sql: &str, m: Option<&str>) -> Result<(), DbError> {
            self.0.lock().unwrap().push(format!("{sql}|{m:?}"));
            Ok(())
        }
    }

    #[test]
    fn binds_and_escapes() {
        let s = bind("SELECT * FROM t WHERE a = ? AND b = ? AND c = ? AND d = ?", &["O'Brien; DROP".into(), 3.into(), true.into(), Param::Null]).unwrap();
        assert_eq!(s, "SELECT * FROM t WHERE a = 'O''Brien; DROP' AND b = 3 AND c = TRUE AND d = NULL");
    }
    #[test]
    fn question_mark_inside_quotes_is_kept_and_counts_checked() {
        assert_eq!(bind("SELECT '?' , ?", &[1.into()]).unwrap(), "SELECT '?' , 1");
        assert!(bind("SELECT ?, ?", &[1.into()]).is_err());
        assert!(bind("SELECT ?", &[1.into(), 2.into()]).is_err());
        assert!(bind("SELECT 'x", &[]).is_err());
        assert!(bind("SELECT ?", &["a\0b".into()]).is_err());
    }
    #[test]
    fn injection_stays_inside_the_literal() {
        let s = bind("SELECT ?", &["x' OR '1'='1".into()]).unwrap();
        assert_eq!(s, "SELECT 'x'' OR ''1''=''1'");
    }
    #[test]
    fn trait_helpers_bind_before_calling() {
        let f = Fake(Mutex::new(vec![]));
        f.execute_committed("INSERT INTO t VALUES (?)", &["a".into()], "msg").unwrap();
        f.query("SELECT ?", &[1.into()]).unwrap();
        let log = f.0.lock().unwrap().clone();
        assert_eq!(log, vec!["INSERT INTO t VALUES ('a')|Some(\"msg\")".to_string(), "SELECT 1".to_string()]);
    }
}

#[cfg(test)]
mod dialect_tests {
    use super::*;

    #[test]
    fn mysql_escapes_backslash_and_understands_backslash_quote() {
        let s = bind_for(Dialect::MySql, "SELECT ?, 'it\\'s ?', ?", &["a\\b'c".into(), true.into()]).unwrap();
        assert_eq!(s, "SELECT 'a\\\\b''c', 'it\\'s ?', TRUE");
        // Postgresでは`\`は普通の文字(引用符を打ち消さない)
        assert_eq!(bind_for(Dialect::Postgres, "SELECT ?", &["a\\b".into()]).unwrap(), "SELECT 'a\\b'");
    }
    #[test]
    fn oracle_sqlserver_sqlite_booleans_and_unicode() {
        assert_eq!(bind_for(Dialect::Oracle, "SELECT ?, ?", &[true.into(), false.into()]).unwrap(), "SELECT 1, 0");
        assert_eq!(bind_for(Dialect::SqlServer, "SELECT ?, ?", &["日本".into(), true.into()]).unwrap(), "SELECT N'日本', 1");
        assert_eq!(bind_for(Dialect::Sqlite, "SELECT ?", &[false.into()]).unwrap(), "SELECT 0");
    }
    #[test]
    fn question_marks_in_identifiers_and_comments_are_kept() {
        let s = bind("SELECT \"a?b\" FROM t -- why ?\nWHERE x = ? /* ? */ AND y = ?", &[1.into(), 2.into()]).unwrap();
        assert_eq!(s, "SELECT \"a?b\" FROM t -- why ?\nWHERE x = 1 /* ? */ AND y = 2");
        assert!(bind("SELECT 1 /* open", &[]).is_err());
    }
}
