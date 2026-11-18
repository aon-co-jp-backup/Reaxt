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
- `reaxt-rpoem/` — RPoem上で配信するアダプタ
- `reaxt-actions/` — Server Actions(型付きConnect呼び出し、JSONはRS-JSON)
- `reaxt-cli/` — `reaxt` コマンド
- `examples/counter/` — SSR・Islands・Server Actionsのデモ
- `examples/aruaru-admin/` — aruaru-admin Webの移植(全10画面、実aruaru-serverと実ブラウザで確認)。`ARUARU_GQL=host:port`で上流、`ARUARU_DB_ADMIN_TOKEN`でサーバー側中継のトークン(ブラウザには出ない)。管理操作は許可リスト11メソッドのみ中継。wasmはserde/serde_json不使用(RS-JSON `light` + `Wire`トレイト)
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
  | PostgreSQL・CockroachDB・YugabyteDB・Redshift | `PgDb`(ワイヤv3、trust/平文/MD5/SCRAM)。**実物のこれらへの接続は未確認**(aruaru-dbのpgwireだけで確認)。TLSは未対応のためTLS必須の接続先はTLS終端のプロキシ経由 |
  | MySQL・MariaDB・TiDB・Oracle・SQL Server・Snowflake・SQLite・BigQuery | **束縛の方言のみ**(`Dialect`、単体テスト)。**接続ドライバは未実装**(つながらない) |


---

# 現状と完成予定 / Status and Roadmap (10 languages)

> 2026-11-19。日本語・英語以外はAI翻訳で、ネイティブ検証前です。/ Languages other than Japanese and English are AI translations, not yet checked by native speakers.

## 日本語
**できていること**: ルート規約・SSR/CSR/Islands・ハイドレーション・Server Actions(Connect)・DI/ガード・フォーム、aruaru-admin全10画面の移植(wasm 202KB・gzip 84KB、serde不使用)、`wire_struct!`、JS/TSクライアント、jQueryプラグイン、Cosmo向けGraphQLサブグラフ(curlで確認)、`reaxt-db`(安全な束縛・方言・PostgreSQL互換ワイヤ+SCRAM、aruaru-dbは実サーバーで確認)。
**完成予定**: 実Cosmo Routerでの合成確認、実PostgreSQL/CockroachDBへの接続とTLS、MySQL・Oracle・SQL Server・Snowflakeのドライバ、接続プール、SETへの配備、`wasm-opt`と描画速度の実測、enum/任意フィールド対応の`Wire`、RPoemのワイヤルドカードルート。

## English
**Done**: route conventions, SSR/CSR/Islands, hydration, Server Actions (Connect), DI/guards, forms; all 10 aruaru-admin screens ported (wasm 202 KB, 84 KB gzip, no serde); `wire_struct!`; JS/TS client; jQuery plugin; a GraphQL subgraph for Cosmo (checked with curl); `reaxt-db` (safe binding, dialects, PostgreSQL-compatible wire + SCRAM; aruaru-db verified against a real server).
**Planned**: composing with a real Cosmo Router, connecting to real PostgreSQL/CockroachDB with TLS, drivers for MySQL, Oracle, SQL Server and Snowflake, a connection pool, deployment on SET, `wasm-opt` and rendering-speed measurements, `Wire` for enums/optional fields, wildcard routes in RPoem.

## 简体中文
**已完成**:路由约定、SSR/CSR/Islands、水合、Server Actions(Connect)、依赖注入/守卫、表单;已移植 aruaru-admin 全部 10 个页面(wasm 202KB,gzip 84KB,不使用 serde);`wire_struct!`;JS/TS 客户端;jQuery 插件;面向 Cosmo 的 GraphQL 子图(已用 curl 验证);`reaxt-db`(安全参数绑定、方言、兼容 PostgreSQL 的协议+SCRAM,aruaru-db 已在真实服务器上验证)。
**计划**:与真实 Cosmo Router 联合验证、连接真实 PostgreSQL/CockroachDB 并支持 TLS、MySQL/Oracle/SQL Server/Snowflake 驱动、连接池、部署到 SET、`wasm-opt` 与渲染速度实测、支持枚举/可选字段的 `Wire`、RPoem 通配路由。

## 繁體中文
**已完成**:路由慣例、SSR/CSR/Islands、水合、Server Actions(Connect)、相依注入/守衛、表單;已移植 aruaru-admin 全部 10 個頁面(wasm 202KB,gzip 84KB,不使用 serde);`wire_struct!`;JS/TS 用戶端;jQuery 外掛;面向 Cosmo 的 GraphQL 子圖(已用 curl 驗證);`reaxt-db`(安全參數綁定、方言、相容 PostgreSQL 的協定+SCRAM,aruaru-db 已在真實伺服器上驗證)。
**計畫**:與真實 Cosmo Router 聯合驗證、連線真實 PostgreSQL/CockroachDB 並支援 TLS、MySQL/Oracle/SQL Server/Snowflake 驅動、連線池、部署到 SET、`wasm-opt` 與算繪速度實測、支援列舉/選用欄位的 `Wire`、RPoem 萬用字元路由。

## 한국어
**완료**: 라우트 규약, SSR/CSR/Islands, 하이드레이션, Server Actions(Connect), DI/가드, 폼, aruaru-admin 10개 화면 전체 이식(wasm 202KB, gzip 84KB, serde 미사용), `wire_struct!`, JS/TS 클라이언트, jQuery 플러그인, Cosmo용 GraphQL 서브그래프(curl로 확인), `reaxt-db`(안전한 바인딩, 방언, PostgreSQL 호환 와이어+SCRAM, aruaru-db는 실제 서버에서 확인).
**예정**: 실제 Cosmo Router와의 합성 확인, 실제 PostgreSQL/CockroachDB 연결과 TLS, MySQL·Oracle·SQL Server·Snowflake 드라이버, 커넥션 풀, SET 배포, `wasm-opt`와 렌더링 속도 실측, enum/선택 필드를 지원하는 `Wire`, RPoem 와일드카드 라우트.

## Deutsch
**Fertig**: Routenkonventionen, SSR/CSR/Islands, Hydration, Server Actions (Connect), DI/Guards, Formulare; alle 10 Bildschirme von aruaru-admin portiert (wasm 202 KB, 84 KB gzip, ohne serde); `wire_struct!`; JS/TS-Client; jQuery-Plugin; GraphQL-Subgraph für Cosmo (mit curl geprüft); `reaxt-db` (sichere Parameterbindung, Dialekte, PostgreSQL-kompatibles Protokoll + SCRAM; aruaru-db an einem echten Server geprüft).
**Geplant**: Komposition mit einem echten Cosmo Router, Anbindung an echtes PostgreSQL/CockroachDB mit TLS, Treiber für MySQL, Oracle, SQL Server und Snowflake, Verbindungspool, Betrieb auf SET, `wasm-opt` und Messung der Rendergeschwindigkeit, `Wire` für Enums/optionale Felder, Wildcard-Routen in RPoem.

## Français
**Réalisé** : conventions de routes, SSR/CSR/Islands, hydratation, Server Actions (Connect), DI/gardes, formulaires ; les 10 écrans d'aruaru-admin portés (wasm 202 Ko, 84 Ko en gzip, sans serde) ; `wire_struct!` ; client JS/TS ; plugin jQuery ; sous-graphe GraphQL pour Cosmo (vérifié avec curl) ; `reaxt-db` (liaison sûre des paramètres, dialectes, protocole compatible PostgreSQL + SCRAM ; aruaru-db vérifié sur un vrai serveur).
**Prévu** : composition avec un vrai Cosmo Router, connexion à un vrai PostgreSQL/CockroachDB avec TLS, pilotes MySQL, Oracle, SQL Server et Snowflake, pool de connexions, déploiement sur SET, `wasm-opt` et mesure de la vitesse de rendu, `Wire` pour les énumérations/champs optionnels, routes génériques dans RPoem.

## Español
**Hecho**: convenciones de rutas, SSR/CSR/Islands, hidratación, Server Actions (Connect), DI/guardas, formularios; las 10 pantallas de aruaru-admin portadas (wasm 202 KB, 84 KB en gzip, sin serde); `wire_struct!`; cliente JS/TS; plugin de jQuery; subgrafo GraphQL para Cosmo (comprobado con curl); `reaxt-db` (enlace seguro de parámetros, dialectos, protocolo compatible con PostgreSQL + SCRAM; aruaru-db verificado en un servidor real).
**Previsto**: composición con un Cosmo Router real, conexión a PostgreSQL/CockroachDB reales con TLS, controladores para MySQL, Oracle, SQL Server y Snowflake, pool de conexiones, despliegue en SET, `wasm-opt` y medición de la velocidad de renderizado, `Wire` para enums/campos opcionales, rutas comodín en RPoem.

## Português
**Pronto**: convenções de rotas, SSR/CSR/Islands, hidratação, Server Actions (Connect), DI/guardas, formulários; as 10 telas do aruaru-admin portadas (wasm 202 KB, 84 KB em gzip, sem serde); `wire_struct!`; cliente JS/TS; plugin jQuery; subgrafo GraphQL para o Cosmo (verificado com curl); `reaxt-db` (vinculação segura de parâmetros, dialetos, protocolo compatível com PostgreSQL + SCRAM; aruaru-db verificado em um servidor real).
**Planejado**: composição com um Cosmo Router real, conexão a PostgreSQL/CockroachDB reais com TLS, drivers para MySQL, Oracle, SQL Server e Snowflake, pool de conexões, implantação no SET, `wasm-opt` e medição da velocidade de renderização, `Wire` para enums/campos opcionais, rotas curinga no RPoem.

## Русский
**Готово**: соглашения о маршрутах, SSR/CSR/Islands, гидратация, Server Actions (Connect), внедрение зависимостей/охранники, формы; перенесены все 10 экранов aruaru-admin (wasm 202 КБ, 84 КБ в gzip, без serde); `wire_struct!`; клиент JS/TS; плагин jQuery; GraphQL-подграф для Cosmo (проверен с помощью curl); `reaxt-db` (безопасная привязка параметров, диалекты, PostgreSQL-совместимый протокол + SCRAM; aruaru-db проверена на реальном сервере).
**Запланировано**: композиция с настоящим Cosmo Router, подключение к настоящим PostgreSQL/CockroachDB с TLS, драйверы MySQL, Oracle, SQL Server и Snowflake, пул соединений, развёртывание на SET, `wasm-opt` и замер скорости отрисовки, `Wire` для перечислений/необязательных полей, маршруты с подстановочными знаками в RPoem.
