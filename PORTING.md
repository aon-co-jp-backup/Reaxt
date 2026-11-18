# Reaxt お引越し・再開メモ(PORTING.md)

別のPC・別アカウントで続きをやるための手順と到達点。設計は [DESIGN.md](DESIGN.md)、ルールは [CLAUDE.md](CLAUDE.md)。

## 必要な道具
- Rust(stable)+ `rustup target add wasm32-unknown-unknown`
- `wasm-bindgen` CLI **0.2.126**(`cargo install wasm-bindgen-cli --version 0.2.126`)
- 兄弟ディレクトリに `RS-JSON`(パス依存。`F:\RS-JSON`、Reaxtの1つ上の階層に並べる)。RPoem連携(`reaxt-rpoem`)は `RPoem` も同じ階層に必要
- 元アプリとの比較をする場合のみ: Node と pnpm(`npm install -g pnpm`)

`reaxt doctor` で確認できる。

## 動かす(aruaru-admin の例)
```bash
# 1) wasmを作る
cargo build -p aruaru-admin-reaxt --lib --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir examples/aruaru-admin/pkg target/wasm32-unknown-unknown/release/aruaru_admin_reaxt.wasm
# 2) 上流(aruaru-server)を起動。管理操作にはトークンが要る
ARUARU_DB_ADMIN_TOKEN=tk aruaru-server --data <dir> --gql-port 4100 --raft-id 1
# 3) 中継サーバー(同じトークンを環境変数で渡す。ブラウザには出ない)
ARUARU_DB_ADMIN_TOKEN=tk ARUARU_GQL=127.0.0.1:4100 cargo run -p aruaru-admin-reaxt --bin serve -- examples/aruaru-admin/pkg 8770
# → http://127.0.0.1:8770/
```
テスト: `cargo test --workspace`(全件成功を確認済み)。

## 到達点(2026-11-18)
- 段階0〜5まで実装(ルート規約・SSR・RPoem接続・ハイドレーション・Islands・Server Actions・CLI・フォーム対応)。
- aruaru-admin 全10画面を移植。管理操作は許可リスト11メソッドだけ中継(`serve.rs`)。
- serde / serde_json をwasmから排除。RS-JSON `light` に `parse_light_secure`・`quote_json`・`to_json`・`as_object`・`as_i64` を追加(RS-JSON `9aca39a`)。`reaxt-actions` は `Wire` トレイト。
- 元のReact版との比較(実測): 元(`aruaru-db-admin-web`)は JS 208KB(gzip 64KB)+CSS 16KB(gzip 3.7KB)、Reaxt版は wasm 366KB(gzip 113KB)+JS 26KB(gzip 5.4KB)。**転送量は約1.8倍**。

## 元アプリ(React)をビルドする手順(比較用)
`aruaru-admin/package.json` の `packageManager: pnpm@9.0.0` があると新しいpnpmは古い版の取得で失敗する。**リポジトリを汚さないよう、作業用コピーで**その行を外して `pnpm install` → `pnpm web:build`(`node_modules` は.gitignore対象外なので元の場所で実行しない)。

## 未了(優先順)
1. SET への配備(Reaxt自体の配備先・手順)
2. 描画速度・初回表示の実測(元との比較は転送量だけ済み)
3. wasm縮小の続き(リリース設定は適用済み。`wasm-opt` は未導入、入れて再計測)
4. (済)`wire_struct!` マクロで名前付きフィールドの構造体は1行で Wire 実装(enum・任意フィールド・入れ子の自動化は未了)
5. 元の画面の再現不足(ウィザードの段階表示、スライダー、CPU棒グラフ、サンプル表示)
6. RPoem のワイルドカードルート(現状 `*slug` は深さ8まで展開)
7. 破壊的操作(復元・移行成功系・ノード追加)の実機確認

## 注意
- 動作確認で作る一時ディレクトリ・サーバーは後始末する(`taskkill` でプロセス終了)。バックアップ作成などの確認用データは一時の `--data` に閉じ込める。
- トークン(`ARUARU_DB_ADMIN_TOKEN`)は表示・コミットしない。
- pushする場合は RS-JSON → Reaxt の順(Reaxt は RS-JSON の新APIに依存)。

## 再開用メッセージ(別アカウントでそのまま貼り付け)
前のセッションの続きです。`F:\Reaxt\PORTING.md` と `CLAUDE.md` を読んでから再開します。aruaru-admin の全10画面を移植し、wasmからserde系を外しました(Reaxt `cb5f3b1`、RS-JSON `9aca39a`)。元のReact版との実測では転送量がReaxt版で約1.8倍で、サイズでは未勝利です。未了は PORTING.md の「未了(優先順)」の順に進めてください。決まり: RESTを作らない、JS/TSを書かない、秘密は表示しない、実ブラウザと実サーバーで確認してから完了と報告する、返答は日本語のみ、pushは確認してから。

## 2026-11-19 追記: 連携の追加と未了
追加: JS/TSクライアント・jQueryプラグイン・Cosmo向けGraphQLサブグラフ・`reaxt-db`(詳細はREADME末尾と CLAUDE.md)。
未了(優先順): (1) 実Cosmo Router で `wgc` による合成と実行を確認する(今はcurlまで)、(2) 実PostgreSQL/CockroachDBへの接続確認とTLS対応、(3) MySQL・Oracle・SQL Server・Snowflake のドライバ(今は束縛の方言のみ。MySQLのワイヤは `caching_sha2_password` にTLSかRSAが要る、Snowflakeは独自のHTTPS API、Oracleは独自のTNSで難所)、(4) (済)接続プール(PgDbのみ。他DBは未実装)、(5) サブグラフの引数リテラル対応・Federationの`@key`エンティティ、(6) `pkg/` の再ビルド(`wire_struct!`反映)。


---

# 再開の手順と次の作業 / How to Resume and Next Steps (10 languages)

> AI翻訳(日英以外はネイティブ検証前)。/ AI translations (other than ja/en not yet native-checked).

## 日本語
**再開**: 兄弟ディレクトリに `RS-JSON`・`RPoem` を置く。`cargo test --workspace` で全件成功を確認。実サーバー試験は環境変数で有効化(`REAXT_LIVE_ARUARU`・`REAXT_LIVE_PG`)。**次の作業(順)**: 実Cosmo Routerでの合成 → 実PostgreSQL/CockroachDB+TLS → MySQL/Oracle/Snowflakeドライバ → 接続プール → SET配備 → `wasm-opt`と速度実測。

## English
**Resume**: place `RS-JSON` and `RPoem` as sibling directories and run `cargo test --workspace` (all tests should pass). Live-server tests are switched on by environment variables (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`). **Next, in order**: compose with a real Cosmo Router, real PostgreSQL/CockroachDB with TLS, MySQL/Oracle/Snowflake drivers, a connection pool, SET deployment, `wasm-opt` and speed measurements.

## 简体中文
**恢复**:将 `RS-JSON`、`RPoem` 放在同级目录,运行 `cargo test --workspace` 确认全部通过。真实服务器测试通过环境变量启用(`REAXT_LIVE_ARUARU`、`REAXT_LIVE_PG`)。**后续工作(按顺序)**:与真实 Cosmo Router 联合验证 → 真实 PostgreSQL/CockroachDB + TLS → MySQL/Oracle/Snowflake 驱动 → 连接池 → 部署到 SET → `wasm-opt` 与速度实测。

## 繁體中文
**恢復**:將 `RS-JSON`、`RPoem` 放在同層目錄,執行 `cargo test --workspace` 確認全部通過。真實伺服器測試透過環境變數啟用(`REAXT_LIVE_ARUARU`、`REAXT_LIVE_PG`)。**後續工作(依序)**:與真實 Cosmo Router 聯合驗證 → 真實 PostgreSQL/CockroachDB + TLS → MySQL/Oracle/Snowflake 驅動 → 連線池 → 部署到 SET → `wasm-opt` 與速度實測。

## 한국어
**재개**: `RS-JSON`과 `RPoem`을 같은 상위 디렉터리에 두고 `cargo test --workspace`로 전체 통과를 확인한다. 실제 서버 테스트는 환경 변수(`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`)로 켠다. **다음 작업(순서대로)**: 실제 Cosmo Router와 합성 → 실제 PostgreSQL/CockroachDB + TLS → MySQL/Oracle/Snowflake 드라이버 → 커넥션 풀 → SET 배포 → `wasm-opt`와 속도 실측.

## Deutsch
**Wiederaufnahme**: `RS-JSON` und `RPoem` als Geschwisterverzeichnisse ablegen und `cargo test --workspace` ausführen (alle Tests sollen bestehen). Tests gegen echte Server werden über Umgebungsvariablen aktiviert (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`). **Nächste Schritte, der Reihe nach**: Komposition mit einem echten Cosmo Router → echtes PostgreSQL/CockroachDB mit TLS → Treiber für MySQL/Oracle/Snowflake → Verbindungspool → Betrieb auf SET → `wasm-opt` und Geschwindigkeitsmessung.

## Français
**Reprise** : placer `RS-JSON` et `RPoem` en répertoires frères et lancer `cargo test --workspace` (tous les tests doivent passer). Les tests sur de vrais serveurs s'activent par variables d'environnement (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`). **Suite, dans l'ordre** : composition avec un vrai Cosmo Router → vrai PostgreSQL/CockroachDB avec TLS → pilotes MySQL/Oracle/Snowflake → pool de connexions → déploiement sur SET → `wasm-opt` et mesures de vitesse.

## Español
**Reanudar**: colocar `RS-JSON` y `RPoem` como directorios hermanos y ejecutar `cargo test --workspace` (todas las pruebas deben pasar). Las pruebas con servidores reales se activan con variables de entorno (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`). **Siguiente, en orden**: composición con un Cosmo Router real → PostgreSQL/CockroachDB reales con TLS → controladores de MySQL/Oracle/Snowflake → pool de conexiones → despliegue en SET → `wasm-opt` y mediciones de velocidad.

## Português
**Retomar**: colocar `RS-JSON` e `RPoem` como diretórios irmãos e executar `cargo test --workspace` (todos os testes devem passar). Os testes com servidores reais são ativados por variáveis de ambiente (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`). **Próximos passos, em ordem**: composição com um Cosmo Router real → PostgreSQL/CockroachDB reais com TLS → drivers de MySQL/Oracle/Snowflake → pool de conexões → implantação no SET → `wasm-opt` e medições de velocidade.

## Русский
**Возобновление**: разместите `RS-JSON` и `RPoem` в соседних каталогах и выполните `cargo test --workspace` (все тесты должны пройти). Тесты с настоящими серверами включаются переменными окружения (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`). **Дальнейшие шаги по порядку**: композиция с настоящим Cosmo Router → настоящие PostgreSQL/CockroachDB с TLS → драйверы MySQL/Oracle/Snowflake → пул соединений → развёртывание на SET → `wasm-opt` и замеры скорости.
