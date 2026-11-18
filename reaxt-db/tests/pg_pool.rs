//! `PgDb` の接続プール試験(偽のPostgreSQLサーバー。外部サーバー不要)。
//! 1接続につき1回だけ問い合わせに答えて切る=2回目は「古くなった接続」になる。
use reaxt_db::pg::PgDb;
use reaxt_db::Db;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn msg(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut m = vec![tag];
    m.extend_from_slice(&((body.len() + 4) as u32).to_be_bytes());
    m.extend_from_slice(body);
    m
}

#[test]
fn stale_pooled_connection_is_replaced_once() {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap().to_string();
    let accepted = Arc::new(AtomicUsize::new(0));
    let acc = accepted.clone();
    std::thread::spawn(move || {
        for s in l.incoming() {
            let mut s = s.unwrap();
            acc.fetch_add(1, Ordering::SeqCst);
            // 開始メッセージを読む(長さ4バイト+本文)
            let mut len = [0u8; 4];
            s.read_exact(&mut len).unwrap();
            let mut rest = vec![0u8; u32::from_be_bytes(len) as usize - 4];
            s.read_exact(&mut rest).unwrap();
            // 認証OK+準備完了
            let mut hello = msg(b'R', &0u32.to_be_bytes());
            hello.extend(msg(b'Z', b"I"));
            s.write_all(&hello).unwrap();
            // 問い合わせを1回だけ受け、1行返して準備完了。その後は接続を閉じる(次の再利用は失敗する)
            let mut h = [0u8; 5];
            s.read_exact(&mut h).unwrap();
            let mut q = vec![0u8; u32::from_be_bytes([h[1], h[2], h[3], h[4]]) as usize - 4];
            s.read_exact(&mut q).unwrap();
            let mut t = 1u16.to_be_bytes().to_vec();
            t.extend_from_slice(b"n\0");
            t.extend_from_slice(&[0u8; 18]);
            let mut d = 1u16.to_be_bytes().to_vec();
            d.extend_from_slice(&1i32.to_be_bytes());
            d.push(b'1');
            let mut out = msg(b'T', &t);
            out.extend(msg(b'D', &d));
            out.extend(msg(b'C', b"SELECT 1\0"));
            out.extend(msg(b'Z', b"I"));
            s.write_all(&out).unwrap();
        }
    });
    let db = PgDb::new(addr, "u", None, "d");
    let a = db.query("SELECT 1", &[]).unwrap();
    assert_eq!(a.rows, vec![vec!["1".to_string()]]);
    assert_eq!(db.idle_connections(), 1, "成功した接続はプールへ戻る");
    // 2回目: プールの接続は相手が閉じている→捨てて作り直し、成功する
    let b = db.query("SELECT 1", &[]).unwrap();
    assert_eq!(b.rows, vec![vec!["1".to_string()]]);
    assert_eq!(accepted.load(Ordering::SeqCst), 2, "古い接続を捨てて1回だけ接続し直した");
    // max_idle=0 なら戻さない
    let db0 = PgDb::new(db.addr.clone(), "u", None, "d").max_idle(0);
    db0.query("SELECT 1", &[]).unwrap();
    assert_eq!(db0.idle_connections(), 0);
}
