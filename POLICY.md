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
