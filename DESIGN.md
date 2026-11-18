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
- 段階2(済): ハイドレーション。`hydrate.rs`(Domトレイト・Patch適用・SSR DOMの採用/不一致時の作り直し・イベント委譲`data-rx-click`・Runtime)をネイティブでテスト(3件)、`wasm.rs`(web-sys)で実ブラウザ確認(SSR DOM採用・ボタン非再生成・クリックで更新・コンソールエラー0・wasm 130KB)。デモは`examples/counter`(`serve`バイナリ)、`App::asset`でboot.js/wasmを配信。
- Islands(済): `hydrate::island(name, props, render)` で静的ページ中の部品に印(`data-rx-island`/`-props`/`-scope`)を付け、ブラウザは印の部分だけをマウント(`mount_islands`)。クリックidはアイランド番号で名前空間化(同じ部品を複数置いても独立)。ネイティブテスト+実ブラウザ確認(2つのアイランドが独立、静的部分は不変、wasm 152KB)。既知の制約: 更新フラグが全体で1つなので、押したアイランドのリスナーだけがflushする前提(タイマー等の非クリック更新は未対応)。
- Server Actions(済): `reaxt-actions` クレート。`Action<I, O>`(`const`で定義し、サーバーとブラウザで共有)、サーバー側は`Module::action(&A, |ctx, input| ..)`(入力JSONはRS-JSONの`from_str_secure`で上限つき、出力は`to_string_strict`)、ブラウザ側は`call(&WebClient, &A, &input, |result| ..)`(fetch、応答のあと全アイランドを再描画)。通信はConnectのみ。CSRF対策として`csrf_guard`+`x-reaxt: 1`(カスタムヘッダ必須)。テスト3件+実ブラウザ確認(POST 200・サーバー側の状態が保たれリロード後のSSRにも反映)。`LocalClient`でテスト/SSR時の呼び出しも可能。
- 修正: 空のテキスト・隣り合うテキストはHTMLで消える/融合するためハイドレーションが不一致になっていた→`hydrate::normalize`で正規化(テスト追加)。更新フラグが全体で1つの制約は`flush_all`(全アイランドを再レンダー)で解消。
- CLI(済): `reaxt-cli`(コマンド名`reaxt`)。`reaxt new <名前> [--path|--git]`(雛形生成、空でないディレクトリには書かない、名前検査、インストール済みwasm-bindgenの版を雛形に自動で合わせる)、`reaxt build`(wasm32ビルド→版照合→`wasm-bindgen`→`pkg/`、版が違えば直し方を表示)、`reaxt dev`(ビルド→サーバー起動→`src/`の変更を監視して再ビルド・再起動、ビルド失敗中も落ちずに次の保存で再試行)、`reaxt doctor`。雛形からの`build`・`dev`・ブラウザでのハイドレーション/クリックまで実際に確認済み。
- 段階5・アプリ移植の第1弾(済): `examples/aruaru-admin`(aruaru-admin Webの枠・状態表示・ダッシュボード・コミット履歴・ブランチ)。元はReact+TypeScript+Tailwind、こちらはRustのみ(`Csr`)。画面→Server Action(`aruaru_admin.v1.Proxy/Graphql`、読み取り専用で`mutation`は403、`x-reaxt`必須)→実aruaru-serverのGraphQL。コアに`App::head`(CSS読み込み)と`wasm::every`(定期実行)を追加。実ブラウザで確認: 実データ表示・ナビ切替・5秒ごとの自動更新(別経路で作ったコミットが反映)・上流停止で「切断」表示(直前のデータは保持)・コンソールエラー0。**サイズ**: wasm 315KB(gzip約103KB)+グルーJS 23KB(gzip約5KB)。
  - 正直な評価: 移植できたのは全10画面のうち3画面で、元アプリのバンドルサイズ・描画速度との比較は未計測(この機械にpnpmが無く元をビルドできていない)。「Rust版Reactの利点が大きい」とは、まだ断定できない。得られたもの=型の共有(サーバーとブラウザで同じ`Action`/構造体)・JS/TSゼロ・BFFでトークンをブラウザに出さない構成。負担=wasmが大きめ(serde+JSON解析込み)、イベントは今のところclickのみ(入力欄・フォームは未対応)、`use_effect`相当が無く定期取得はグローバルのsetter経由。残り7画面(Query/Migrate/Backup等)は入力欄が要るので、フォーム対応(input/changeイベント)が次の課題。
- フォーム対応(済): `hydrate::on_input(id, |値| ..)`(input/textarea/select/checkbox、値つき)と`on_submit(id, ||..)`(`<form>`の送信、ブラウザ既定の送信・遷移は止める)。属性`data-rx-input`/`data-rx-submit`の登録簿方式でインラインJS無し。委譲リスナーはルートに click/input/change/submit。移植2画面目としてQuery画面(SQL/GraphQL切替・自動コミット・実行)を追加。SQLの書き込みは専用Action`ExecSql`(トークン等はサーバー側)、GraphQL側は読み取り専用のまま。実ブラウザ確認: 入力しても同じtextareaが保たれフォーカスも維持、送信でページ遷移なし、SQLが実サーバーでコミットされcommitId表示、GraphQL問い合わせ成功、mutationは403。ネイティブテスト(属性が付く・エスケープ・SQL組み立て)も追加。制約: 入力欄は非制御(状態→値の書き戻しはしない)、`on_input`のidは画面内で一意に。
- 未実装: SETへの配備、`reaxt build`の最適化(wasm-opt等)、`*slug`のRPoem側ワイルドカード。
