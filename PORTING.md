# Reaxt お引越し・再開メモ(PORTING.md)

別のPC・別アカウントで続きをやるための手順と到達点。設計は [DESIGN.md](DESIGN.md)、ルールは [CLAUDE.md](CLAUDE.md)。

## 必要な道具
- Rust(stable)+ `rustup target add wasm32-unknown-unknown`
- `wasm-bindgen` CLI **0.2.126**(`cargo install wasm-bindgen-cli --version 0.2.126`)
- 兄弟ディレクトリに `RS-JSON`(パス依存。`F:\RS-JSON`、Reaxtの1つ上の階層に並べる)。RPoem連携(`reaxt-rpoem`)は `RPoem` も同じ階層に必要
- 元アプリとの比較をする場合のみ: Node と pnpm(`npm install -g pnpm`)

`reaxt doctor` で確認できる。

## 動かす(aruaru-admin の例)
```bash
# 1) wasmを作る
cargo build -p aruaru-admin-reaxt --lib --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir examples/aruaru-admin/pkg target/wasm32-unknown-unknown/release/aruaru_admin_reaxt.wasm
# 2) 上流(aruaru-server)を起動。管理操作にはトークンが要る
ARUARU_DB_ADMIN_TOKEN=tk aruaru-server --data <dir> --gql-port 4100 --raft-id 1
# 3) 中継サーバー(同じトークンを環境変数で渡す。ブラウザには出ない)
ARUARU_DB_ADMIN_TOKEN=tk ARUARU_GQL=127.0.0.1:4100 cargo run -p aruaru-admin-reaxt --bin serve -- examples/aruaru-admin/pkg 8770
# → http://127.0.0.1:8770/
```
テスト: `cargo test --workspace`(全件成功を確認済み)。

## 到達点(2026-11-18)
- 段階0〜5まで実装(ルート規約・SSR・RPoem接続・ハイドレーション・Islands・Server Actions・CLI・フォーム対応)。
- aruaru-admin 全10画面を移植。管理操作は許可リスト11メソッドだけ中継(`serve.rs`)。
- serde / serde_json をwasmから排除。RS-JSON `light` に `parse_light_secure`・`quote_json`・`to_json`・`as_object`・`as_i64` を追加(RS-JSON `9aca39a`)。`reaxt-actions` は `Wire` トレイト。
- 元のReact版との比較(実測): 元(`aruaru-db-admin-web`)は JS 208KB(gzip 64KB)+CSS 16KB(gzip 3.7KB)、Reaxt版は wasm 366KB(gzip 113KB)+JS 26KB(gzip 5.4KB)。**転送量は約1.8倍**。

## 元アプリ(React)をビルドする手順(比較用)
`aruaru-admin/package.json` の `packageManager: pnpm@9.0.0` があると新しいpnpmは古い版の取得で失敗する。**リポジトリを汚さないよう、作業用コピーで**その行を外して `pnpm install` → `pnpm web:build`(`node_modules` は.gitignore対象外なので元の場所で実行しない)。

## 未了(優先順)
1. SET への配備(Reaxt自体の配備先・手順)
2. 描画速度・初回表示の実測(元との比較は転送量だけ済み)
3. wasm縮小の続き(リリース設定は適用済み。`wasm-opt` は未導入、入れて再計測)
4. (済)`wire_struct!` マクロで名前付きフィールドの構造体は1行で Wire 実装(enum・任意フィールド・入れ子の自動化は未了)
5. 元の画面の再現不足(ウィザードの段階表示、スライダー、CPU棒グラフ、サンプル表示)
6. RPoem のワイルドカードルート(現状 `*slug` は深さ8まで展開)
7. 破壊的操作(復元・移行成功系・ノード追加)の実機確認

## 注意
- 動作確認で作る一時ディレクトリ・サーバーは後始末する(`taskkill` でプロセス終了)。バックアップ作成などの確認用データは一時の `--data` に閉じ込める。
- トークン(`ARUARU_DB_ADMIN_TOKEN`)は表示・コミットしない。
- pushする場合は RS-JSON → Reaxt の順(Reaxt は RS-JSON の新APIに依存)。

## 再開用メッセージ(別アカウントでそのまま貼り付け)
前のセッションの続きです。`F:\Reaxt\PORTING.md` と `CLAUDE.md` を読んでから再開します。aruaru-admin の全10画面を移植し、wasmからserde系を外しました(Reaxt `cb5f3b1`、RS-JSON `9aca39a`)。元のReact版との実測では転送量がReaxt版で約1.8倍で、サイズでは未勝利です。未了は PORTING.md の「未了(優先順)」の順に進めてください。決まり: RESTを作らない、JS/TSを書かない、秘密は表示しない、実ブラウザと実サーバーで確認してから完了と報告する、返答は日本語のみ、pushは確認してから。
