# Reaxt

Rust だけで書くフロント+バックエンド統合の枠組み(React / Next.js / Nuxt / Nest.js の良い所を研究して統合する構想)。
RPoem と SET で使う。設計は [DESIGN.md](DESIGN.md)。現状は 0.0.1(ルート規約と描画方式の核のみ)。

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

```bash
cargo test --workspace
```
ライセンス: Apache-2.0 OR MIT
