//! PostgreSQLワイヤ(v3・簡易問い合わせ)のクライアント。標準ライブラリだけで動く(TLSなし)。
//! PostgreSQL・CockroachDB・YugabyteDB・aruaru-db(pgwire)など、PostgreSQL互換のデータベースに使える。
//! 認証: trust / 平文 / MD5 / SCRAM-SHA-256。接続は使い回す(待機接続を最大4本プール。古い接続は1回だけ作り直す)。
//! TLSが必要な接続先(クラウドの多く)はTLS終端のプロキシ経由にする。
use crate::{Db, DbError, Dialect, Rows};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub struct PgDb {
    pub addr: String,
    pub user: String,
    pub password: Option<String>,
    pub database: String,
    /// aruaru-db(バージョン管理DB)なら true: `execute_committed`で`aruaru_commit`を積む。他のDBでは false のまま。
    pub aruaru: bool,
    /// 使い終わった接続を再利用する入れ物(最大`max_idle`本)。
    pool: std::sync::Mutex<Vec<Conn>>,
    pub max_idle: usize,
}

impl PgDb {
    pub fn new(addr: impl Into<String>, user: impl Into<String>, password: Option<String>, database: impl Into<String>) -> Self {
        PgDb { addr: addr.into(), user: user.into(), password, database: database.into(), aruaru: false, pool: std::sync::Mutex::new(Vec::new()), max_idle: 4 }
    }
    /// 再利用する待機接続の最大数(0なら毎回接続し直す)。
    pub fn max_idle(mut self, n: usize) -> Self {
        self.max_idle = n;
        self
    }
    /// 現在プールにある待機接続の数(試験・監視用)。
    pub fn idle_connections(&self) -> usize {
        self.pool.lock().unwrap_or_else(|x| x.into_inner()).len()
    }
    /// aruaru-db として扱う(コミット操作を有効にする)。
    pub fn aruaru(mut self) -> Self {
        self.aruaru = true;
        self
    }
}

fn e<T>(m: impl Into<String>) -> Result<T, DbError> {
    Err(DbError(m.into()))
}

struct Conn(TcpStream);
impl Conn {
    fn send(&mut self, tag: Option<u8>, body: &[u8]) -> Result<(), DbError> {
        let mut m = Vec::with_capacity(body.len() + 5);
        if let Some(t) = tag {
            m.push(t);
        }
        m.extend_from_slice(&((body.len() + 4) as u32).to_be_bytes());
        m.extend_from_slice(body);
        self.0.write_all(&m).map_err(|x| DbError(x.to_string()))
    }
    fn recv(&mut self) -> Result<(u8, Vec<u8>), DbError> {
        let mut h = [0u8; 5];
        self.0.read_exact(&mut h).map_err(|x| DbError(format!("読み取り失敗: {x}")))?;
        let len = u32::from_be_bytes([h[1], h[2], h[3], h[4]]) as usize;
        if !(4..=(256 << 20)).contains(&len) {
            return e("不正なメッセージ長");
        }
        let mut b = vec![0u8; len - 4];
        self.0.read_exact(&mut b).map_err(|x| DbError(format!("読み取り失敗: {x}")))?;
        Ok((h[0], b))
    }
}

fn cstr(b: &[u8]) -> (String, &[u8]) {
    let n = b.iter().position(|&x| x == 0).unwrap_or(b.len());
    (String::from_utf8_lossy(&b[..n]).into_owned(), b.get(n + 1..).unwrap_or(&[]))
}
fn pg_error(b: &[u8]) -> String {
    let mut msg = String::new();
    let mut r = b;
    while let Some((&k, rest)) = r.split_first() {
        if k == 0 {
            break;
        }
        let (v, rest) = cstr(rest);
        if k == b'M' {
            msg = v;
        }
        r = rest;
    }
    msg
}

impl PgDb {
    fn connect(&self) -> Result<Conn, DbError> {
        let s = TcpStream::connect(&self.addr).map_err(|x| DbError(format!("接続できません {}: {x}", self.addr)))?;
        s.set_read_timeout(Some(Duration::from_secs(30))).ok();
        let mut c = Conn(s);
        let mut sm = Vec::new();
        sm.extend_from_slice(&196608u32.to_be_bytes());
        for (k, v) in [("user", &self.user), ("database", &self.database)] {
            sm.extend_from_slice(k.as_bytes());
            sm.push(0);
            sm.extend_from_slice(v.as_bytes());
            sm.push(0);
        }
        sm.push(0);
        c.send(None, &sm)?;
        let mut scram: Option<crate::scram::Client> = None;
        loop {
            let (t, b) = c.recv()?;
            match t {
                b'R' => {
                    let code = u32::from_be_bytes(b[..4].try_into().map_err(|_| DbError("認証メッセージが短い".into()))?);
                    match code {
                        0 => {}
                        3 => {
                            let p = self.password.as_ref().ok_or_else(|| DbError("パスワードが必要です".into()))?;
                            c.send(Some(b'p'), format!("{p}\0").as_bytes())?;
                        }
                        5 => {
                            let p = self.password.as_ref().ok_or_else(|| DbError("パスワードが必要です".into()))?;
                            let inner = crate::scram::md5_hex(format!("{p}{}", self.user).as_bytes());
                            let mut salted = inner.into_bytes();
                            salted.extend_from_slice(&b[4..8]);
                            c.send(Some(b'p'), format!("md5{}\0", crate::scram::md5_hex(&salted)).as_bytes())?;
                        }
                        10 => {
                            let p = self.password.as_ref().ok_or_else(|| DbError("パスワードが必要です".into()))?;
                            let cl = crate::scram::Client::new(p);
                            let first = cl.first_message();
                            let mut body = b"SCRAM-SHA-256\0".to_vec();
                            body.extend_from_slice(&(first.len() as u32).to_be_bytes());
                            body.extend_from_slice(first.as_bytes());
                            c.send(Some(b'p'), &body)?;
                            scram = Some(cl);
                        }
                        11 => {
                            let cl = scram.as_mut().ok_or_else(|| DbError("SCRAMの順序が不正".into()))?;
                            let fin = cl.final_message(&String::from_utf8_lossy(&b[4..]))?;
                            c.send(Some(b'p'), fin.as_bytes())?;
                        }
                        12 => {
                            let cl = scram.as_ref().ok_or_else(|| DbError("SCRAMの順序が不正".into()))?;
                            cl.verify(&String::from_utf8_lossy(&b[4..]))?;
                        }
                        n => return e(format!("未対応の認証方式 {n}")),
                    }
                }
                b'E' => return e(pg_error(&b)),
                b'Z' => return Ok(c),
                _ => {} // S(パラメータ)・K(鍵)・N(通知)は無視
            }
        }
    }

    /// 接続1本で1つ実行する。外側のErrは通信失敗(bool=何か受信済みか)、内側のErrはSQLの失敗(接続は再利用できる)。
    fn run_on(&self, c: &mut Conn, sql: &str) -> Result<Result<Rows, String>, (DbError, bool)> {
        let mut any = false;
        c.send(Some(b'Q'), format!("{sql}\0").as_bytes()).map_err(|x| (x, false))?;
        let mut out = Rows::default();
        let mut failure: Option<String> = None;
        loop {
            let (t, b) = c.recv().map_err(|x| (x, any))?;
            any = true;
            let short = || (DbError("行が短い".into()), true);
            match t {
                b'T' => {
                    let n = u16::from_be_bytes([b[0], b[1]]) as usize;
                    let mut r = &b[2..];
                    for _ in 0..n {
                        let (name, rest) = cstr(r);
                        out.columns.push(name);
                        r = rest.get(18..).unwrap_or(&[]); // 表OID(4)・列番号(2)・型OID(4)・型長(2)・修飾(4)・形式(2)
                    }
                }
                b'D' => {
                    let n = u16::from_be_bytes([b[0], b[1]]) as usize;
                    let mut r = &b[2..];
                    let mut row = Vec::with_capacity(n);
                    for _ in 0..n {
                        let l = i32::from_be_bytes(r.get(..4).ok_or_else(short)?.try_into().map_err(|_| short())?);
                        r = &r[4..];
                        if l < 0 {
                            row.push(String::new());
                        } else {
                            let l = l as usize;
                            row.push(String::from_utf8_lossy(r.get(..l).ok_or_else(short)?).into_owned());
                            r = &r[l..];
                        }
                    }
                    out.rows.push(row);
                }
                b'E' => failure = Some(pg_error(&b)),
                b'Z' => break,
                _ => {}
            }
        }
        Ok(match failure {
            Some(m) => Err(m),
            None => Ok(out),
        })
    }

    fn give_back(&self, c: Conn) {
        let mut p = self.pool.lock().unwrap_or_else(|x| x.into_inner());
        if p.len() < self.max_idle {
            p.push(c);
        }
    }

    /// プールの接続があれば再利用する。古くなった接続(最初の応答の前に切れた)は捨てて1回だけ作り直す。
    /// 応答の途中で切れた場合は再試行しない(書き込みの二重実行を避ける)。
    fn run(&self, sql: &str) -> Result<Rows, DbError> {
        let idle = self.pool.lock().unwrap_or_else(|x| x.into_inner()).pop();
        if let Some(mut c) = idle {
            match self.run_on(&mut c, sql) {
                Ok(r) => {
                    self.give_back(c);
                    return r.map_err(DbError);
                }
                Err((_, false)) => {} // 古い接続: 作り直して再試行
                Err((x, true)) => return Err(x),
            }
        }
        let mut c = self.connect()?;
        match self.run_on(&mut c, sql) {
            Ok(r) => {
                self.give_back(c);
                r.map_err(DbError)
            }
            Err((x, _)) => Err(x),
        }
    }
}

impl Db for PgDb {
    fn dialect(&self) -> Dialect {
        Dialect::Postgres
    }
    fn query_raw(&self, sql: &str) -> Result<Rows, DbError> {
        self.run(sql)
    }
    fn exec_raw(&self, sql: &str, commit_message: Option<&str>) -> Result<(), DbError> {
        self.run(sql)?;
        // aruaru-dbはバージョン管理DB: 指定があればコミットを積む(他のDBでは使わない)
        if let Some(m) = commit_message {
            if !self.aruaru {
                return e("コミットメッセージはaruaru-db専用です(PgDb::aruaru()を指定)");
            }
            self.run(&format!("SELECT aruaru_commit('{}')", m.replace('\'', "''")))?;
        }
        Ok(())
    }
}
