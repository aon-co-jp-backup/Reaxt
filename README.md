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

## English
**What works today**: route conventions, SSR/CSR/Islands, hydration, Server Actions (Connect), DI/guards and forms. All 10 aruaru-admin screens are ported (wasm 202 KB, 84 KB gzipped, no serde), along with `wire_struct!`, a JS/TS client, a jQuery plugin, a GraphQL subgraph for Cosmo (checked with curl) and `reaxt-db` (safe parameter binding, dialects, a PostgreSQL-compatible wire driver with SCRAM, and a connection pool; aruaru-db is verified against a real server).
**Still to come**: composing with a real Cosmo Router, connecting to real PostgreSQL/CockroachDB over TLS, drivers for MySQL, Oracle, SQL Server and Snowflake, deployment on SET, `wasm-opt` plus rendering-speed measurements, `Wire` support for enums and optional fields, and wildcard routes in RPoem.

## 简体中文
**目前已实现**:路由约定、SSR/CSR/Islands、水合、Server Actions(Connect)、依赖注入/守卫、表单;aruaru-admin 的 10 个页面已全部迁移(wasm 202KB,gzip 后 84KB,不依赖 serde);`wire_struct!`、JS/TS 客户端、jQuery 插件、面向 Cosmo 的 GraphQL 子图(已用 curl 验证);`reaxt-db`(安全的参数绑定、SQL 方言、兼容 PostgreSQL 的协议驱动及 SCRAM 认证、连接池,其中 aruaru-db 已在真实服务器上验证)。
**后续规划**:与真实的 Cosmo Router 联调、通过 TLS 连接真实的 PostgreSQL/CockroachDB、MySQL/Oracle/SQL Server/Snowflake 驱动、部署到 SET、引入 `wasm-opt` 并实测渲染速度、让 `Wire` 支持枚举和可选字段、RPoem 的通配路由。

## 繁體中文
**目前已完成**:路由慣例、SSR/CSR/Islands、水合、Server Actions(Connect)、相依注入/守衛、表單;aruaru-admin 的 10 個頁面已全數移植(wasm 202KB,gzip 後 84KB,不使用 serde);`wire_struct!`、JS/TS 用戶端、jQuery 外掛、供 Cosmo 使用的 GraphQL 子圖(已用 curl 驗證);`reaxt-db`(安全的參數綁定、SQL 方言、相容 PostgreSQL 的協定驅動程式與 SCRAM 驗證、連線池,其中 aruaru-db 已在真實伺服器上驗證)。
**後續規劃**:與真實的 Cosmo Router 整合測試、透過 TLS 連線真實的 PostgreSQL/CockroachDB、MySQL/Oracle/SQL Server/Snowflake 驅動程式、部署到 SET、導入 `wasm-opt` 並實測算繪速度、讓 `Wire` 支援列舉與選用欄位、RPoem 的萬用字元路由。

## 한국어
**현재 지원하는 기능**: 라우트 규약, SSR/CSR/Islands, 하이드레이션, Server Actions(Connect), DI/가드, 폼. aruaru-admin의 10개 화면을 모두 이식했고(wasm 202KB, gzip 84KB, serde 미사용), `wire_struct!`, JS/TS 클라이언트, jQuery 플러그인, Cosmo용 GraphQL 서브그래프(curl로 확인), `reaxt-db`(안전한 파라미터 바인딩, SQL 방언, SCRAM을 지원하는 PostgreSQL 호환 와이어 드라이버, 커넥션 풀)를 갖추었습니다. aruaru-db는 실제 서버에서 검증을 마쳤습니다.
**앞으로의 계획**: 실제 Cosmo Router와의 통합 검증, TLS를 통한 실제 PostgreSQL/CockroachDB 연결, MySQL·Oracle·SQL Server·Snowflake 드라이버, SET 배포, `wasm-opt` 적용과 렌더링 속도 측정, enum·선택 필드를 지원하는 `Wire`, RPoem의 와일드카드 라우트.

## Deutsch
**Bereits umgesetzt**: Routenkonventionen, SSR/CSR/Islands, Hydration, Server Actions (Connect), Dependency Injection/Guards und Formulare. Alle 10 Bildschirme von aruaru-admin sind portiert (wasm 202 KB, 84 KB gzip-komprimiert, ohne serde), dazu `wire_struct!`, ein JS/TS-Client, ein jQuery-Plugin, ein GraphQL-Subgraph für Cosmo (mit curl geprüft) und `reaxt-db` (sichere Parameterbindung, SQL-Dialekte, PostgreSQL-kompatibler Protokolltreiber mit SCRAM, Verbindungspool; aruaru-db wurde gegen einen echten Server getestet).
**Noch geplant**: Zusammenspiel mit einem echten Cosmo Router, Anbindung an echtes PostgreSQL/CockroachDB über TLS, Treiber für MySQL, Oracle, SQL Server und Snowflake, Bereitstellung auf SET, `wasm-opt` samt Messung der Rendergeschwindigkeit, `Wire` für Enums und optionale Felder sowie Wildcard-Routen in RPoem.

## Français
**Ce qui fonctionne déjà** : conventions de routes, SSR/CSR/Islands, hydratation, Server Actions (Connect), injection de dépendances/gardes et formulaires. Les 10 écrans d'aruaru-admin sont portés (wasm de 202 Ko, soit 84 Ko après gzip, sans serde), ainsi que `wire_struct!`, un client JS/TS, un plugin jQuery, un sous-graphe GraphQL pour Cosmo (vérifié avec curl) et `reaxt-db` (liaison sûre des paramètres, dialectes SQL, pilote de protocole compatible PostgreSQL avec SCRAM, pool de connexions ; aruaru-db a été testé sur un vrai serveur).
**À venir** : intégration avec un vrai Cosmo Router, connexion à de vrais PostgreSQL/CockroachDB via TLS, pilotes pour MySQL, Oracle, SQL Server et Snowflake, déploiement sur SET, `wasm-opt` et mesure de la vitesse de rendu, prise en charge des énumérations et des champs optionnels dans `Wire`, routes génériques dans RPoem.

## Español
**Lo que ya funciona**: convenciones de rutas, SSR/CSR/Islands, hidratación, Server Actions (Connect), inyección de dependencias/guardas y formularios. Las 10 pantallas de aruaru-admin están portadas (wasm de 202 KB, 84 KB con gzip, sin serde), junto con `wire_struct!`, un cliente JS/TS, un plugin de jQuery, un subgrafo GraphQL para Cosmo (comprobado con curl) y `reaxt-db` (enlace seguro de parámetros, dialectos SQL, controlador de protocolo compatible con PostgreSQL con SCRAM y un pool de conexiones; aruaru-db se ha probado en un servidor real).
**Próximos pasos**: integración con un Cosmo Router real, conexión a PostgreSQL/CockroachDB reales mediante TLS, controladores para MySQL, Oracle, SQL Server y Snowflake, despliegue en SET, `wasm-opt` y medición de la velocidad de renderizado, compatibilidad de `Wire` con enumeraciones y campos opcionales, y rutas comodín en RPoem.

## Português
**O que já funciona**: convenções de rotas, SSR/CSR/Islands, hidratação, Server Actions (Connect), injeção de dependências/guardas e formulários. As 10 telas do aruaru-admin foram portadas (wasm de 202 KB, 84 KB com gzip, sem serde), além de `wire_struct!`, um cliente JS/TS, um plugin jQuery, um subgrafo GraphQL para o Cosmo (verificado com curl) e o `reaxt-db` (vinculação segura de parâmetros, dialetos SQL, driver de protocolo compatível com PostgreSQL com SCRAM e pool de conexões; o aruaru-db foi testado em um servidor real).
**Próximos passos**: integração com um Cosmo Router real, conexão a PostgreSQL/CockroachDB reais via TLS, drivers para MySQL, Oracle, SQL Server e Snowflake, implantação no SET, `wasm-opt` e medição da velocidade de renderização, suporte a enums e campos opcionais no `Wire` e rotas curinga no RPoem.

## Русский
**Что уже работает**: соглашения о маршрутах, SSR/CSR/Islands, гидратация, Server Actions (Connect), внедрение зависимостей и гарды, формы. Перенесены все 10 экранов aruaru-admin (wasm — 202 КБ, 84 КБ после gzip, без serde), а также `wire_struct!`, клиент для JS/TS, плагин для jQuery, GraphQL-подграф для Cosmo (проверен через curl) и `reaxt-db` (безопасная подстановка параметров, диалекты SQL, драйвер PostgreSQL-совместимого протокола с SCRAM, пул соединений; aruaru-db проверена на реальном сервере).
**В планах**: интеграция с настоящим Cosmo Router, подключение к реальным PostgreSQL/CockroachDB по TLS, драйверы MySQL, Oracle, SQL Server и Snowflake, развёртывание на SET, `wasm-opt` и замеры скорости отрисовки, поддержка перечислений и необязательных полей в `Wire`, маршруты с подстановочными символами в RPoem.
