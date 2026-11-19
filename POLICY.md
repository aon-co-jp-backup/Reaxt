# API 方針(Reaxt) / API policy (Reaxt)

## 日本語
正本は `aon-co-jp/runo` の `API-POLICY.md`。要点: REST は全廃し、新しい REST は作らない。ブラウザや `curl` からは Connect(`POST /<パッケージ>.<Service>/<Method>`、JSON)、サービス間は gRPC / gRPC-web。GraphQL は問い合わせ(読み取り)専用。JSON の読み書きは RS-JSON に統一する。

- **このリポジトリの状況(2026-11-19)**: アクションは Connect で公開し、Cosmo 向けには `POST /_reaxt/graphql`(サブグラフ入口。`_service{sdl}` と最小の `__schema` に答え、`input:` のリテラルと `$変数` を受け付ける)を持つ。自前の REST はない。
- **例外(相手の規格が形を決めるもの)**: ヘルスチェック、静的ファイル、第三者の Webhook、ACME、OAuth/OIDC のコールバック、OTLP。
- **3姉妹**: `Reaxt`・`RPoRea`・`open-cosmo` は常に同時に開発する。接点(Cosmo のサブグラフ形式・manifest・Connect・`x-reaxt`)を変えたら、3つの文書を同じ日に更新する。

## English
The source of truth is `API-POLICY.md` in `aon-co-jp/runo`. In short: REST is being abolished and no new REST is added. Browsers and `curl` use Connect (`POST /<package>.<Service>/<Method>`, JSON); service-to-service calls use gRPC / gRPC-web. GraphQL is for read-only queries. JSON is read and written through RS-JSON only.

- **Status of this repository (2026-11-19)**: Actions are exposed over Connect, and for Cosmo there is `POST /_reaxt/graphql` (a subgraph entry that answers `_service{sdl}` and a minimal `__schema`, and accepts `input:` literals and `$variables`). There is no REST of our own.
- **Exceptions (shapes dictated by someone else's standard)**: health checks, static files, third-party webhooks, ACME, OAuth/OIDC callbacks, OTLP.
- **The three sisters**: `Reaxt`, `RPoRea` and `open-cosmo` are always developed together. When an interface between them changes (the Cosmo subgraph format, the manifest, Connect, `x-reaxt`), all three documents are updated on the same day.

### 追記(入れ子の結合) / Update (nested joins)
**日本語**: 上の「ゲートウェイの動き」(3)の `@key` 結合は、現在は入れ子の段にも対応している(浅い段から順に実行)。拒否する条件は変わらない。 **English**: The `@key` join in item (3) above now also works at nested levels (executed from the shallowest level down). The rejection rules are unchanged.

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
