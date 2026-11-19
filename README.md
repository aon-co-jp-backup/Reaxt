# Reaxt

Rust だけで書くフロント+バックエンド統合の枠組み(React / Next.js / Nuxt / Nest.js の良い所を研究して統合する構想)。
RPoem と SET で使う。設計は [DESIGN.md](DESIGN.md)。ルート規約・SSR・ハイドレーション・Islands・Server Actions・フォームまで実装済み(実例=aruaru-admin全10画面)。
引き継ぎは [PORTING.md](PORTING.md)、開発ルールは [CLAUDE.md](CLAUDE.md)。

## はじめかた

```bash
cargo install --path reaxt-cli        # reaxt コマンドを入れる
reaxt doctor                          # 必要な道具の確認(wasm32ターゲット・wasm-bindgen)
reaxt new my-app --path <Reaxtのパス>  # 雛形(省略時はGitHubのReaxtを参照)
cd my-app && reaxt dev                # ビルド+起動+変更の監視 http://127.0.0.1:3000/
```

## 構成

- `src/` — ルート規約・SSR・ハイドレーション・Islands・バックエンド層(DI/ガード/Connect)・`react/`(仮想DOM・hooks)
- `reaxt-rpoem/` — RPoem上で配信するアダプタ。`route(app)`はページ・静的アセット・Connect・外部連携の入口(Cosmoサブグラフ等)・`/_reaxt/manifest.json`を登録、`route_production(app)`はそれにRPoemのトレースと圧縮を重ねた本番向け既定(CORSは必要なら`.with_cors()`)
- `reaxt-actions/` — Server Actions(型付きConnect呼び出し、JSONはRS-JSON)
- `reaxt-cli/` — `reaxt` コマンド
- `examples/counter/` — SSR・Islands・Server Actionsのデモ
- `examples/aruaru-admin/` — aruaru-admin Webの移植(全10画面、実aruaru-serverと実ブラウザで確認)。`ARUARU_GQL=host:port`で上流、`ARUARU_DB_ADMIN_TOKEN`でサーバー側中継のトークン(ブラウザには出ない)。管理操作は許可リスト11メソッドのみ中継。wasmはserde/serde_json不使用(RS-JSON `light` + `Wire`トレイト)
- **Reactの各流儀の機能(2026-11-19)**: `reaxt::react` に、フック(`use_state`・`use_ref`・`use_memo`・`use_reducer`・`use_effect`・`use_layout_effect`・`use_id`・`use_deferred_value`・`use_transition`)、コンテキスト(`provide_context`・`use_context`)、`memo`、Fragment(`VNode::fragment`)、Suspense(`suspense`)、エラー境界(`error_boundary`、`Result`で返す方式でpanicは捕まえない)。Solid型のシグナル(`create_signal`・`create_effect`・`create_memo`・`batch`)、Vue風の呼び名(`vue::ref_`・`computed`・`watch`・`watch_effect`。`reactive`は作らない)。ブラウザ側は `wasm::portal(selector, id, render)`(独立した木を別要素へ描く。コンテキストは引き継がれない)。`stream` はストリーミングSSRのチャンク列(シェル+順不同の差し替え)を作る純関数で、chunkedでの送出はホスト側が必要。
- **確認状況**: ストリーミングSSRの差し替えは `examples/counter` の `stream_demo`(chunkedで400msずつ送出)で実ブラウザ確認済み(A・Bとも差し替わり、`<`は文字のまま、templateとscriptは消える)。マウント直後の副作用による状態更新は、マウント後の初回flushで即反映(`/portal`で初期表示`runs=1`を確認)。シグナル・batch・Portalは `examples/counter` の `/portal` で実ブラウザ確認済み(本体の外の`#modal-root`にモーダルが描かれ、1回のクリックで2つのシグナルをbatch更新→副作用は1回だけ実行→表示に反映、コンソールエラー0)。ストリーミングは `App::stream(path, ..)` で登録でき、`reaxt-rpoem` が実際にchunkedで送出する(`cargo run -p reaxt-rpoem --example stream`でcurlのフレーム分割とブラウザ表示を確認)。**未確認**: 解決を非同期に待つ本物の遅延(今は解決関数を同期で呼び、フレームだけ分けて送る)、Preact互換API。
- 実測(元のReact版比較): 元 JS 208KB(gzip 64KB)に対しReaxt版 wasm 202KB(gzip 84KB、リリース設定で縮小。当初366KB)で転送量は約1.3倍。利点は型の共有とJS/TSゼロ。手順は [PORTING.md](PORTING.md)

```bash
cargo test --workspace
```
ライセンス: Apache-2.0 OR MIT

## 他の言語・ツール・データベースとの連携(2026-11-19)

- **JavaScript / TypeScript**: `clients/js/`(`reaxt-client`、依存なし、型定義つき)。`createClient({baseUrl}).call("pkg.Service", "Method", input)`。公開メソッド一覧は `GET /_reaxt/manifest.json`。
- **jQuery**: `clients/js/reaxt-jquery.js`(`$.reaxt.call(...)`、`$("#btn").reaxt("click", ...)`)。jQuery 3.7.1 の実ブラウザで確認。デモは `examples/counter` の `/_reaxt/jquery-demo.html`。
- **WunderGraph Cosmo**: `reaxt_actions::subgraph::install(app)` で `POST /_reaxt/graphql` に連合(Federation v2)対応のGraphQLサブグラフを足す。Connectメソッドが `pkg_Service_Method(input: JSON): JSON` というMutationになり、`_service { sdl }` でSDLを返す。引数は `input: $変数` の形だけ対応。RouterのヘッダーにCSRF用の `x-reaxt: 1` を伝播させる。
- **データベース**: `reaxt-db`(安全な `?` 束縛・方言・`Db`トレイト・DI用`DbHandle`)。
  | データベース | 状態 |
  |---|---|
  | aruaru-db | GraphQL(`AruaruDb`)と pgwire(`PgDb::aruaru()`、SCRAM-SHA-256)を実サーバーで確認 |
  | PostgreSQL・CockroachDB・YugabyteDB・Redshift | `PgDb`(ワイヤv3、trust/平文/MD5/SCRAM)。**実物のこれらへの接続は未確認**(aruaru-dbのpgwireだけで確認)。TLSは未対応のためTLS必須の接続先はTLS終端のプロキシ経由。接続は最大4本プールして再利用 |
  | MySQL・MariaDB・TiDB・Oracle・SQL Server・Snowflake・SQLite・BigQuery | **束縛の方言のみ**(`Dialect`、単体テスト)。**接続ドライバは未実装**(つながらない) |


---

# 現状と完成予定 / Status and Roadmap (10 languages)

> 2026-11-19。日本語・英語以外は AI 翻訳をネイティブらしい表現に整えたものですが、母語話者による確認はまだです。 / Apart from Japanese and English, these are AI translations polished toward natural phrasing; they have not yet been reviewed by native speakers.

## 日本語
**できていること**: ルート規約、SSR/CSR/Islands、ハイドレーション、Server Actions(Connect)、DI/ガード、フォーム。aruaru-admin 全10画面の移植(wasm 202KB・gzip 84KB、serde 不使用)、`wire_struct!`、JS/TS クライアント、jQuery プラグイン、Cosmo 向け GraphQL サブグラフ(curl で確認)、`reaxt-db`(安全なパラメータ束縛、方言、PostgreSQL 互換ワイヤ+SCRAM、接続プール。aruaru-db は実サーバーで確認済み)。
**完成予定**: 実 Cosmo Router での合成確認、実 PostgreSQL/CockroachDB への接続と TLS、MySQL・Oracle・SQL Server・Snowflake のドライバ、SET への配備、`wasm-opt` と描画速度の実測、enum/任意フィールドに対応した `Wire`、RPoem のワイルドカードルート。

**追加(2026-11-19)**: フック(`use_ref`・`use_memo`・`use_reducer`・`use_effect` ほか)、コンテキスト、`memo`、Fragment、Suspense、エラー境界、Solid型シグナル(`batch`込み)、Vue風・Preact風の呼び名、`wasm::portal`。ストリーミングSSRは**本物の非同期**: シェルを待たずに先に送り、各境界を並行に走らせて終わった順に差し替え、期限を超えたら代替表示に切り替える(`App::stream_async`、`reaxt-rpoem`が実HTTPで送出。実ブラウザでシェル15ms・A 1023ms・B 2015msの順に表示されるのを確認)。同期ホストは501を返す。未確認: クライアント切断時の中止の自動テスト、Preactの`Component`クラス(非対応)。

## English
**What works today**: route conventions, SSR/CSR/Islands, hydration, Server Actions (Connect), DI/guards and forms. All 10 aruaru-admin screens are ported (wasm 202 KB, 84 KB gzipped, no serde), along with `wire_struct!`, a JS/TS client, a jQuery plugin, a GraphQL subgraph for Cosmo (checked with curl) and `reaxt-db` (safe parameter binding, dialects, a PostgreSQL-compatible wire driver with SCRAM, and a connection pool; aruaru-db is verified against a real server).
**Still to come**: composing with a real Cosmo Router, connecting to real PostgreSQL/CockroachDB over TLS, drivers for MySQL, Oracle, SQL Server and Snowflake, deployment on SET, `wasm-opt` plus rendering-speed measurements, `Wire` support for enums and optional fields, and wildcard routes in RPoem.

**Added (2026-11-19)**: hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` and more), context, `memo`, Fragment, Suspense, error boundaries, Solid-style signals (with `batch`), Vue- and Preact-style names, and `wasm::portal`. Streaming SSR is now **truly asynchronous**: the shell goes out immediately without waiting, every boundary runs concurrently and is swapped in as it finishes, and a boundary that exceeds its deadline falls back to a substitute (`App::stream_async`, sent over real HTTP by `reaxt-rpoem`; in a real browser the shell showed at 15 ms, A at 1023 ms and B at 2015 ms). A synchronous host answers 501. Not yet verified: an automated test for aborting on client disconnect; Preact's `Component` class is not supported.

## 简体中文
**目前已实现**:路由约定、SSR/CSR/Islands、水合、Server Actions(Connect)、依赖注入/守卫、表单;aruaru-admin 的 10 个页面已全部迁移(wasm 202KB,gzip 后 84KB,不依赖 serde);`wire_struct!`、JS/TS 客户端、jQuery 插件、面向 Cosmo 的 GraphQL 子图(已用 curl 验证);`reaxt-db`(安全的参数绑定、SQL 方言、兼容 PostgreSQL 的协议驱动及 SCRAM 认证、连接池,其中 aruaru-db 已在真实服务器上验证)。
**后续规划**:与真实的 Cosmo Router 联调、通过 TLS 连接真实的 PostgreSQL/CockroachDB、MySQL/Oracle/SQL Server/Snowflake 驱动、部署到 SET、引入 `wasm-opt` 并实测渲染速度、让 `Wire` 支持枚举和可选字段、RPoem 的通配路由。

**新增(2026-11-19)**:钩子(`use_ref`、`use_memo`、`use_reducer`、`use_effect` 等)、上下文、`memo`、Fragment、Suspense、错误边界、Solid 风格的信号(含 `batch`)、Vue 和 Preact 风格的命名,以及 `wasm::portal`。流式 SSR 现在是**真正的异步**:外壳不等待、立即发出,各个边界并发执行并按完成顺序替换,超过期限则切换为备用内容(`App::stream_async`,由 `reaxt-rpoem` 通过真实 HTTP 发送;在真实浏览器中,外壳 15 毫秒、A 1023 毫秒、B 2015 毫秒依次显示)。同步宿主返回 501。尚未验证:客户端断开时中止的自动化测试;不支持 Preact 的 `Component` 类。

## 繁體中文
**目前已完成**:路由慣例、SSR/CSR/Islands、水合、Server Actions(Connect)、相依注入/守衛、表單;aruaru-admin 的 10 個頁面已全數移植(wasm 202KB,gzip 後 84KB,不使用 serde);`wire_struct!`、JS/TS 用戶端、jQuery 外掛、供 Cosmo 使用的 GraphQL 子圖(已用 curl 驗證);`reaxt-db`(安全的參數綁定、SQL 方言、相容 PostgreSQL 的協定驅動程式與 SCRAM 驗證、連線池,其中 aruaru-db 已在真實伺服器上驗證)。
**後續規劃**:與真實的 Cosmo Router 整合測試、透過 TLS 連線真實的 PostgreSQL/CockroachDB、MySQL/Oracle/SQL Server/Snowflake 驅動程式、部署到 SET、導入 `wasm-opt` 並實測算繪速度、讓 `Wire` 支援列舉與選用欄位、RPoem 的萬用字元路由。

**新增(2026-11-19)**:Hook(`use_ref`、`use_memo`、`use_reducer`、`use_effect` 等)、Context、`memo`、Fragment、Suspense、錯誤邊界、Solid 風格的 Signal(含 `batch`)、Vue 與 Preact 風格的命名,以及 `wasm::portal`。串流 SSR 現已是**真正的非同步**:外殼不等待、立即送出,各個邊界並行執行並依完成順序替換,超過期限則改為備用內容(`App::stream_async`,由 `reaxt-rpoem` 透過真實 HTTP 送出;在真實瀏覽器中,外殼 15 毫秒、A 1023 毫秒、B 2015 毫秒依序顯示)。同步主機會回傳 501。尚未驗證:用戶端中斷連線時中止的自動化測試;不支援 Preact 的 `Component` 類別。

## 한국어
**현재 지원하는 기능**: 라우트 규약, SSR/CSR/Islands, 하이드레이션, Server Actions(Connect), DI/가드, 폼. aruaru-admin의 10개 화면을 모두 이식했고(wasm 202KB, gzip 84KB, serde 미사용), `wire_struct!`, JS/TS 클라이언트, jQuery 플러그인, Cosmo용 GraphQL 서브그래프(curl로 확인), `reaxt-db`(안전한 파라미터 바인딩, SQL 방언, SCRAM을 지원하는 PostgreSQL 호환 와이어 드라이버, 커넥션 풀)를 갖추었습니다. aruaru-db는 실제 서버에서 검증을 마쳤습니다.
**앞으로의 계획**: 실제 Cosmo Router와의 통합 검증, TLS를 통한 실제 PostgreSQL/CockroachDB 연결, MySQL·Oracle·SQL Server·Snowflake 드라이버, SET 배포, `wasm-opt` 적용과 렌더링 속도 측정, enum·선택 필드를 지원하는 `Wire`, RPoem의 와일드카드 라우트.

**추가(2026-11-19)**: 훅(`use_ref`, `use_memo`, `use_reducer`, `use_effect` 등), 컨텍스트, `memo`, Fragment, Suspense, 에러 경계, Solid 방식의 시그널(`batch` 포함), Vue·Preact 방식의 이름, 그리고 `wasm::portal`을 갖추었습니다. 스트리밍 SSR은 이제 **진짜 비동기**입니다. 셸은 기다리지 않고 먼저 내보내고, 각 경계는 동시에 실행되어 끝나는 순서대로 교체되며, 기한을 넘기면 대체 내용으로 바뀝니다(`App::stream_async`, `reaxt-rpoem`이 실제 HTTP로 전송. 실제 브라우저에서 셸 15ms, A 1023ms, B 2015ms 순으로 표시되는 것을 확인). 동기 호스트는 501을 반환합니다. 아직 확인하지 못한 것: 클라이언트 연결이 끊겼을 때 중단하는 자동 테스트, Preact의 `Component` 클래스(미지원).

## Deutsch
**Bereits umgesetzt**: Routenkonventionen, SSR/CSR/Islands, Hydration, Server Actions (Connect), Dependency Injection/Guards und Formulare. Alle 10 Bildschirme von aruaru-admin sind portiert (wasm 202 KB, 84 KB gzip-komprimiert, ohne serde), dazu `wire_struct!`, ein JS/TS-Client, ein jQuery-Plugin, ein GraphQL-Subgraph für Cosmo (mit curl geprüft) und `reaxt-db` (sichere Parameterbindung, SQL-Dialekte, PostgreSQL-kompatibler Protokolltreiber mit SCRAM, Verbindungspool; aruaru-db wurde gegen einen echten Server getestet).
**Noch geplant**: Zusammenspiel mit einem echten Cosmo Router, Anbindung an echtes PostgreSQL/CockroachDB über TLS, Treiber für MySQL, Oracle, SQL Server und Snowflake, Bereitstellung auf SET, `wasm-opt` samt Messung der Rendergeschwindigkeit, `Wire` für Enums und optionale Felder sowie Wildcard-Routen in RPoem.

**Neu (2026-11-19)**: Hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` u. a.), Context, `memo`, Fragment, Suspense, Error Boundaries, Signale nach Solid-Art (mit `batch`), Bezeichnungen im Stil von Vue und Preact sowie `wasm::portal`. Streaming-SSR ist jetzt **wirklich asynchron**: Die Shell wird sofort ausgeliefert, ohne zu warten; alle Grenzen laufen parallel und werden in der Reihenfolge ihrer Fertigstellung eingesetzt; wer die Frist überschreitet, wird durch einen Ersatzinhalt ersetzt (`App::stream_async`, von `reaxt-rpoem` über echtes HTTP gesendet; im echten Browser erschien die Shell nach 15 ms, A nach 1023 ms und B nach 2015 ms). Ein synchroner Host antwortet mit 501. Noch nicht geprüft: ein automatischer Test für den Abbruch bei Verbindungsabbruch des Clients; die `Component`-Klasse von Preact wird nicht unterstützt.

## Français
**Ce qui fonctionne déjà** : conventions de routes, SSR/CSR/Islands, hydratation, Server Actions (Connect), injection de dépendances/gardes et formulaires. Les 10 écrans d'aruaru-admin sont portés (wasm de 202 Ko, soit 84 Ko après gzip, sans serde), ainsi que `wire_struct!`, un client JS/TS, un plugin jQuery, un sous-graphe GraphQL pour Cosmo (vérifié avec curl) et `reaxt-db` (liaison sûre des paramètres, dialectes SQL, pilote de protocole compatible PostgreSQL avec SCRAM, pool de connexions ; aruaru-db a été testé sur un vrai serveur).
**À venir** : intégration avec un vrai Cosmo Router, connexion à de vrais PostgreSQL/CockroachDB via TLS, pilotes pour MySQL, Oracle, SQL Server et Snowflake, déploiement sur SET, `wasm-opt` et mesure de la vitesse de rendu, prise en charge des énumérations et des champs optionnels dans `Wire`, routes génériques dans RPoem.

**Ajouts (2026-11-19)** : hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect`, etc.), contexte, `memo`, Fragment, Suspense, limites d'erreur, signaux à la Solid (avec `batch`), noms dans le style de Vue et de Preact, et `wasm::portal`. Le SSR en streaming est désormais **réellement asynchrone** : la coquille part immédiatement sans attendre, chaque frontière s'exécute en parallèle et se substitue dans l'ordre d'achèvement, et une frontière qui dépasse son délai est remplacée par un contenu de repli (`App::stream_async`, envoyé en vrai HTTP par `reaxt-rpoem` ; dans un vrai navigateur, la coquille s'est affichée à 15 ms, A à 1023 ms et B à 2015 ms). Un hôte synchrone répond 501. Pas encore vérifié : un test automatique de l'abandon lors d'une déconnexion du client ; la classe `Component` de Preact n'est pas prise en charge.

## Español
**Lo que ya funciona**: convenciones de rutas, SSR/CSR/Islands, hidratación, Server Actions (Connect), inyección de dependencias/guardas y formularios. Las 10 pantallas de aruaru-admin están portadas (wasm de 202 KB, 84 KB con gzip, sin serde), junto con `wire_struct!`, un cliente JS/TS, un plugin de jQuery, un subgrafo GraphQL para Cosmo (comprobado con curl) y `reaxt-db` (enlace seguro de parámetros, dialectos SQL, controlador de protocolo compatible con PostgreSQL con SCRAM y un pool de conexiones; aruaru-db se ha probado en un servidor real).
**Próximos pasos**: integración con un Cosmo Router real, conexión a PostgreSQL/CockroachDB reales mediante TLS, controladores para MySQL, Oracle, SQL Server y Snowflake, despliegue en SET, `wasm-opt` y medición de la velocidad de renderizado, compatibilidad de `Wire` con enumeraciones y campos opcionales, y rutas comodín en RPoem.

**Novedades (2026-11-19)**: hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` y más), contexto, `memo`, Fragment, Suspense, límites de error, señales al estilo de Solid (con `batch`), nombres al estilo de Vue y Preact, y `wasm::portal`. El SSR en streaming ahora es **realmente asíncrono**: el esqueleto se envía de inmediato sin esperar, cada frontera se ejecuta en paralelo y se sustituye según termina, y si una frontera supera su plazo se cambia por un contenido alternativo (`App::stream_async`, enviado por HTTP real mediante `reaxt-rpoem`; en un navegador real el esqueleto apareció a los 15 ms, A a los 1023 ms y B a los 2015 ms). Un host síncrono responde 501. Aún sin verificar: una prueba automática de la cancelación cuando el cliente se desconecta; la clase `Component` de Preact no es compatible.

## Português
**O que já funciona**: convenções de rotas, SSR/CSR/Islands, hidratação, Server Actions (Connect), injeção de dependências/guardas e formulários. As 10 telas do aruaru-admin foram portadas (wasm de 202 KB, 84 KB com gzip, sem serde), além de `wire_struct!`, um cliente JS/TS, um plugin jQuery, um subgrafo GraphQL para o Cosmo (verificado com curl) e o `reaxt-db` (vinculação segura de parâmetros, dialetos SQL, driver de protocolo compatível com PostgreSQL com SCRAM e pool de conexões; o aruaru-db foi testado em um servidor real).
**Próximos passos**: integração com um Cosmo Router real, conexão a PostgreSQL/CockroachDB reais via TLS, drivers para MySQL, Oracle, SQL Server e Snowflake, implantação no SET, `wasm-opt` e medição da velocidade de renderização, suporte a enums e campos opcionais no `Wire` e rotas curinga no RPoem.

**Novidades (2026-11-19)**: hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` e outros), contexto, `memo`, Fragment, Suspense, limites de erro, sinais no estilo Solid (com `batch`), nomes no estilo Vue e Preact, e `wasm::portal`. O SSR com streaming agora é **realmente assíncrono**: o esqueleto é enviado imediatamente, sem esperar; cada fronteira roda em paralelo e é substituída na ordem em que termina, e uma fronteira que ultrapassa o prazo é trocada por um conteúdo alternativo (`App::stream_async`, enviado por HTTP real pelo `reaxt-rpoem`; em um navegador real, o esqueleto apareceu aos 15 ms, o A aos 1023 ms e o B aos 2015 ms). Um host síncrono responde 501. Ainda não verificado: um teste automático do cancelamento quando o cliente se desconecta; a classe `Component` do Preact não é compatível.

## Русский
**Что уже работает**: соглашения о маршрутах, SSR/CSR/Islands, гидратация, Server Actions (Connect), внедрение зависимостей и гарды, формы. Перенесены все 10 экранов aruaru-admin (wasm — 202 КБ, 84 КБ после gzip, без serde), а также `wire_struct!`, клиент для JS/TS, плагин для jQuery, GraphQL-подграф для Cosmo (проверен через curl) и `reaxt-db` (безопасная подстановка параметров, диалекты SQL, драйвер PostgreSQL-совместимого протокола с SCRAM, пул соединений; aruaru-db проверена на реальном сервере).
**В планах**: интеграция с настоящим Cosmo Router, подключение к реальным PostgreSQL/CockroachDB по TLS, драйверы MySQL, Oracle, SQL Server и Snowflake, развёртывание на SET, `wasm-opt` и замеры скорости отрисовки, поддержка перечислений и необязательных полей в `Wire`, маршруты с подстановочными символами в RPoem.
**Добавлено (2026-11-19)**: хуки (`use_ref`, `use_memo`, `use_reducer`, `use_effect` и другие), контекст, `memo`, Fragment, Suspense, границы ошибок, сигналы в стиле Solid (с `batch`), названия в стиле Vue и Preact, а также `wasm::portal`. Потоковый SSR теперь **по-настоящему асинхронный**: оболочка отправляется сразу, не дожидаясь ничего, каждая граница выполняется параллельно и подставляется по мере готовности, а граница, превысившая срок, заменяется запасным содержимым (`App::stream_async`, отправка по настоящему HTTP через `reaxt-rpoem`; в настоящем браузере оболочка появилась через 15 мс, A — через 1023 мс, B — через 2015 мс). Синхронный хост отвечает 501. Ещё не проверено: автоматический тест прерывания при отключении клиента; класс `Component` из Preact не поддерживается.


## 2026-11-19 更新: 複数サブグラフにまたがる問い合わせ(3姉妹共通) / Cross-subgraph queries (shared by the three sisters)

### 日本語
`open-cosmo` のゲートウェイが、複数のサブグラフ(別々のサーバー)にまたがる問い合わせを扱えるようになりました。`Reaxt`・`RPoRea`・`open-cosmo` は常に同時に開発しているため、3つのリポジトリに同じ内容を書いています。

- **分割と統合**: 読み取り専用の問い合わせで、最上位のフィールドが別々のサブグラフにあるときは、サブグラフごとの問い合わせに分け、同時に実行して、結果を1つにまとめます。
- **`@key` による結合(1段)**: `@key` を持つ型について、サブグラフAが返した `id` を使い、サブグラフBの `_entities` に問い合わせて、Aにないフィールドを足します。
- **従来どおり拒否するもの**: 変更系(mutation)が複数のサブグラフにまたがる問い合わせ、リモートとローカルが混ざる問い合わせ、フラグメントやディレクティブを含む分割対象の問い合わせ。
- **確認済み**: 擬似サブグラフでの試験(gateway 44件、federation 13件)に成功。Reaxt の実サーバーが返す SDL も、`compose` と管理 RPC `ComposeFederation` で受け付けられます。
- **未了**: 複数段にわたる入れ子の結合、`@requires` / `@provides`、実際の Reaxt サーバーを複数並べた通し確認(今のサンプルは同じフィールド名を持つため、振り分けの確認にはサンプルの追加が必要)。

### English
The `open-cosmo` gateway can now handle queries that span several subgraphs (separate servers). `Reaxt`, `RPoRea` and `open-cosmo` are always developed together, so all three repositories carry this same note.

- **Split and merge**: for a read-only query whose top-level fields live in different subgraphs, the gateway builds one sub-query per subgraph, runs them concurrently, and merges the results.
- **`@key` joins (one level)**: for a type that has `@key`, the gateway takes the `id` returned by subgraph A and asks subgraph B, through `_entities`, for the fields A does not have.
- **Still rejected**: mutations that span subgraphs, queries that mix remote and local fields, and split queries that contain fragments or directives.
- **Verified**: tests against fake subgraphs pass (gateway 44, federation 13). The SDL served by a real Reaxt server is accepted by `compose` and by the admin RPC `ComposeFederation`.
- **Open**: joins nested more than one level, `@requires` / `@provides`, and a run with several real Reaxt servers (the current example exposes the same field names, so a second example is needed to check routing).

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

## 2026-11-19 追記(続き6): `@skip` / `@include` と、3者以上の連鎖に対応(4姉妹共通) / `@skip` / `@include` and chains of three or more subgraphs (shared by the four sisters)

### 日本語
`open-cosmo` のゲートウェイの結合が、次の2つに対応しました。

- **`@skip(if:)` / `@include(if:)`**: 条件は、リテラルか、問い合わせの時点で分かっている変数の値で、ゲートウェイが**計画時に**判定します。除外されたフィールドは問い合わせから外し、そのフィールドのためだけに必要だった別のサブグラフへの呼び出しも行いません。これ以外のディレクティブや、値の分からない変数は、従来どおり結合せずに転送します。
- **3者以上の連鎖**: `@requires` の必須フィールドが、最初のサブグラフでも結合先でもない**さらに別のサブグラフ**にあるとき、その別のサブグラフへの結合を先に行い、得た値を結合先の表現(representation)に入れます(例: catalog → pricing で `price` を取得 → shipping の `shippingEstimate` に渡す)。結合には順位を付けて、必要な順に実行します(最大4段)。クライアントが選んでいない必須フィールドは、最後に結果から取り除きます。どのサブグラフも供給できないときは、結合せずに転送します。
- **試験**: ゲートウェイ51件に成功(擬似サブグラフ)。
- **未了**: 実際のサーバー(Reaxt 以外のサブグラフ)での `@key` / `@requires` の通し確認、`@skip` / `@include` をフラグメント内で使う場合(フラグメントは分割・結合の対象外のまま)。

### English
The join in the `open-cosmo` gateway now handles two more cases.

- **`@skip(if:)` / `@include(if:)`**: the condition is a literal or a variable whose value is already known, and the gateway decides it **at planning time**. An excluded field is dropped from the query, and a call to another subgraph that was needed only for that field is not made. Other directives, and variables with no known value, are forwarded as before, without a join.
- **Chains of three or more subgraphs**: when a `@requires` field lives in **yet another subgraph** (neither the first one nor the one being joined), the gateway first joins that third subgraph and puts the value into the representation sent onward (for example: catalog, then pricing supplies `price`, then shipping computes `shippingEstimate`). Joins get a rank and run in dependency order (up to four levels). Required fields the client did not select are removed from the final result. If no subgraph can supply a field, the query is forwarded without a join.
- **Tests**: the gateway passes 51 tests (fake subgraphs).
- **Open**: a run of `@key` / `@requires` against real servers other than Reaxt, and `@skip` / `@include` inside fragments (fragments are still outside the split and join planner).

## 2026-11-19 追記(続き7): 本物のフェデレーション実装2台で `@key` 結合を確認し、ルート型の名前の不具合を修正(4姉妹共通) / `@key` join verified against two real federation servers; root-type-name bug fixed (shared by the four sisters)

### 日本語
擬似サブグラフだけでなく、**独立した本物の GraphQL サーバー**でも確認しました。`async-graphql` のフェデレーション機能(`_service { sdl }` と `_entities` を自前で実装)で users と reviews の2台を立て、`open-cosmo` のゲートウェイ越しに `{ me { name reviews } }` を送ると、`{"name":"user-7","reviews":["review-of-7"]}` と正しく結合されました(試験はゲートウェイ52件に成功)。

- **見つかった不具合と修正**: 実際のサーバーは、ルート型の名前が `Query` とは限りません(今回は `QueryA`)。ゲートウェイは `Query` / `Mutation` 固定で探していたため、結合が働かず、元の問い合わせがそのまま転送されて失敗しました。内省(introspection)でルート型の名前を取得し、内部では `Query` / `Mutation` に正規化するようにして直しました。擬似サブグラフだけの試験では見つからなかった不具合です。
- **未了**: `@requires` と `@provides` を本物のサーバーで確認する試験(今回は `@key` のみ)、Reaxt 自身のエンティティ(`@key`)対応、フラグメント内のディレクティブ。

### English
We verified the join not only with fake subgraphs but also with **independent, real GraphQL servers**. Two servers (users and reviews) were built with `async-graphql`'s own federation support (its own `_service { sdl }` and `_entities`); sending `{ me { name reviews } }` through the `open-cosmo` gateway returned the correctly joined `{"name":"user-7","reviews":["review-of-7"]}` (the gateway passes 52 tests).

- **Bug found and fixed**: a real server's root type is not always called `Query` (here it was `QueryA`). The gateway looked for `Query` / `Mutation` by name, so no join happened and the original query was forwarded unchanged, which failed. The gateway now reads the root type names through introspection and normalizes them to `Query` / `Mutation` internally. The fake-subgraph tests alone could not have found this.
- **Open**: tests of `@requires` and `@provides` against real servers (this run covered `@key` only), entity (`@key`) support in Reaxt itself, and directives inside fragments.

## 2026-11-19 追記(続き8): 本物の SDL での `@external` / `@requires` / `@provides` の読み取りを確認、Reaxt のエンティティ対応は見送り(4姉妹共通) / Directives read from real SDL; Reaxt entities deliberately not added (shared by the four sisters)

### 日本語
- **確認**: `async-graphql` が実際に生成する SDL(`#[graphql(external)]`・`#[graphql(requires = "price")]`・`#[graphql(provides = "price")]` つき)を、ゲートウェイのモデルが正しく読むことを試験で確認しました(`@key`・`@external`・`@requires`・`@provides` の4種。ゲートウェイ53件に成功)。手書きの SDL だけでなく、実際の実装が出力する書式でも読めます。ただし、`@requires` と `@provides` を使った**結合の実行**を本物のサーバー2台で通すところまでは、まだ行っていません(実行の確認は擬似サブグラフでの試験のみ)。
- **判断(見送り)**: Reaxt 自身を `@key` のエンティティにすることは**しません**。Reaxt のアクションは `JSON` スカラーを返し、GraphQL の型(フィールドの構造)を持たないためです。型を持たせるには Reaxt のアクション定義に型情報を足す大きな設計変更が必要で、現状の用途(アクションの呼び出しと読み取り)には不要です。必要になったら、`Wire` 型から GraphQL の型を生成する形を検討します。
- **未了**: `@requires` / `@provides` を使った結合を、本物のサーバー2台で実行する試験、フラグメント内のディレクティブ。

### English
- **Verified**: the gateway model correctly reads the SDL that `async-graphql` really generates (with `#[graphql(external)]`, `#[graphql(requires = "price")]` and `#[graphql(provides = "price")]`): `@key`, `@external`, `@requires` and `@provides` (the gateway passes 53 tests). It reads the format produced by a real implementation, not only hand-written SDL. However, running a **join that uses `@requires` and `@provides`** against two real servers has not been done yet (that execution is covered only by fake-subgraph tests).
- **Decision (not added)**: Reaxt itself will **not** become a `@key` entity. Reaxt actions return the `JSON` scalar and have no GraphQL type (no field structure). Giving them one would need a large design change in the action definitions, and the current use (calling actions and reading values) does not need it. If it is ever needed, we will consider generating GraphQL types from the `Wire` types.
- **Open**: running joins that use `@requires` / `@provides` against two real servers, and directives inside fragments.
