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
- **2026-11-18**: aruaru-admin の全10画面を移植(ダッシュボード/コミット/ブランチ/クエリ/お引越し/バックアップ/レジストリ/並列化/統合/クラスタ)、実aruaru-serverと実ブラウザで確認。wasmからserde系を排除(RS-JSON `light` + `Wire`トレイト)。**元のReact版との実測**: 元 JS 208KB(gzip 64KB)+CSS 3.7KB に対し、Reaxt版は wasm 366KB(gzip 113KB)+JS 5.4KB で約1.8倍だったが、リリース設定(opt-level z・LTO・panic=abort・strip)で wasm 202KB(gzip 84KB)に縮小=約1.3倍(wasm-optは未導入で未適用)。描画速度は未計測。未了=SETへの配備、`wasm-opt`、`Wire` の derive、RPoem側ワイルドカード(`*slug`)、速度の実測、元アプリの「ウィザード段階表示・スライダー・CPU棒」等の再現。詳細は PORTING.md。
