//! 実aruaru-serverとの往復(環境変数 REAXT_LIVE_ARUARU=host:port がある時だけ実行)。
use reaxt_db::{AruaruDb, Db};

#[test]
fn live_round_trip() {
    let Ok(addr) = std::env::var("REAXT_LIVE_ARUARU") else { return };
    let db = AruaruDb::new(addr, std::env::var("ARUARU_DB_ADMIN_TOKEN").ok());
    let _ = db.execute("DROP TABLE reaxt_t", &[]);
    db.execute("CREATE TABLE reaxt_t (id INT, name TEXT)", &[]).unwrap();
    db.execute_committed("INSERT INTO reaxt_t (id, name) VALUES (?, ?)", &[1.into(), "O'Brien ? x".into()], "reaxt test").unwrap();
    let r = db.query("SELECT id, name FROM reaxt_t WHERE id = ?", &[1.into()]).unwrap();
    // 既知: aruaru-db の main は INSERT 内の二重引用符('')を戻さず '' のまま保存する(修正ブランチ fix/insert-quoted-values が未マージ)。
    // 束縛側は標準SQLどおりに二重化しているので、どちらの結果も許す。`?` が値の中にあっても壊れないことを確かめる。
    let name = r.rows.first().and_then(|x| x.get(1)).cloned().unwrap_or_default();
    assert!(name == "O'Brien ? x" || name == "O''Brien ? x", "{r:?}");
    assert_eq!(r.rows.len(), 1);
    assert!(db.query("SELECT * FROM no_such_table", &[]).is_err());
}
