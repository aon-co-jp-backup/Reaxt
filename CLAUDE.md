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

**追加(2026-11-19)**: フック(`use_ref`・`use_memo`・`use_reducer`・`use_effect` ほか)、コンテキスト、`memo`、Fragment、Suspense、エラー境界、Solid型シグナル(`batch`込み)、Vue風・Preact風の呼び名、`wasm::portal`。ストリーミングSSRは**本物の非同期**: シェルを待たずに先に送り、各境界を並行に走らせて終わった順に差し替え、期限を超えたら代替表示に切り替える(`App::stream_async`、`reaxt-rpoem`が実HTTPで送出。実ブラウザでシェル15ms・A 1023ms・B 2015msの順に表示されるのを確認)。同期ホストは501を返す。未確認: クライアント切断時の中止の自動テスト、Preactの`Component`クラス(非対応)。

## English
**Rules**: write no JS/TS (Rust compiles to wasm); build no REST (Connect only); handle JSON with RS-JSON and keep serde out of wasm; attach tokens in the server-side relay; and report work as done only after checking it in a real browser against a real server.
**Status**: 10 screens ported, with JS/TS, jQuery, Cosmo and database integration (including a connection pool) in place.
**Open**: a real Cosmo Router, real PostgreSQL/CockroachDB, TLS, MySQL/Oracle/Snowflake drivers and deployment on SET.

**Added (2026-11-19)**: hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` and more), context, `memo`, Fragment, Suspense, error boundaries, Solid-style signals (with `batch`), Vue- and Preact-style names, and `wasm::portal`. Streaming SSR is now **truly asynchronous**: the shell goes out immediately without waiting, every boundary runs concurrently and is swapped in as it finishes, and a boundary that exceeds its deadline falls back to a substitute (`App::stream_async`, sent over real HTTP by `reaxt-rpoem`; in a real browser the shell showed at 15 ms, A at 1023 ms and B at 2015 ms). A synchronous host answers 501. Not yet verified: an automated test for aborting on client disconnect; Preact's `Component` class is not supported.

## 简体中文
**规则**:不写 JS/TS(由 Rust 编译为 wasm);不做 REST(只用 Connect);JSON 统一使用 RS-JSON,wasm 中不引入 serde;令牌由服务端中继附加;必须在真实浏览器和真实服务器上验证通过,才能报告完成。
**进展**:已迁移 10 个页面,并完成 JS/TS、jQuery、Cosmo 和数据库(含连接池)的集成。
**待完成**:真实的 Cosmo Router、真实的 PostgreSQL/CockroachDB、TLS、MySQL/Oracle/Snowflake 驱动、部署到 SET。

**新增(2026-11-19)**:钩子(`use_ref`、`use_memo`、`use_reducer`、`use_effect` 等)、上下文、`memo`、Fragment、Suspense、错误边界、Solid 风格的信号(含 `batch`)、Vue 和 Preact 风格的命名,以及 `wasm::portal`。流式 SSR 现在是**真正的异步**:外壳不等待、立即发出,各个边界并发执行并按完成顺序替换,超过期限则切换为备用内容(`App::stream_async`,由 `reaxt-rpoem` 通过真实 HTTP 发送;在真实浏览器中,外壳 15 毫秒、A 1023 毫秒、B 2015 毫秒依次显示)。同步宿主返回 501。尚未验证:客户端断开时中止的自动化测试;不支持 Preact 的 `Component` 类。

## 繁體中文
**規則**:不撰寫 JS/TS(由 Rust 編譯為 wasm);不做 REST(僅使用 Connect);JSON 一律交由 RS-JSON 處理,wasm 中不引入 serde;權杖由伺服器端中繼附加;必須在真實瀏覽器與真實伺服器上驗證無誤,才能回報完成。
**進度**:已移植 10 個頁面,並完成 JS/TS、jQuery、Cosmo 與資料庫(含連線池)的整合。
**待完成**:真實的 Cosmo Router、真實的 PostgreSQL/CockroachDB、TLS、MySQL/Oracle/Snowflake 驅動程式、部署到 SET。

**新增(2026-11-19)**:Hook(`use_ref`、`use_memo`、`use_reducer`、`use_effect` 等)、Context、`memo`、Fragment、Suspense、錯誤邊界、Solid 風格的 Signal(含 `batch`)、Vue 與 Preact 風格的命名,以及 `wasm::portal`。串流 SSR 現已是**真正的非同步**:外殼不等待、立即送出,各個邊界並行執行並依完成順序替換,超過期限則改為備用內容(`App::stream_async`,由 `reaxt-rpoem` 透過真實 HTTP 送出;在真實瀏覽器中,外殼 15 毫秒、A 1023 毫秒、B 2015 毫秒依序顯示)。同步主機會回傳 501。尚未驗證:用戶端中斷連線時中止的自動化測試;不支援 Preact 的 `Component` 類別。

## 한국어
**규칙**: JS/TS는 작성하지 않습니다(Rust를 wasm으로 컴파일). REST는 만들지 않습니다(Connect만 사용). JSON은 RS-JSON으로 처리하며 wasm에는 serde를 넣지 않습니다. 토큰은 서버 측 중계에서 붙입니다. 실제 브라우저와 실제 서버에서 확인한 뒤에만 완료로 보고합니다.
**현황**: 10개 화면 이식을 마쳤고, JS/TS·jQuery·Cosmo·데이터베이스(커넥션 풀 포함) 연동까지 갖추었습니다.
**남은 작업**: 실제 Cosmo Router, 실제 PostgreSQL/CockroachDB, TLS, MySQL/Oracle/Snowflake 드라이버, SET 배포.

**추가(2026-11-19)**: 훅(`use_ref`, `use_memo`, `use_reducer`, `use_effect` 등), 컨텍스트, `memo`, Fragment, Suspense, 에러 경계, Solid 방식의 시그널(`batch` 포함), Vue·Preact 방식의 이름, 그리고 `wasm::portal`을 갖추었습니다. 스트리밍 SSR은 이제 **진짜 비동기**입니다. 셸은 기다리지 않고 먼저 내보내고, 각 경계는 동시에 실행되어 끝나는 순서대로 교체되며, 기한을 넘기면 대체 내용으로 바뀝니다(`App::stream_async`, `reaxt-rpoem`이 실제 HTTP로 전송. 실제 브라우저에서 셸 15ms, A 1023ms, B 2015ms 순으로 표시되는 것을 확인). 동기 호스트는 501을 반환합니다. 아직 확인하지 못한 것: 클라이언트 연결이 끊겼을 때 중단하는 자동 테스트, Preact의 `Component` 클래스(미지원).

## Deutsch
**Regeln**: kein JS/TS schreiben (Rust wird zu wasm kompiliert); kein REST bauen (nur Connect); JSON über RS-JSON verarbeiten und serde aus dem wasm heraushalten; Tokens im serverseitigen Relay anfügen; eine Aufgabe erst als erledigt melden, wenn sie in einem echten Browser gegen einen echten Server geprüft wurde.
**Stand**: 10 Bildschirme portiert; die Anbindung von JS/TS, jQuery, Cosmo und Datenbanken (einschließlich Verbindungspool) steht.
**Offen**: ein echter Cosmo Router, echtes PostgreSQL/CockroachDB, TLS, Treiber für MySQL/Oracle/Snowflake und die Bereitstellung auf SET.

**Neu (2026-11-19)**: Hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` u. a.), Context, `memo`, Fragment, Suspense, Error Boundaries, Signale nach Solid-Art (mit `batch`), Bezeichnungen im Stil von Vue und Preact sowie `wasm::portal`. Streaming-SSR ist jetzt **wirklich asynchron**: Die Shell wird sofort ausgeliefert, ohne zu warten; alle Grenzen laufen parallel und werden in der Reihenfolge ihrer Fertigstellung eingesetzt; wer die Frist überschreitet, wird durch einen Ersatzinhalt ersetzt (`App::stream_async`, von `reaxt-rpoem` über echtes HTTP gesendet; im echten Browser erschien die Shell nach 15 ms, A nach 1023 ms und B nach 2015 ms). Ein synchroner Host antwortet mit 501. Noch nicht geprüft: ein automatischer Test für den Abbruch bei Verbindungsabbruch des Clients; die `Component`-Klasse von Preact wird nicht unterstützt.

## Français
**Règles** : ne pas écrire de JS/TS (Rust est compilé en wasm) ; ne pas créer de REST (Connect uniquement) ; traiter le JSON avec RS-JSON et garder serde hors du wasm ; ajouter les jetons dans le relais côté serveur ; ne considérer un travail comme terminé qu'après l'avoir vérifié dans un vrai navigateur face à un vrai serveur.
**État** : 10 écrans portés ; les intégrations JS/TS, jQuery, Cosmo et bases de données (pool de connexions compris) sont en place.
**Reste à faire** : un vrai Cosmo Router, de vrais PostgreSQL/CockroachDB, TLS, les pilotes MySQL/Oracle/Snowflake et le déploiement sur SET.

**Ajouts (2026-11-19)** : hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect`, etc.), contexte, `memo`, Fragment, Suspense, limites d'erreur, signaux à la Solid (avec `batch`), noms dans le style de Vue et de Preact, et `wasm::portal`. Le SSR en streaming est désormais **réellement asynchrone** : la coquille part immédiatement sans attendre, chaque frontière s'exécute en parallèle et se substitue dans l'ordre d'achèvement, et une frontière qui dépasse son délai est remplacée par un contenu de repli (`App::stream_async`, envoyé en vrai HTTP par `reaxt-rpoem` ; dans un vrai navigateur, la coquille s'est affichée à 15 ms, A à 1023 ms et B à 2015 ms). Un hôte synchrone répond 501. Pas encore vérifié : un test automatique de l'abandon lors d'une déconnexion du client ; la classe `Component` de Preact n'est pas prise en charge.

## Español
**Reglas**: no escribir JS/TS (Rust se compila a wasm); no crear REST (solo Connect); procesar el JSON con RS-JSON y mantener serde fuera del wasm; añadir los tokens en el relé del servidor; y dar un trabajo por terminado solo después de comprobarlo en un navegador real frente a un servidor real.
**Estado**: 10 pantallas portadas; la integración con JS/TS, jQuery, Cosmo y bases de datos (incluido un pool de conexiones) está lista.
**Pendiente**: un Cosmo Router real, PostgreSQL/CockroachDB reales, TLS, los controladores de MySQL/Oracle/Snowflake y el despliegue en SET.

**Novedades (2026-11-19)**: hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` y más), contexto, `memo`, Fragment, Suspense, límites de error, señales al estilo de Solid (con `batch`), nombres al estilo de Vue y Preact, y `wasm::portal`. El SSR en streaming ahora es **realmente asíncrono**: el esqueleto se envía de inmediato sin esperar, cada frontera se ejecuta en paralelo y se sustituye según termina, y si una frontera supera su plazo se cambia por un contenido alternativo (`App::stream_async`, enviado por HTTP real mediante `reaxt-rpoem`; en un navegador real el esqueleto apareció a los 15 ms, A a los 1023 ms y B a los 2015 ms). Un host síncrono responde 501. Aún sin verificar: una prueba automática de la cancelación cuando el cliente se desconecta; la clase `Component` de Preact no es compatible.

## Português
**Regras**: não escrever JS/TS (o Rust é compilado para wasm); não criar REST (apenas Connect); tratar o JSON com o RS-JSON e manter o serde fora do wasm; anexar os tokens no relé do servidor; e só considerar um trabalho concluído depois de verificá-lo em um navegador real contra um servidor real.
**Estado**: 10 telas portadas; a integração com JS/TS, jQuery, Cosmo e bancos de dados (incluindo um pool de conexões) está pronta.
**Pendente**: um Cosmo Router real, PostgreSQL/CockroachDB reais, TLS, os drivers de MySQL/Oracle/Snowflake e a implantação no SET.

**Novidades (2026-11-19)**: hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` e outros), contexto, `memo`, Fragment, Suspense, limites de erro, sinais no estilo Solid (com `batch`), nomes no estilo Vue e Preact, e `wasm::portal`. O SSR com streaming agora é **realmente assíncrono**: o esqueleto é enviado imediatamente, sem esperar; cada fronteira roda em paralelo e é substituída na ordem em que termina, e uma fronteira que ultrapassa o prazo é trocada por um conteúdo alternativo (`App::stream_async`, enviado por HTTP real pelo `reaxt-rpoem`; em um navegador real, o esqueleto apareceu aos 15 ms, o A aos 1023 ms e o B aos 2015 ms). Um host síncrono responde 501. Ainda não verificado: um teste automático do cancelamento quando o cliente se desconecta; a classe `Component` do Preact não é compatível.

## Русский
**Правила**: не писать JS/TS (Rust компилируется в wasm); не создавать REST (только Connect); работать с JSON через RS-JSON и не подключать serde к wasm; токены добавлять в серверном ретрансляторе; считать работу завершённой только после проверки в настоящем браузере на настоящем сервере.
**Состояние**: перенесены 10 экранов; готова интеграция с JS/TS, jQuery, Cosmo и базами данных (включая пул соединений).
**Осталось**: настоящий Cosmo Router, настоящие PostgreSQL/CockroachDB, TLS, драйверы MySQL/Oracle/Snowflake и развёртывание на SET.
**Добавлено (2026-11-19)**: хуки (`use_ref`, `use_memo`, `use_reducer`, `use_effect` и другие), контекст, `memo`, Fragment, Suspense, границы ошибок, сигналы в стиле Solid (с `batch`), названия в стиле Vue и Preact, а также `wasm::portal`. Потоковый SSR теперь **по-настоящему асинхронный**: оболочка отправляется сразу, не дожидаясь ничего, каждая граница выполняется параллельно и подставляется по мере готовности, а граница, превысившая срок, заменяется запасным содержимым (`App::stream_async`, отправка по настоящему HTTP через `reaxt-rpoem`; в настоящем браузере оболочка появилась через 15 мс, A — через 1023 мс, B — через 2015 мс). Синхронный хост отвечает 501. Ещё не проверено: автоматический тест прерывания при отключении клиента; класс `Component` из Preact не поддерживается.


## 3姉妹リポジトリ(2026-11-19、常に同時開発)
Reaxt / [`RPoRea`](https://github.com/aon-co-jp/RPoRea)(RPoem+Reaxtの融合、新設)/ [`open-cosmo`](https://github.com/aon-co-jp/open-cosmo)(WunderGraph Cosmo相当。GitHub上の正式名称は`open-cosmo`と確認済み)。接点(サブグラフ形式・manifest・Connect・`x-reaxt`)を変えたら3つのCLAUDE.mdを同じ日に更新する。約束と接点の現状は RPoRea の `docs/THREE-SISTERS.md`。最優先の未了は、open-cosmo のルーターへ Reaxt のサブグラフを実際に合成させる確認。

## 2026-11-19 前提(ユーザー指示): 開発中の Rust 版 HTML5+CSS3+TypeScript と一緒に使う想定
- RPoem・Reaxt・RPoRea は基本 **Rust から**使い、Web サイト・Web アプリ・アプリを作る。開発中の **Rust 版 HTML5+CSS3+TypeScript**(`rhtml5`・`rcss3`・`RS-TypeScript`、JS 実行エンジン `RS-JavaScript` は雛形)と一緒に使う想定で設計・実装する。
- 方針: HTML5/CSS3 の使用量は極力少なく / tokio と WASM は適材適所 / REST は使わない(Connect・gRPC) / JavaScript の Rust 版も進めるが **TypeScript の Rust 版を優先**。
- 部品ごとの現状とつなぎ方の想定は `RPoRea/docs/WEB-STACK.md`(2026-11-19 調査: `RS-TypeScript` は本日拡張し Node で実行確認、`rhtml5` はトークナイザのみ、`rcss3` はパーサー+カスケード、`RS-JavaScript` は雛形)。
- README/CLAUDE/PORTING は更新のたびに 10 か国語で(ネイティブに近い自然さ、IT・Web に詳しい人が一般の人にも親切に)。この追記の10言語版は `RPoRea` の各文書にある。


## 2026-11-19 追記(3姉妹の同時更新): 複数サブグラフの問い合わせ分割と `@key` 結合 / Query splitting and `@key` joins

**日本語(やさしい説明)**: open-cosmo のゲートウェイが、複数のサブグラフにまたがる問い合わせを扱えるようになりました。(1) 読み取り専用の問い合わせで、ルートのフィールドが別々のサブグラフにあるときは、サブグラフごとに分けて同時に実行し、結果を1つにまとめます。(2) `@key` つきの型(エンティティ)は、サブグラフAが返した「id」を使って、別のサブグラフBの `_entities` に問い合わせ、足りないフィールドを足します(1段のみ)。変更系(mutation)をまたぐ問い合わせ、リモートとローカルの混在、フラグメント/ディレクティブつきは、従来どおり拒否します。試験はゲートウェイ44件・federation13件が成功(擬似サブグラフでの確認。Reaxt実機を複数並べた通しは未了)。Reaxt のサブグラフ(`_service{sdl}`・`__schema`)はそのまま対応しており、Reaxt側の変更は不要でした。
**English (simple)**: The open-cosmo gateway now handles queries that span several subgraphs. (1) A read-only query whose root fields live in different subgraphs is split, run at the same time, and merged. (2) For a type with `@key` (an entity), the gateway takes the id returned by subgraph A and asks subgraph B through `_entities` for the missing fields (one level only). Mutations across subgraphs, a mix of remote and local fields, and fragments/directives are still rejected. Tests pass (gateway 44, federation 13) against fake subgraphs; a run with several real Reaxt servers is not done yet. Reaxt's subgraph (`_service{sdl}`, `__schema`) already works, so Reaxt needed no change.
**未了 / Open**: 複数段の入れ子の結合・`@requires`/`@provides`・実Reaxtを複数並べた通し・ミューテーション後の再取得。Nested (multi-level) joins, `@requires`/`@provides`, a real multi-Reaxt run.

## 2026-11-19 追記: 実サーバーでの振り分けを確認(3姉妹共通) / Routing verified on real servers (shared by the three sisters)

### 日本語
`Reaxt` に2つ目のデモサーバー `examples/counter/src/bin/serve_greeter.rs`(GraphQL のフィールド名 `demo_v1_Greeter_Visit`)を追加し、カウンターのサーバー(`demo_v1_Counter_Inc`)と2台並べて、`open-cosmo` のゲートウェイ経由で実機確認しました。結果は次のとおりです。

- カウンターの変更はカウンターのサーバーへ、グリーターの変更はグリーターのサーバーへ、それぞれ正しく届き、数値は別々に増えました(カウンター 1→2、グリーター 1)。
- 2台にまたがる変更系(mutation)は、仕様どおり拒否されました。
- 注意: Reaxt のサブグラフは、引数を `input: $変数` の形でしか受け付けず、値は `null`(入力なし)が必要です。リテラルの引数は未対応です。
- 読み取り専用の問い合わせを2台にまたがって分割する経路は、Reaxt のサンプルが読み取りフィールドを `reaxt_ready`(共有)しか持たないため、実機では未確認です。擬似サブグラフでの試験は成功しています。

### English
We added a second Reaxt demo server, `examples/counter/src/bin/serve_greeter.rs` (GraphQL field `demo_v1_Greeter_Visit`), ran it next to the counter server (`demo_v1_Counter_Inc`), and checked both through the `open-cosmo` gateway on a real machine.

- A counter mutation reached the counter server and a greeter mutation reached the greeter server, and each counter advanced on its own (counter 1 then 2, greeter 1).
- A mutation spanning both servers was rejected, as designed.
- Note: a Reaxt subgraph accepts arguments only in the form `input: $variable`, and the value must be `null` (no input). Literal arguments are not supported yet.
- The read-only split across two servers was not checked on real servers, because the Reaxt examples expose only one read field, `reaxt_ready`, which both share. Tests against fake subgraphs pass.

**未了 / Open**: Reaxt のサブグラフの引数リテラル対応、読み取りフィールドを持つ実サンプルでの分割の通し確認、複数段の結合、`@requires` / `@provides`。 Literal arguments in the Reaxt subgraph, a real split run with an example that has read fields, multi-level joins, `@requires` / `@provides`.

## 2026-11-19 追記(続き): Reaxt のサブグラフが引数リテラルに対応(3姉妹共通) / Literal arguments in the Reaxt subgraph (shared by the three sisters)

### 日本語
`Reaxt` のサブグラフ(`reaxt-actions/src/subgraph.rs`)が、`input: $変数` に加えて **`input:` のリテラル**を受け付けるようになりました。オブジェクト・配列・文字列・数値・真偽値・`null`・列挙値、そして入れ子の中の `$変数` が使えます(例: `input: {by: 1, tags: ["x"], n: $n}`)。列挙値は文字列として渡します。壊れたリテラルはそのフィールドだけ `invalid_argument` のエラーになり、他のフィールドには影響しません。ブロック文字列(`"""`)と `input` 以外の引数名は未対応です。`Reaxt` の試験(`reaxt-actions` 10件)に成功しました。前の追記にある「リテラルは未対応」は、この更新で解消しています。

### English
The Reaxt subgraph (`reaxt-actions/src/subgraph.rs`) now accepts **literal values for `input:`** as well as `input: $variable`. Objects, lists, strings, numbers, booleans, `null`, enum values, and `$variable` references nested inside them all work (for example `input: {by: 1, tags: ["x"], n: $n}`). Enum values are passed as strings. A malformed literal fails only its own field with `invalid_argument` and leaves the other fields alone. Block strings (`"""`) and argument names other than `input` are not supported. The Reaxt tests pass (`reaxt-actions`, 10 tests). The "literals are not supported" remark in the previous note is resolved by this update.

**未了 / Open**: 読み取りフィールドを持つ実サンプルでの分割の通し確認、複数段の結合、`@requires` / `@provides`。 A real split run with an example that has read fields, multi-level joins, `@requires` / `@provides`.

## 2026-11-19 追記(続き2): `@key` 結合が入れ子に対応(3姉妹共通) / `@key` joins now work at any depth (shared by the three sisters)

### 日本語
`open-cosmo` のゲートウェイの `@key` 結合が、1段だけでなく**入れ子の段にも**対応しました。たとえば `{ me { name reviews manager { name reviews } } }` のように、`me` の中の `manager`(同じ型 `User`)でも、別のサブグラフ(reviews)にしかない `reviews` を取れます。浅い段から順に結合し、結合のために足した `id` と `__typename` は、最後に結果から取り除きます。試験はゲートウェイ全体で45件に成功しました(擬似サブグラフ)。**未了**: `@requires` / `@provides`、`@key` が複数のフィールドにまたがる入れ子の鍵、変更系の後の再取得、読み取りフィールドを持つ実際の Reaxt サンプルでの通し確認。

### English
The `@key` join in the `open-cosmo` gateway now works at **any depth of nesting**, not only one level. For example, in `{ me { name reviews manager { name reviews } } }` the `manager` inside `me` (also a `User`) can get `reviews`, which only another subgraph (reviews) defines. Joins run from the shallowest level down, and the `id` and `__typename` that were added only for the join are removed from the final result. Tests pass across the gateway (45 tests, fake subgraphs). **Open**: `@requires` / `@provides`, nested composite keys, re-fetching after a mutation, and a real run with a Reaxt example that has read fields.

## 2026-11-19 追記: 4姉妹プロジェクト(`open-cosmo`・`RPoem`・`Reaxt`・`RPoRea`)を同時に開発 / The four sister projects are developed together

### 日本語
これまでの「3姉妹」(`Reaxt`・`RPoRea`・`open-cosmo`)に、土台である **`RPoem`** を加えた **4姉妹**を、常に同時に開発します(ユーザー指示)。役割は次のとおりです。

- **`RPoem`**: 土台(アプリケーションサーバー、Tomcat 相当。管理 API は Connect / gRPC)。
- **`Reaxt`**: Rust で書く画面とサーバーアクション。Cosmo 向けのサブグラフ入口 `POST /_reaxt/graphql` を持つ。
- **`RPoRea`**: `RPoem` と `Reaxt` を合わせた土台。Rust 以外からは C ABI(`rporea-ffi`)で使う。
- **`open-cosmo`**: 複数のサブグラフをまとめるゲートウェイ(WunderGraph Cosmo 相当)。

**決まり**: どれかの接点(Cosmo のサブグラフ形式・manifest・Connect・`x-reaxt`・管理 API)を変えたら、4つの README・CLAUDE・PORTING・POLICY を同じ日に、日本語とネイティブに近い英語で更新する。最新の到達点: ゲートウェイの複数サブグラフ分割、`@key` 結合(入れ子の段にも対応)、Reaxt の `input:` リテラル対応(いずれも push 済み)。未了: `@requires` / `@provides`、変更系の後の再取得、読み取りフィールドを持つ実際の Reaxt サンプルでの通し確認。

### English
The former "three sisters" (`Reaxt`, `RPoRea`, `open-cosmo`) now include their foundation, **`RPoem`**, making **four sister projects** that are always developed together (user instruction). Their roles:

- **`RPoem`**: the foundation (an application server, comparable to Tomcat; its admin API uses Connect / gRPC).
- **`Reaxt`**: UI and server actions written in Rust, with a Cosmo-compatible subgraph entry, `POST /_reaxt/graphql`.
- **`RPoRea`**: the base that joins `RPoem` and `Reaxt`; callers other than Rust use its C ABI (`rporea-ffi`).
- **`open-cosmo`**: the gateway that combines several subgraphs (comparable to WunderGraph Cosmo).

**Rule**: when an interface between them changes (the Cosmo subgraph format, the manifest, Connect, `x-reaxt`, the admin API), update the README, CLAUDE, PORTING and POLICY of all four on the same day, in Japanese and natural English. Latest state: the gateway splits queries across subgraphs and supports `@key` joins (including nested levels), and Reaxt accepts `input:` literals (all pushed). Open: `@requires` / `@provides`, re-fetching after a mutation, and a real run with a Reaxt example that has read fields.

## 2026-11-19 追記(続き3): 実サーバー2台での「読み取りの分割」を確認、Reaxt に読み取り用アクションを追加(4姉妹共通) / Read-query split verified on two real servers; read actions added to Reaxt (shared by the four sisters)

### 日本語
- **Reaxt の規約**: メソッド名が `Get` / `List` / `Read` で始まるアクション(例: `GetCount`、`GetVisits`)は、Cosmo 向けのサブグラフで GraphQL の **Query** として公開されます。それ以外は従来どおり **Mutation** です。変更用のアクションが1つも無いときは、SDL に `Mutation` 型を出しません。`reaxt_ready` は引き続き全サブグラフで共有されます。
- **実機での確認**: カウンター(`GetCount`)とグリーター(`GetVisits`)の2台を `open-cosmo` のゲートウェイ越しに動かし、`query { c: demo_v1_Counter_GetCount g: demo_v1_Greeter_GetVisits }` が、2台に分けて実行され、`{"data":{"c":2,"g":1}}` と1つの結果にまとまることを確認しました。2台にまたがる変更系は、従来どおり「読み取り専用の問い合わせだけが分割できる」というメッセージで拒否されます。`input:` のリテラルも実機で通りました。
- **試験**: Reaxt の `reaxt-actions` は11件に成功。
- **未了**: `@requires` / `@provides`、変更系の後の再取得。

### English
- **Reaxt convention**: actions whose method name starts with `Get`, `List` or `Read` (for example `GetCount`, `GetVisits`) are exposed by the Cosmo subgraph as GraphQL **Query** fields; all other actions remain **Mutation** fields. When there are no mutating actions, the SDL has no `Mutation` type. `reaxt_ready` is still shared by every subgraph.
- **Verified on a real machine**: with the counter (`GetCount`) and greeter (`GetVisits`) servers running behind the `open-cosmo` gateway, `query { c: demo_v1_Counter_GetCount g: demo_v1_Greeter_GetVisits }` was split across the two servers and merged into one answer, `{"data":{"c":2,"g":1}}`. A mutation spanning both is still rejected, now with the message that only read-only queries can be split. `input:` literals also worked on the real servers.
- **Tests**: `reaxt-actions` passes 11 tests.
- **Open**: `@requires` / `@provides`, and re-fetching after a mutation.

## 2026-11-19 追記(続き4): `@external`・`@provides`・`@requires` に対応(4姉妹共通) / `@external`, `@provides` and `@requires` supported (shared by the four sisters)

### 日本語
`open-cosmo` のゲートウェイの `@key` 結合が、Apollo Federation の残りの主なディレクティブにも対応しました。

- **`@external`**: サブグラフAが `@external` と宣言したフィールドは、Aでは解決できないものとして扱い、実際に持つサブグラフBへの結合(`_entities`)の対象にします。
- **`@provides`**: 親のフィールドが `@provides(fields: "...")` で供給すると宣言したフィールドだけは、`@external` でもAに任せ、結合しません(余計な往復を減らします)。
- **`@requires`**: サブグラフBのフィールドが `@requires(fields: "a b")` で他のフィールドを必要とするとき、(1) そのフィールドが選ばれたときだけ、`a` と `b` をサブグラフAの問い合わせに足して先に取得し、(2) エンティティ表現(representation)に鍵と一緒に入れて B の `_entities` へ送ります。クライアントが選んでいない `a` と `b` は、最後に結果から取り除きます。Aが必須フィールドを供給できないときは、結合せずに従来どおり転送します。
- **設計の根拠(調査)**: Apollo の [Subgraph 仕様](https://www.apollographql.com/docs/graphos/reference/federation/subgraph-spec/)(`_entities` は表現と同じ順で返す、`@requires` のフィールドは表現に含める)と、[apollographql/router](https://github.com/apollographql/router) の計画器(必須フィールドは、要求したフィールドが選ばれたときだけ取得する修正、`FetchNode` の `requires`)を参考にしました。コードは流用せず、Rust で一から実装しています。
- **試験**: ゲートウェイ47件・federation 15件に成功(擬似サブグラフ)。
- **未了**: 必須フィールドが「さらに別のサブグラフ」にある場合(3者以上の連鎖)、`@skip` / `@include` つきの条件、変更系の後の再取得。

### English
The `@key` join in the `open-cosmo` gateway now also handles the other main Apollo Federation directives.

- **`@external`**: a field that subgraph A marks `@external` is treated as not resolvable by A, and is joined (through `_entities`) from the subgraph B that really owns it.
- **`@provides`**: a field that the parent declares as supplied by `@provides(fields: "...")` stays with A even though it is `@external`, so no extra round trip is made.
- **`@requires`**: when a field of subgraph B uses `@requires(fields: "a b")`, the gateway (1) adds `a` and `b` to the query sent to subgraph A, only when that field is selected, and (2) sends them in the entity representation together with the key to B's `_entities`. Fields the client did not select are removed from the final result. If A cannot supply a required field, the query is forwarded as before without a join.
- **Design sources (research)**: the Apollo [Subgraph specification](https://www.apollographql.com/docs/graphos/reference/federation/subgraph-spec/) (`_entities` returns entities in the order of the representations; `@requires` fields belong in the representation) and the planner in [apollographql/router](https://github.com/apollographql/router) (required fields are fetched only when the requiring field is selected; `FetchNode` has `requires`). No code was copied; this is a fresh Rust implementation.
- **Tests**: gateway 47 and federation 15 pass (fake subgraphs).
- **Open**: required fields that live in yet another subgraph (chains of three or more), `@skip` / `@include` conditions, and re-fetching after a mutation.

## 2026-11-19 追記(続き5): 変更系(mutation)が返したエンティティの結合に対応(4姉妹共通) / Joins on entities returned by a mutation (shared by the four sisters)

### 日本語
`open-cosmo` のゲートウェイが、**変更系(mutation)が返したエンティティ**にも `@key` 結合を行うようになりました。たとえば `mutation { createUser { name reviews } }` で、`createUser` は users サブグラフが実行し、`reviews`(別のサブグラフにしかないフィールド)は、返ってきた `id` を使って reviews サブグラフの `_entities`(読み取り専用)から足します。

- **変更は1回だけ**: mutation は元のサブグラフで1回だけ実行し、結合のための追加の呼び出しは読み取り専用の `_entities` だけです(二重実行しません)。試験で、users 側が `mutation` を1回だけ、reviews 側が `_entities` だけを受けたことを確認しました。
- **従来どおり拒否**: 変更系が複数のサブグラフにまたがる問い合わせは、拒否します(原子的にできないため)。
- **試験**: ゲートウェイ48件に成功(擬似サブグラフ)。
- **未了**: 必須フィールドが別のサブグラフにある3者以上の連鎖、`@skip` / `@include` つきの条件。

### English
The `open-cosmo` gateway now also performs `@key` joins on **entities returned by a mutation**. For example, in `mutation { createUser { name reviews } }` the users subgraph runs `createUser`, and `reviews` (a field only another subgraph defines) is added from the reviews subgraph's read-only `_entities`, using the returned `id`.

- **The mutation runs once**: it is executed once at its own subgraph, and the only extra calls are read-only `_entities` calls (nothing runs twice). The test confirms that the users side saw exactly one `mutation` and the reviews side saw only `_entities`.
- **Still rejected**: mutations that span several subgraphs (they cannot be made atomic).
- **Tests**: the gateway passes 48 tests (fake subgraphs).
- **Open**: chains of three or more subgraphs where a required field lives in yet another subgraph, and `@skip` / `@include` conditions.
