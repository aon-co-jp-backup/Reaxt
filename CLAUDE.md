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

> AI翻訳(日英以外はネイティブ検証前)。/ AI translations (other than ja/en not yet native-checked).

## 日本語
**ルール**: JS/TSを書かない(Rust→wasm)、RESTを作らない(Connectのみ)、JSONはRS-JSON(wasmにserdeを入れない)、トークンはサーバー側の中継で付ける、実ブラウザ・実サーバーで確認してから完了と報告する。**到達点**: 10画面移植・JS/TS/jQuery/Cosmo/DB連携まで。**未了**: 実Cosmo Router・実PostgreSQL/CockroachDB・TLS・MySQL/Oracle/Snowflakeドライバ・SET配備。

## English
**Rules**: write no JS/TS (Rust to wasm), build no REST (Connect only), JSON through RS-JSON (no serde in wasm), attach tokens in the server-side relay, and report done only after checking in a real browser and against a real server. **Status**: 10 screens ported; JS/TS, jQuery, Cosmo and DB integration in place. **Open**: real Cosmo Router, real PostgreSQL/CockroachDB, TLS, MySQL/Oracle/Snowflake drivers, SET deployment.

## 简体中文
**规则**:不写 JS/TS(Rust 编译为 wasm),不做 REST(仅 Connect),JSON 使用 RS-JSON(wasm 中不引入 serde),令牌由服务端中继附加,在真实浏览器和真实服务器上验证后才报告完成。**进展**:已移植 10 个页面,已具备 JS/TS、jQuery、Cosmo、数据库集成。**未完成**:真实 Cosmo Router、真实 PostgreSQL/CockroachDB、TLS、MySQL/Oracle/Snowflake 驱动、SET 部署。

## 繁體中文
**規則**:不寫 JS/TS(Rust 編譯為 wasm),不做 REST(僅 Connect),JSON 使用 RS-JSON(wasm 中不引入 serde),權杖由伺服器端中繼附加,在真實瀏覽器與真實伺服器上驗證後才回報完成。**進度**:已移植 10 個頁面,已具備 JS/TS、jQuery、Cosmo、資料庫整合。**未完成**:真實 Cosmo Router、真實 PostgreSQL/CockroachDB、TLS、MySQL/Oracle/Snowflake 驅動、SET 部署。

## 한국어
**규칙**: JS/TS를 작성하지 않는다(Rust→wasm), REST를 만들지 않는다(Connect만), JSON은 RS-JSON으로 처리(wasm에 serde 금지), 토큰은 서버 측 중계에서 붙인다, 실제 브라우저와 실제 서버에서 확인한 뒤에만 완료를 보고한다. **현황**: 10개 화면 이식, JS/TS·jQuery·Cosmo·DB 연동 완료. **미완료**: 실제 Cosmo Router, 실제 PostgreSQL/CockroachDB, TLS, MySQL/Oracle/Snowflake 드라이버, SET 배포.

## Deutsch
**Regeln**: kein JS/TS schreiben (Rust nach wasm), kein REST bauen (nur Connect), JSON über RS-JSON (kein serde in wasm), Tokens im serverseitigen Relay anfügen, Fertigstellung erst nach Prüfung in einem echten Browser und an einem echten Server melden. **Stand**: 10 Bildschirme portiert; JS/TS-, jQuery-, Cosmo- und DB-Anbindung vorhanden. **Offen**: echter Cosmo Router, echtes PostgreSQL/CockroachDB, TLS, Treiber für MySQL/Oracle/Snowflake, Betrieb auf SET.

## Français
**Règles** : n'écrire ni JS ni TS (Rust vers wasm), ne pas créer de REST (Connect uniquement), JSON via RS-JSON (pas de serde dans le wasm), ajouter les jetons dans le relais côté serveur, ne déclarer terminé qu'après vérification dans un vrai navigateur et sur un vrai serveur. **État** : 10 écrans portés ; intégrations JS/TS, jQuery, Cosmo et base de données en place. **Reste** : vrai Cosmo Router, vrais PostgreSQL/CockroachDB, TLS, pilotes MySQL/Oracle/Snowflake, déploiement sur SET.

## Español
**Reglas**: no escribir JS/TS (Rust a wasm), no crear REST (solo Connect), JSON mediante RS-JSON (sin serde en wasm), añadir los tokens en el relé del servidor, y dar algo por terminado solo tras comprobarlo en un navegador real y contra un servidor real. **Estado**: 10 pantallas portadas; integración con JS/TS, jQuery, Cosmo y bases de datos lista. **Pendiente**: Cosmo Router real, PostgreSQL/CockroachDB reales, TLS, controladores de MySQL/Oracle/Snowflake, despliegue en SET.

## Português
**Regras**: não escrever JS/TS (Rust para wasm), não criar REST (somente Connect), JSON via RS-JSON (sem serde no wasm), anexar os tokens no relé do servidor e só dar como concluído após verificar em um navegador real e em um servidor real. **Estado**: 10 telas portadas; integração com JS/TS, jQuery, Cosmo e bancos de dados pronta. **Pendente**: Cosmo Router real, PostgreSQL/CockroachDB reais, TLS, drivers de MySQL/Oracle/Snowflake, implantação no SET.

## Русский
**Правила**: не писать JS/TS (Rust в wasm), не создавать REST (только Connect), JSON через RS-JSON (без serde в wasm), токены добавляются в серверном ретрансляторе, о завершении сообщать только после проверки в настоящем браузере и на настоящем сервере. **Состояние**: перенесены 10 экранов; интеграции с JS/TS, jQuery, Cosmo и базами данных готовы. **Осталось**: настоящий Cosmo Router, настоящие PostgreSQL/CockroachDB, TLS, драйверы MySQL/Oracle/Snowflake, развёртывание на SET.
