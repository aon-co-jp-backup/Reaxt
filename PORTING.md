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

> 日本語・英語以外は AI 翻訳をネイティブらしい表現に整えたもので、母語話者による確認はまだです。 / Apart from Japanese and English, these are AI translations polished toward natural phrasing, not yet reviewed by native speakers.

## 日本語
**再開のしかた**: `RS-JSON` と `RPoem` を同じ階層の兄弟ディレクトリに置き、`cargo test --workspace` を実行して全件成功を確認します。実サーバーを使う試験は環境変数で有効にします(`REAXT_LIVE_ARUARU`、`REAXT_LIVE_PG`)。
**次の作業(この順)**: 実 Cosmo Router での合成 → 実 PostgreSQL/CockroachDB と TLS → MySQL/Oracle/Snowflake ドライバ → SET への配備 → `wasm-opt` と速度の実測。

## English
**How to resume**: put `RS-JSON` and `RPoem` next to this repository as sibling directories and run `cargo test --workspace`; every test should pass. Tests that need a live server are switched on with environment variables (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Next, in this order**: compose with a real Cosmo Router, connect to real PostgreSQL/CockroachDB over TLS, write the MySQL/Oracle/Snowflake drivers, deploy on SET, then add `wasm-opt` and measure speed.

## 简体中文
**如何继续**:把 `RS-JSON` 和 `RPoem` 放在与本仓库同级的目录下,运行 `cargo test --workspace`,确认全部测试通过。需要真实服务器的测试通过环境变量开启(`REAXT_LIVE_ARUARU`、`REAXT_LIVE_PG`)。
**接下来依次要做**:与真实的 Cosmo Router 联调 → 通过 TLS 连接真实的 PostgreSQL/CockroachDB → 实现 MySQL/Oracle/Snowflake 驱动 → 部署到 SET → 引入 `wasm-opt` 并实测速度。

## 繁體中文
**如何繼續**:將 `RS-JSON` 與 `RPoem` 放在與本儲存庫同層的目錄下,執行 `cargo test --workspace`,確認所有測試皆通過。需要真實伺服器的測試以環境變數啟用(`REAXT_LIVE_ARUARU`、`REAXT_LIVE_PG`)。
**接下來依序要做**:與真實的 Cosmo Router 整合測試 → 透過 TLS 連線真實的 PostgreSQL/CockroachDB → 實作 MySQL/Oracle/Snowflake 驅動程式 → 部署到 SET → 導入 `wasm-opt` 並實測速度。

## 한국어
**이어서 작업하는 방법**: `RS-JSON`과 `RPoem`을 이 저장소와 같은 상위 디렉터리에 나란히 두고 `cargo test --workspace`를 실행해 모든 테스트가 통과하는지 확인합니다. 실제 서버가 필요한 테스트는 환경 변수(`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`)로 켭니다.
**다음 작업(순서대로)**: 실제 Cosmo Router와 통합 → TLS로 실제 PostgreSQL/CockroachDB 연결 → MySQL/Oracle/Snowflake 드라이버 구현 → SET 배포 → `wasm-opt` 적용 및 속도 측정.

## Deutsch
**So geht es weiter**: `RS-JSON` und `RPoem` als Geschwisterverzeichnisse neben dieses Repository legen und `cargo test --workspace` ausführen; alle Tests müssen bestehen. Tests, die einen laufenden Server brauchen, werden über Umgebungsvariablen aktiviert (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Nächste Schritte in dieser Reihenfolge**: Zusammenspiel mit einem echten Cosmo Router, Anbindung an echtes PostgreSQL/CockroachDB über TLS, Treiber für MySQL/Oracle/Snowflake, Bereitstellung auf SET, danach `wasm-opt` und Geschwindigkeitsmessungen.

## Français
**Pour reprendre** : placer `RS-JSON` et `RPoem` à côté de ce dépôt, dans des répertoires frères, puis lancer `cargo test --workspace` ; tous les tests doivent passer. Les tests qui nécessitent un vrai serveur s'activent par des variables d'environnement (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Étapes suivantes, dans cet ordre** : intégration avec un vrai Cosmo Router, connexion à de vrais PostgreSQL/CockroachDB via TLS, écriture des pilotes MySQL/Oracle/Snowflake, déploiement sur SET, puis `wasm-opt` et mesures de vitesse.

## Español
**Cómo retomar el trabajo**: coloca `RS-JSON` y `RPoem` junto a este repositorio, como directorios hermanos, y ejecuta `cargo test --workspace`; todas las pruebas deben pasar. Las pruebas que necesitan un servidor real se activan con variables de entorno (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Siguientes pasos, por orden**: integración con un Cosmo Router real, conexión a PostgreSQL/CockroachDB reales mediante TLS, controladores de MySQL/Oracle/Snowflake, despliegue en SET y, después, `wasm-opt` y mediciones de velocidad.

## Português
**Como retomar**: coloque `RS-JSON` e `RPoem` ao lado deste repositório, como diretórios irmãos, e execute `cargo test --workspace`; todos os testes devem passar. Os testes que exigem um servidor real são ativados por variáveis de ambiente (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Próximos passos, em ordem**: integração com um Cosmo Router real, conexão a PostgreSQL/CockroachDB reais via TLS, drivers de MySQL/Oracle/Snowflake, implantação no SET e, por fim, `wasm-opt` e medições de velocidade.

## Русский
**Как продолжить работу**: поместите `RS-JSON` и `RPoem` рядом с этим репозиторием, в соседние каталоги, и выполните `cargo test --workspace` — все тесты должны пройти. Тесты, которым нужен настоящий сервер, включаются переменными окружения (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Дальнейшие шаги по порядку**: интеграция с настоящим Cosmo Router, подключение к реальным PostgreSQL/CockroachDB по TLS, драйверы MySQL/Oracle/Snowflake, развёртывание на SET, затем `wasm-opt` и замеры скорости.
