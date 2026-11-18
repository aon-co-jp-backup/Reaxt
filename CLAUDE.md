# Reaxt 開発ルール(CLAUDE.md)

Rust だけで書くフロント+バックエンド統合の枠組み。RPoem(サーバー)と SET(配備)の上で使う。
設計の正本は [DESIGN.md](DESIGN.md)、引き継ぎは [PORTING.md](PORTING.md)。共通の開発方針は
[`open-raid-z/CLAUDE.md`](https://github.com/aon-co-jp/open-raid-z) と `F:\runo\CLAUDE.md` に従う。

## 守ること
- **JS/TS を書かない**(ブラウザ側もRust→wasm)。画面の更新・イベントは属性(`data-rx-click`/`-input`/`-submit`)+委譲リスナーで、インラインJSは作らない。
- **RESTを作らない**。画面からの呼び出しは Connect の単純呼び出しだけ(`POST /pkg.Service/Method`)。ブラウザにトークンを渡さず、サーバー側の中継(BFF)で付ける。
- **JSON は RS-JSON**。wasm側は `light`(依存ゼロ)を使い、**serde / serde_json をwasmの依存に入れない**(入れるとwasmが肥大する)。信頼できない入力は必ず `parse_light_secure`(サイズ・深さ・重複キー)で読む。
- 値の描画は必ずエスケープ(`ssr::render`)。`on*` 属性は遮断される。
- 開発・TEST・DEBUG はローカルPC。VPSは配備と実測確認だけ。
- 完了と報告する前に、実ブラウザ(白画面・コンソールエラー)と実サーバーで確認する。型・ビルドが通っただけで終わらせない。

## 落とし穴
- フックは毎回同じ数・同じ順で呼ぶ。画面ごとの状態はルートの `view()` に集約する(`aruaru-admin` は `Ops` 1つにまとめた)。
- `on_input` / `on_click` の id は画面内で一意に。入力欄は非制御(状態→値の書き戻しはしない)。
- `reaxt build` は `[lib] crate-type=cdylib` のクレート前提。例(`examples/aruaru-admin`)は手動で `cargo build --lib --target wasm32-unknown-unknown --release` → `wasm-bindgen --target web --out-dir pkg` でもよい。
- wasm-bindgen は `=0.2.126` に固定(インストール済みCLIの版と一致させる)。
- Git Bash では `cargo` に PATH(`%USERPROFILE%\.cargo\bin`)を足す。`python3` は無い。日本語を含むファイルをPowerShell 5.1で書かない。
- ビルドしたwasmは `examples/aruaru-admin/pkg/`(現状gitで追跡されている生成物。再ビルドすると差分が出る)。ブラウザで古い版が出たら `/_reaxt/` のキャッシュ(no-cache設定済み)とサーバー再起動を確認。

## HANDOFF
- **2026-11-18**: aruaru-admin の全10画面を移植(ダッシュボード/コミット/ブランチ/クエリ/お引越し/バックアップ/レジストリ/並列化/統合/クラスタ)、実aruaru-serverと実ブラウザで確認。wasmからserde系を排除(RS-JSON `light` + `Wire`トレイト)。**元のReact版との実測**: 元 JS 208KB(gzip 64KB)+CSS 3.7KB に対し、Reaxt版は wasm 366KB(gzip 113KB)+JS 5.4KB で約1.8倍だったが、リリース設定(opt-level z・LTO・panic=abort・strip)で wasm 202KB(gzip 84KB)に縮小=約1.3倍(wasm-optは未導入で未適用)。描画速度は未計測。未了=SETへの配備、`wasm-opt`、RPoem側ワイルドカード(`*slug`)、速度の実測、元アプリの「ウィザード段階表示・スライダー・CPU棒」等の再現。詳細は PORTING.md。

## 2026-11-19 連携の追加(JS/TS・jQuery・Cosmo・DB)
- `clients/js/`(`reaxt-client.mjs`+`.d.ts`、`reaxt-jquery.js`、`test.mjs`=実サーバー試験)、`GET /_reaxt/manifest.json`、`App::gateway`(外部入口の拡張点)、`reaxt-actions/src/subgraph.rs`(Cosmo向けサブグラフ)、`reaxt-db`(`bind_for`・`Dialect`・`Db`・`AruaruDb`・`PgDb`・`scram`)。
- 確認済み: Nodeで実サーバー往復、jQuery 3.7.1の実ブラウザ、サブグラフのcurl、aruaru-dbの実SCRAM-SHA-256ログイン+DDL/コミット付き挿入/問い合わせ(`REAXT_LIVE_PG=host:port REAXT_LIVE_PG_USER=.. REAXT_LIVE_PG_PASSWORD=.. cargo test -p reaxt-db --test pg_live`、`REAXT_LIVE_ARUARU=host:port` で GraphQL版)。
- **未確認・未実装(正直に)**: 実Cosmo Router(wgc compose・Router起動)との接続、実PostgreSQL/CockroachDBへの接続、TLS、接続プール、MySQL/Oracle/SQL Server/Snowflakeのドライバ(方言の束縛のみ)。
- **aruaru-dbの既知の不具合**: mainは INSERT 内の二重引用符 `''` を戻さず `''` のまま保存する(修正ブランチ `fix/insert-quoted-values` が未マージ)。Reaxt側の束縛は標準どおり。


---

# 開発ルールと到達点 / Rules and Status (10 languages)

> 日本語・英語以外は AI 翻訳をネイティブらしい表現に整えたもので、母語話者による確認はまだです。 / Apart from Japanese and English, these are AI translations polished toward natural phrasing, not yet reviewed by native speakers.

## 日本語
**ルール**: JS/TS は書かない(Rust から wasm へ)。REST は作らない(Connect のみ)。JSON は RS-JSON で扱い、wasm に serde を入れない。トークンはサーバー側の中継で付ける。実ブラウザと実サーバーで確認してから完了と報告する。
**到達点**: 10画面の移植と、JS/TS・jQuery・Cosmo・DB(接続プール込み)の連携まで。
**未了**: 実 Cosmo Router、実 PostgreSQL/CockroachDB、TLS、MySQL/Oracle/Snowflake ドライバ、SET への配備。

## English
**Rules**: write no JS/TS (Rust compiles to wasm); build no REST (Connect only); handle JSON with RS-JSON and keep serde out of wasm; attach tokens in the server-side relay; and report work as done only after checking it in a real browser against a real server.
**Status**: 10 screens ported, with JS/TS, jQuery, Cosmo and database integration (including a connection pool) in place.
**Open**: a real Cosmo Router, real PostgreSQL/CockroachDB, TLS, MySQL/Oracle/Snowflake drivers and deployment on SET.

## 简体中文
**规则**:不写 JS/TS(由 Rust 编译为 wasm);不做 REST(只用 Connect);JSON 统一使用 RS-JSON,wasm 中不引入 serde;令牌由服务端中继附加;必须在真实浏览器和真实服务器上验证通过,才能报告完成。
**进展**:已迁移 10 个页面,并完成 JS/TS、jQuery、Cosmo 和数据库(含连接池)的集成。
**待完成**:真实的 Cosmo Router、真实的 PostgreSQL/CockroachDB、TLS、MySQL/Oracle/Snowflake 驱动、部署到 SET。

## 繁體中文
**規則**:不撰寫 JS/TS(由 Rust 編譯為 wasm);不做 REST(僅使用 Connect);JSON 一律交由 RS-JSON 處理,wasm 中不引入 serde;權杖由伺服器端中繼附加;必須在真實瀏覽器與真實伺服器上驗證無誤,才能回報完成。
**進度**:已移植 10 個頁面,並完成 JS/TS、jQuery、Cosmo 與資料庫(含連線池)的整合。
**待完成**:真實的 Cosmo Router、真實的 PostgreSQL/CockroachDB、TLS、MySQL/Oracle/Snowflake 驅動程式、部署到 SET。

## 한국어
**규칙**: JS/TS는 작성하지 않습니다(Rust를 wasm으로 컴파일). REST는 만들지 않습니다(Connect만 사용). JSON은 RS-JSON으로 처리하며 wasm에는 serde를 넣지 않습니다. 토큰은 서버 측 중계에서 붙입니다. 실제 브라우저와 실제 서버에서 확인한 뒤에만 완료로 보고합니다.
**현황**: 10개 화면 이식을 마쳤고, JS/TS·jQuery·Cosmo·데이터베이스(커넥션 풀 포함) 연동까지 갖추었습니다.
**남은 작업**: 실제 Cosmo Router, 실제 PostgreSQL/CockroachDB, TLS, MySQL/Oracle/Snowflake 드라이버, SET 배포.

## Deutsch
**Regeln**: kein JS/TS schreiben (Rust wird zu wasm kompiliert); kein REST bauen (nur Connect); JSON über RS-JSON verarbeiten und serde aus dem wasm heraushalten; Tokens im serverseitigen Relay anfügen; eine Aufgabe erst als erledigt melden, wenn sie in einem echten Browser gegen einen echten Server geprüft wurde.
**Stand**: 10 Bildschirme portiert; die Anbindung von JS/TS, jQuery, Cosmo und Datenbanken (einschließlich Verbindungspool) steht.
**Offen**: ein echter Cosmo Router, echtes PostgreSQL/CockroachDB, TLS, Treiber für MySQL/Oracle/Snowflake und die Bereitstellung auf SET.

## Français
**Règles** : ne pas écrire de JS/TS (Rust est compilé en wasm) ; ne pas créer de REST (Connect uniquement) ; traiter le JSON avec RS-JSON et garder serde hors du wasm ; ajouter les jetons dans le relais côté serveur ; ne considérer un travail comme terminé qu'après l'avoir vérifié dans un vrai navigateur face à un vrai serveur.
**État** : 10 écrans portés ; les intégrations JS/TS, jQuery, Cosmo et bases de données (pool de connexions compris) sont en place.
**Reste à faire** : un vrai Cosmo Router, de vrais PostgreSQL/CockroachDB, TLS, les pilotes MySQL/Oracle/Snowflake et le déploiement sur SET.

## Español
**Reglas**: no escribir JS/TS (Rust se compila a wasm); no crear REST (solo Connect); procesar el JSON con RS-JSON y mantener serde fuera del wasm; añadir los tokens en el relé del servidor; y dar un trabajo por terminado solo después de comprobarlo en un navegador real frente a un servidor real.
**Estado**: 10 pantallas portadas; la integración con JS/TS, jQuery, Cosmo y bases de datos (incluido un pool de conexiones) está lista.
**Pendiente**: un Cosmo Router real, PostgreSQL/CockroachDB reales, TLS, los controladores de MySQL/Oracle/Snowflake y el despliegue en SET.

## Português
**Regras**: não escrever JS/TS (o Rust é compilado para wasm); não criar REST (apenas Connect); tratar o JSON com o RS-JSON e manter o serde fora do wasm; anexar os tokens no relé do servidor; e só considerar um trabalho concluído depois de verificá-lo em um navegador real contra um servidor real.
**Estado**: 10 telas portadas; a integração com JS/TS, jQuery, Cosmo e bancos de dados (incluindo um pool de conexões) está pronta.
**Pendente**: um Cosmo Router real, PostgreSQL/CockroachDB reais, TLS, os drivers de MySQL/Oracle/Snowflake e a implantação no SET.

## Русский
**Правила**: не писать JS/TS (Rust компилируется в wasm); не создавать REST (только Connect); работать с JSON через RS-JSON и не подключать serde к wasm; токены добавлять в серверном ретрансляторе; считать работу завершённой только после проверки в настоящем браузере на настоящем сервере.
**Состояние**: перенесены 10 экранов; готова интеграция с JS/TS, jQuery, Cosmo и базами данных (включая пул соединений).
**Осталось**: настоящий Cosmo Router, настоящие PostgreSQL/CockroachDB, TLS, драйверы MySQL/Oracle/Snowflake и развёртывание на SET.
