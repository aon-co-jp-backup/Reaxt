# Reaxt 設計(0.0.1・草案)

目的: JavaScript/TypeScript を使わず、Rust だけでフロント+バックエンドを書く全部入り枠組み。
RPoem(サーバー、Tomcat相当)と SET(配備・サイト群)の上で使う。部品の土台は
(RS-Reactは廃止し、2026-11-18に本リポジトリの`src/react/`へ統合済み)
RS-React(仮想DOM・diff・hooks、既存)・RS-HTML・RS-CSS。

## 取り込む良い所(出典の考え方のみ。コードは流用しない)
| 元 | 取り込む点 | Reaxt での形 |
|---|---|---|
| React | コンポーネント・hooks・仮想DOM | RS-React をそのまま利用 |
| Next.js | ファイル規約ルーティング、ルート単位の描画方式、Server Actions | `pages/`規約(実装済)、`RenderMode`、Actions は Connect 呼び出しへ |
| Nuxt | routeRules、自動import、Islands | `RenderMode::Islands`、ルート別設定表 |
| Nest.js | モジュール/DI/ガード/デコレータ的構造 | `module`単位の型付きDI、ガードはミドルウェア(RPoem) |
| (共通) | データ取得とキャッシュ | RPoem の型駆動API+キャッシュ |

## 三者ハイブリッド(トライブリッド)
ルートごとに Ssr / SsrHydrate / Csr / Islands を選ぶ。同じコンポーネントがサーバー(ネイティブ)と
ブラウザ(WASM)の両方で動く。サーバー側は RPoem、配備は SET。

## API方針との整合
画面からの呼び出しは **Connect のみ**(REST を作らない)。WASM に prost は入れない。
JSON は RS-JSON(`from_str_secure`)。

## 段階
0. (済)ルート表・ファイル規約・描画方式の型
1a. (済)SSR描画(エスケープ・on*属性遮断)+Appルーター+開発用HTTPサーバー(実HTTP往復テスト)
1b. (済)RPoem接続 `reaxt-rpoem`(実RPoemサーバーで往復テスト。RPoemのルーターにワイルドカードが無いため`*slug`は深さ8まで展開。兄弟ディレクトリのパス依存が必要)
2. WASM ハイドレーション+Islands(`island`印)
3. Server Actions = Connect 自動生成
4. `reaxt new` / `reaxt dev` / `reaxt build` CLI、SET への配備
5. 実アプリ1本(aruaru-admin Web)を移植して実測(性能・サイズ・白画面検査)

## 正直な注意
現時点は核のみ。RS-React の README は「コンポーネントモデル未着手」とあるが、
実際は hooks.rs/app.rs が既にある(README が古い)。Rust 版 React の利点は
実測前なので断定しない(段階5で判断)。

## 実装状況の追記(2026-11-18)
- Nest相当: `backend.rs` — DIコンテナ(型キー)・ガード(モジュール/全体)・Module・Connect単純呼び出し(`POST /pkg.Service/Method`、本文上限1MiB、エラーはConnectのコード名のJSON)。
- Nuxt相当: `App::route_rule(接頭辞, RouteRule)` — 描画方式の上書き・任意ヘッダ・cache-control。
- Next相当: `pages/`規約・SSR。React相当: `src/react/`(旧RS-React)。
- RPoem上で GET(ページ)/POST(Connect)とも実サーバーで確認済み(28+3テスト)。
- 未実装: WASMハイドレーション(段階2)、Islands、Server Actionsの自動生成、`*slug`のRPoem側ワイルドカード。
