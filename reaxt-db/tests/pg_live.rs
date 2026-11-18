//! PostgreSQLワイヤ(`PgDb`)の実サーバー往復(環境変数 REAXT_LIVE_PG=host:port がある時だけ実行)。
//! 例: aruaru-server の pgwire ポート。ユーザー・パスワード・DB名は REAXT_LIVE_PG_USER/PASSWORD/DB。
use reaxt_db::pg::PgDb;
use reaxt_db::Db;

#[test]
fn pg_live_round_trip() {
    let Ok(addr) = std::env::var("REAXT_LIVE_PG") else { return };
    let user = std::env::var("REAXT_LIVE_PG_USER").unwrap_or_else(|_| "aruaru".into());
    let db = PgDb::new(addr, user, std::env::var("REAXT_LIVE_PG_PASSWORD").ok(), std::env::var("REAXT_LIVE_PG_DB").unwrap_or_else(|_| "aruaru".into())).aruaru();
    let _ = db.execute("DROP TABLE reaxt_pg", &[]);
    db.execute("CREATE TABLE reaxt_pg (id INT, name TEXT)", &[]).unwrap();
    db.execute_committed("INSERT INTO reaxt_pg (id, name) VALUES (?, ?)", &[7.into(), "a ? b".into()], "reaxt pg test").unwrap();
    let r = db.query("SELECT id, name FROM reaxt_pg WHERE id = ?", &[7.into()]).unwrap();
    assert_eq!(r.columns, vec!["id".to_string(), "name".to_string()], "{r:?}");
    assert_eq!(r.rows, vec![vec!["7".to_string(), "a ? b".to_string()]], "{r:?}");
    assert!(db.query("SELECT * FROM no_such_table", &[]).is_err());
    // 失敗の後も接続を作り直して続けられる
    assert_eq!(db.query("SELECT id FROM reaxt_pg", &[]).unwrap().rows.len(), 1);
}
