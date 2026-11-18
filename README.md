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
