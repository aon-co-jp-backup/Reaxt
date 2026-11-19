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

## 2026-11-19 追記: 連携の追加と未了
追加: JS/TSクライアント・jQueryプラグイン・Cosmo向けGraphQLサブグラフ・`reaxt-db`(詳細はREADME末尾と CLAUDE.md)。
未了(優先順): (1) 実Cosmo Router で `wgc` による合成と実行を確認する(今はcurlまで)、(2) 実PostgreSQL/CockroachDBへの接続確認とTLS対応、(3) MySQL・Oracle・SQL Server・Snowflake のドライバ(今は束縛の方言のみ。MySQLのワイヤは `caching_sha2_password` にTLSかRSAが要る、Snowflakeは独自のHTTPS API、Oracleは独自のTNSで難所)、(4) (済)接続プール(PgDbのみ。他DBは未実装)、(5) サブグラフの引数リテラル対応・Federationの`@key`エンティティ、(6) `pkg/` の再ビルド(`wire_struct!`反映)。


---

# 再開の手順と次の作業 / How to Resume and Next Steps (10 languages)

> 日本語・英語以外は AI 翻訳をネイティブらしい表現に整えたもので、母語話者による確認はまだです。 / Apart from Japanese and English, these are AI translations polished toward natural phrasing, not yet reviewed by native speakers.

## 日本語
**再開のしかた**: `RS-JSON` と `RPoem` を同じ階層の兄弟ディレクトリに置き、`cargo test --workspace` を実行して全件成功を確認します。実サーバーを使う試験は環境変数で有効にします(`REAXT_LIVE_ARUARU`、`REAXT_LIVE_PG`)。
**次の作業(この順)**: 実 Cosmo Router での合成 → 実 PostgreSQL/CockroachDB と TLS → MySQL/Oracle/Snowflake ドライバ → SET への配備 → `wasm-opt` と速度の実測。

**追加(2026-11-19)**: フック(`use_ref`・`use_memo`・`use_reducer`・`use_effect` ほか)、コンテキスト、`memo`、Fragment、Suspense、エラー境界、Solid型シグナル(`batch`込み)、Vue風・Preact風の呼び名、`wasm::portal`。ストリーミングSSRは**本物の非同期**: シェルを待たずに先に送り、各境界を並行に走らせて終わった順に差し替え、期限を超えたら代替表示に切り替える(`App::stream_async`、`reaxt-rpoem`が実HTTPで送出。実ブラウザでシェル15ms・A 1023ms・B 2015msの順に表示されるのを確認)。同期ホストは501を返す。未確認: クライアント切断時の中止の自動テスト、Preactの`Component`クラス(非対応)。

## English
**How to resume**: put `RS-JSON` and `RPoem` next to this repository as sibling directories and run `cargo test --workspace`; every test should pass. Tests that need a live server are switched on with environment variables (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Next, in this order**: compose with a real Cosmo Router, connect to real PostgreSQL/CockroachDB over TLS, write the MySQL/Oracle/Snowflake drivers, deploy on SET, then add `wasm-opt` and measure speed.

**Added (2026-11-19)**: hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` and more), context, `memo`, Fragment, Suspense, error boundaries, Solid-style signals (with `batch`), Vue- and Preact-style names, and `wasm::portal`. Streaming SSR is now **truly asynchronous**: the shell goes out immediately without waiting, every boundary runs concurrently and is swapped in as it finishes, and a boundary that exceeds its deadline falls back to a substitute (`App::stream_async`, sent over real HTTP by `reaxt-rpoem`; in a real browser the shell showed at 15 ms, A at 1023 ms and B at 2015 ms). A synchronous host answers 501. Not yet verified: an automated test for aborting on client disconnect; Preact's `Component` class is not supported.

## 简体中文
**如何继续**:把 `RS-JSON` 和 `RPoem` 放在与本仓库同级的目录下,运行 `cargo test --workspace`,确认全部测试通过。需要真实服务器的测试通过环境变量开启(`REAXT_LIVE_ARUARU`、`REAXT_LIVE_PG`)。
**接下来依次要做**:与真实的 Cosmo Router 联调 → 通过 TLS 连接真实的 PostgreSQL/CockroachDB → 实现 MySQL/Oracle/Snowflake 驱动 → 部署到 SET → 引入 `wasm-opt` 并实测速度。

**新增(2026-11-19)**:钩子(`use_ref`、`use_memo`、`use_reducer`、`use_effect` 等)、上下文、`memo`、Fragment、Suspense、错误边界、Solid 风格的信号(含 `batch`)、Vue 和 Preact 风格的命名,以及 `wasm::portal`。流式 SSR 现在是**真正的异步**:外壳不等待、立即发出,各个边界并发执行并按完成顺序替换,超过期限则切换为备用内容(`App::stream_async`,由 `reaxt-rpoem` 通过真实 HTTP 发送;在真实浏览器中,外壳 15 毫秒、A 1023 毫秒、B 2015 毫秒依次显示)。同步宿主返回 501。尚未验证:客户端断开时中止的自动化测试;不支持 Preact 的 `Component` 类。

## 繁體中文
**如何繼續**:將 `RS-JSON` 與 `RPoem` 放在與本儲存庫同層的目錄下,執行 `cargo test --workspace`,確認所有測試皆通過。需要真實伺服器的測試以環境變數啟用(`REAXT_LIVE_ARUARU`、`REAXT_LIVE_PG`)。
**接下來依序要做**:與真實的 Cosmo Router 整合測試 → 透過 TLS 連線真實的 PostgreSQL/CockroachDB → 實作 MySQL/Oracle/Snowflake 驅動程式 → 部署到 SET → 導入 `wasm-opt` 並實測速度。

**新增(2026-11-19)**:Hook(`use_ref`、`use_memo`、`use_reducer`、`use_effect` 等)、Context、`memo`、Fragment、Suspense、錯誤邊界、Solid 風格的 Signal(含 `batch`)、Vue 與 Preact 風格的命名,以及 `wasm::portal`。串流 SSR 現已是**真正的非同步**:外殼不等待、立即送出,各個邊界並行執行並依完成順序替換,超過期限則改為備用內容(`App::stream_async`,由 `reaxt-rpoem` 透過真實 HTTP 送出;在真實瀏覽器中,外殼 15 毫秒、A 1023 毫秒、B 2015 毫秒依序顯示)。同步主機會回傳 501。尚未驗證:用戶端中斷連線時中止的自動化測試;不支援 Preact 的 `Component` 類別。

## 한국어
**이어서 작업하는 방법**: `RS-JSON`과 `RPoem`을 이 저장소와 같은 상위 디렉터리에 나란히 두고 `cargo test --workspace`를 실행해 모든 테스트가 통과하는지 확인합니다. 실제 서버가 필요한 테스트는 환경 변수(`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`)로 켭니다.
**다음 작업(순서대로)**: 실제 Cosmo Router와 통합 → TLS로 실제 PostgreSQL/CockroachDB 연결 → MySQL/Oracle/Snowflake 드라이버 구현 → SET 배포 → `wasm-opt` 적용 및 속도 측정.

**추가(2026-11-19)**: 훅(`use_ref`, `use_memo`, `use_reducer`, `use_effect` 등), 컨텍스트, `memo`, Fragment, Suspense, 에러 경계, Solid 방식의 시그널(`batch` 포함), Vue·Preact 방식의 이름, 그리고 `wasm::portal`을 갖추었습니다. 스트리밍 SSR은 이제 **진짜 비동기**입니다. 셸은 기다리지 않고 먼저 내보내고, 각 경계는 동시에 실행되어 끝나는 순서대로 교체되며, 기한을 넘기면 대체 내용으로 바뀝니다(`App::stream_async`, `reaxt-rpoem`이 실제 HTTP로 전송. 실제 브라우저에서 셸 15ms, A 1023ms, B 2015ms 순으로 표시되는 것을 확인). 동기 호스트는 501을 반환합니다. 아직 확인하지 못한 것: 클라이언트 연결이 끊겼을 때 중단하는 자동 테스트, Preact의 `Component` 클래스(미지원).

## Deutsch
**So geht es weiter**: `RS-JSON` und `RPoem` als Geschwisterverzeichnisse neben dieses Repository legen und `cargo test --workspace` ausführen; alle Tests müssen bestehen. Tests, die einen laufenden Server brauchen, werden über Umgebungsvariablen aktiviert (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Nächste Schritte in dieser Reihenfolge**: Zusammenspiel mit einem echten Cosmo Router, Anbindung an echtes PostgreSQL/CockroachDB über TLS, Treiber für MySQL/Oracle/Snowflake, Bereitstellung auf SET, danach `wasm-opt` und Geschwindigkeitsmessungen.

**Neu (2026-11-19)**: Hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` u. a.), Context, `memo`, Fragment, Suspense, Error Boundaries, Signale nach Solid-Art (mit `batch`), Bezeichnungen im Stil von Vue und Preact sowie `wasm::portal`. Streaming-SSR ist jetzt **wirklich asynchron**: Die Shell wird sofort ausgeliefert, ohne zu warten; alle Grenzen laufen parallel und werden in der Reihenfolge ihrer Fertigstellung eingesetzt; wer die Frist überschreitet, wird durch einen Ersatzinhalt ersetzt (`App::stream_async`, von `reaxt-rpoem` über echtes HTTP gesendet; im echten Browser erschien die Shell nach 15 ms, A nach 1023 ms und B nach 2015 ms). Ein synchroner Host antwortet mit 501. Noch nicht geprüft: ein automatischer Test für den Abbruch bei Verbindungsabbruch des Clients; die `Component`-Klasse von Preact wird nicht unterstützt.

## Français
**Pour reprendre** : placer `RS-JSON` et `RPoem` à côté de ce dépôt, dans des répertoires frères, puis lancer `cargo test --workspace` ; tous les tests doivent passer. Les tests qui nécessitent un vrai serveur s'activent par des variables d'environnement (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Étapes suivantes, dans cet ordre** : intégration avec un vrai Cosmo Router, connexion à de vrais PostgreSQL/CockroachDB via TLS, écriture des pilotes MySQL/Oracle/Snowflake, déploiement sur SET, puis `wasm-opt` et mesures de vitesse.

**Ajouts (2026-11-19)** : hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect`, etc.), contexte, `memo`, Fragment, Suspense, limites d'erreur, signaux à la Solid (avec `batch`), noms dans le style de Vue et de Preact, et `wasm::portal`. Le SSR en streaming est désormais **réellement asynchrone** : la coquille part immédiatement sans attendre, chaque frontière s'exécute en parallèle et se substitue dans l'ordre d'achèvement, et une frontière qui dépasse son délai est remplacée par un contenu de repli (`App::stream_async`, envoyé en vrai HTTP par `reaxt-rpoem` ; dans un vrai navigateur, la coquille s'est affichée à 15 ms, A à 1023 ms et B à 2015 ms). Un hôte synchrone répond 501. Pas encore vérifié : un test automatique de l'abandon lors d'une déconnexion du client ; la classe `Component` de Preact n'est pas prise en charge.

## Español
**Cómo retomar el trabajo**: coloca `RS-JSON` y `RPoem` junto a este repositorio, como directorios hermanos, y ejecuta `cargo test --workspace`; todas las pruebas deben pasar. Las pruebas que necesitan un servidor real se activan con variables de entorno (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Siguientes pasos, por orden**: integración con un Cosmo Router real, conexión a PostgreSQL/CockroachDB reales mediante TLS, controladores de MySQL/Oracle/Snowflake, despliegue en SET y, después, `wasm-opt` y mediciones de velocidad.

**Novedades (2026-11-19)**: hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` y más), contexto, `memo`, Fragment, Suspense, límites de error, señales al estilo de Solid (con `batch`), nombres al estilo de Vue y Preact, y `wasm::portal`. El SSR en streaming ahora es **realmente asíncrono**: el esqueleto se envía de inmediato sin esperar, cada frontera se ejecuta en paralelo y se sustituye según termina, y si una frontera supera su plazo se cambia por un contenido alternativo (`App::stream_async`, enviado por HTTP real mediante `reaxt-rpoem`; en un navegador real el esqueleto apareció a los 15 ms, A a los 1023 ms y B a los 2015 ms). Un host síncrono responde 501. Aún sin verificar: una prueba automática de la cancelación cuando el cliente se desconecta; la clase `Component` de Preact no es compatible.

## Português
**Como retomar**: coloque `RS-JSON` e `RPoem` ao lado deste repositório, como diretórios irmãos, e execute `cargo test --workspace`; todos os testes devem passar. Os testes que exigem um servidor real são ativados por variáveis de ambiente (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Próximos passos, em ordem**: integração com um Cosmo Router real, conexão a PostgreSQL/CockroachDB reais via TLS, drivers de MySQL/Oracle/Snowflake, implantação no SET e, por fim, `wasm-opt` e medições de velocidade.

**Novidades (2026-11-19)**: hooks (`use_ref`, `use_memo`, `use_reducer`, `use_effect` e outros), contexto, `memo`, Fragment, Suspense, limites de erro, sinais no estilo Solid (com `batch`), nomes no estilo Vue e Preact, e `wasm::portal`. O SSR com streaming agora é **realmente assíncrono**: o esqueleto é enviado imediatamente, sem esperar; cada fronteira roda em paralelo e é substituída na ordem em que termina, e uma fronteira que ultrapassa o prazo é trocada por um conteúdo alternativo (`App::stream_async`, enviado por HTTP real pelo `reaxt-rpoem`; em um navegador real, o esqueleto apareceu aos 15 ms, o A aos 1023 ms e o B aos 2015 ms). Um host síncrono responde 501. Ainda não verificado: um teste automático do cancelamento quando o cliente se desconecta; a classe `Component` do Preact não é compatível.

## Русский
**Как продолжить работу**: поместите `RS-JSON` и `RPoem` рядом с этим репозиторием, в соседние каталоги, и выполните `cargo test --workspace` — все тесты должны пройти. Тесты, которым нужен настоящий сервер, включаются переменными окружения (`REAXT_LIVE_ARUARU`, `REAXT_LIVE_PG`).
**Дальнейшие шаги по порядку**: интеграция с настоящим Cosmo Router, подключение к реальным PostgreSQL/CockroachDB по TLS, драйверы MySQL/Oracle/Snowflake, развёртывание на SET, затем `wasm-opt` и замеры скорости.
**Добавлено (2026-11-19)**: хуки (`use_ref`, `use_memo`, `use_reducer`, `use_effect` и другие), контекст, `memo`, Fragment, Suspense, границы ошибок, сигналы в стиле Solid (с `batch`), названия в стиле Vue и Preact, а также `wasm::portal`. Потоковый SSR теперь **по-настоящему асинхронный**: оболочка отправляется сразу, не дожидаясь ничего, каждая граница выполняется параллельно и подставляется по мере готовности, а граница, превысившая срок, заменяется запасным содержимым (`App::stream_async`, отправка по настоящему HTTP через `reaxt-rpoem`; в настоящем браузере оболочка появилась через 15 мс, A — через 1023 мс, B — через 2015 мс). Синхронный хост отвечает 501. Ещё не проверено: автоматический тест прерывания при отключении клиента; класс `Component` из Preact не поддерживается.


## 2026-11-19 更新: 複数サブグラフにまたがる問い合わせ(3姉妹共通) / Cross-subgraph queries (shared by the three sisters)

### 日本語
`open-cosmo` のゲートウェイが、複数のサブグラフ(別々のサーバー)にまたがる問い合わせを扱えるようになりました。`Reaxt`・`RPoRea`・`open-cosmo` は常に同時に開発しているため、3つのリポジトリに同じ内容を書いています。

- **分割と統合**: 読み取り専用の問い合わせで、最上位のフィールドが別々のサブグラフにあるときは、サブグラフごとの問い合わせに分け、同時に実行して、結果を1つにまとめます。
- **`@key` による結合(1段)**: `@key` を持つ型について、サブグラフAが返した `id` を使い、サブグラフBの `_entities` に問い合わせて、Aにないフィールドを足します。
- **従来どおり拒否するもの**: 変更系(mutation)が複数のサブグラフにまたがる問い合わせ、リモートとローカルが混ざる問い合わせ、フラグメントやディレクティブを含む分割対象の問い合わせ。
- **確認済み**: 擬似サブグラフでの試験(gateway 44件、federation 13件)に成功。Reaxt の実サーバーが返す SDL も、`compose` と管理 RPC `ComposeFederation` で受け付けられます。
- **未了**: 複数段にわたる入れ子の結合、`@requires` / `@provides`、実際の Reaxt サーバーを複数並べた通し確認(今のサンプルは同じフィールド名を持つため、振り分けの確認にはサンプルの追加が必要)。

### English
The `open-cosmo` gateway can now handle queries that span several subgraphs (separate servers). `Reaxt`, `RPoRea` and `open-cosmo` are always developed together, so all three repositories carry this same note.

- **Split and merge**: for a read-only query whose top-level fields live in different subgraphs, the gateway builds one sub-query per subgraph, runs them concurrently, and merges the results.
- **`@key` joins (one level)**: for a type that has `@key`, the gateway takes the `id` returned by subgraph A and asks subgraph B, through `_entities`, for the fields A does not have.
- **Still rejected**: mutations that span subgraphs, queries that mix remote and local fields, and split queries that contain fragments or directives.
- **Verified**: tests against fake subgraphs pass (gateway 44, federation 13). The SDL served by a real Reaxt server is accepted by `compose` and by the admin RPC `ComposeFederation`.
- **Open**: joins nested more than one level, `@requires` / `@provides`, and a run with several real Reaxt servers (the current example exposes the same field names, so a second example is needed to check routing).

## 2026-11-19 追記: 実サーバーでの振り分けを確認(3姉妹共通) / Routing verified on real servers (shared by the three sisters)

### 日本語
`Reaxt` に2つ目のデモサーバー `examples/counter/src/bin/serve_greeter.rs`(GraphQL のフィールド名 `demo_v1_Greeter_Visit`)を追加し、カウンターのサーバー(`demo_v1_Counter_Inc`)と2台並べて、`open-cosmo` のゲートウェイ経由で実機確認しました。結果は次のとおりです。

- カウンターの変更はカウンターのサーバーへ、グリーターの変更はグリーターのサーバーへ、それぞれ正しく届き、数値は別々に増えました(カウンター 1→2、グリーター 1)。
- 2台にまたがる変更系(mutation)は、仕様どおり拒否されました。
- 注意: Reaxt のサブグラフは、引数を `input: $変数` の形でしか受け付けず、値は `null`(入力なし)が必要です。リテラルの引数は未対応です。
- 読み取り専用の問い合わせを2台にまたがって分割する経路は、Reaxt のサンプルが読み取りフィールドを `reaxt_ready`(共有)しか持たないため、実機では未確認です。擬似サブグラフでの試験は成功しています。

### English
We added a second Reaxt demo server, `examples/counter/src/bin/serve_greeter.rs` (GraphQL field `demo_v1_Greeter_Visit`), ran it next to the counter server (`demo_v1_Counter_Inc`), and checked both through the `open-cosmo` gateway on a real machine.

- A counter mutation reached the counter server and a greeter mutation reached the greeter server, and each counter advanced on its own (counter 1 then 2, greeter 1).
- A mutation spanning both servers was rejected, as designed.
- Note: a Reaxt subgraph accepts arguments only in the form `input: $variable`, and the value must be `null` (no input). Literal arguments are not supported yet.
- The read-only split across two servers was not checked on real servers, because the Reaxt examples expose only one read field, `reaxt_ready`, which both share. Tests against fake subgraphs pass.

**未了 / Open**: Reaxt のサブグラフの引数リテラル対応、読み取りフィールドを持つ実サンプルでの分割の通し確認、複数段の結合、`@requires` / `@provides`。 Literal arguments in the Reaxt subgraph, a real split run with an example that has read fields, multi-level joins, `@requires` / `@provides`.

## 2026-11-19 追記(続き): Reaxt のサブグラフが引数リテラルに対応(3姉妹共通) / Literal arguments in the Reaxt subgraph (shared by the three sisters)

### 日本語
`Reaxt` のサブグラフ(`reaxt-actions/src/subgraph.rs`)が、`input: $変数` に加えて **`input:` のリテラル**を受け付けるようになりました。オブジェクト・配列・文字列・数値・真偽値・`null`・列挙値、そして入れ子の中の `$変数` が使えます(例: `input: {by: 1, tags: ["x"], n: $n}`)。列挙値は文字列として渡します。壊れたリテラルはそのフィールドだけ `invalid_argument` のエラーになり、他のフィールドには影響しません。ブロック文字列(`"""`)と `input` 以外の引数名は未対応です。`Reaxt` の試験(`reaxt-actions` 10件)に成功しました。前の追記にある「リテラルは未対応」は、この更新で解消しています。

### English
The Reaxt subgraph (`reaxt-actions/src/subgraph.rs`) now accepts **literal values for `input:`** as well as `input: $variable`. Objects, lists, strings, numbers, booleans, `null`, enum values, and `$variable` references nested inside them all work (for example `input: {by: 1, tags: ["x"], n: $n}`). Enum values are passed as strings. A malformed literal fails only its own field with `invalid_argument` and leaves the other fields alone. Block strings (`"""`) and argument names other than `input` are not supported. The Reaxt tests pass (`reaxt-actions`, 10 tests). The "literals are not supported" remark in the previous note is resolved by this update.

**未了 / Open**: 読み取りフィールドを持つ実サンプルでの分割の通し確認、複数段の結合、`@requires` / `@provides`。 A real split run with an example that has read fields, multi-level joins, `@requires` / `@provides`.

## 2026-11-19 追記(続き2): `@key` 結合が入れ子に対応(3姉妹共通) / `@key` joins now work at any depth (shared by the three sisters)

### 日本語
`open-cosmo` のゲートウェイの `@key` 結合が、1段だけでなく**入れ子の段にも**対応しました。たとえば `{ me { name reviews manager { name reviews } } }` のように、`me` の中の `manager`(同じ型 `User`)でも、別のサブグラフ(reviews)にしかない `reviews` を取れます。浅い段から順に結合し、結合のために足した `id` と `__typename` は、最後に結果から取り除きます。試験はゲートウェイ全体で45件に成功しました(擬似サブグラフ)。**未了**: `@requires` / `@provides`、`@key` が複数のフィールドにまたがる入れ子の鍵、変更系の後の再取得、読み取りフィールドを持つ実際の Reaxt サンプルでの通し確認。

### English
The `@key` join in the `open-cosmo` gateway now works at **any depth of nesting**, not only one level. For example, in `{ me { name reviews manager { name reviews } } }` the `manager` inside `me` (also a `User`) can get `reviews`, which only another subgraph (reviews) defines. Joins run from the shallowest level down, and the `id` and `__typename` that were added only for the join are removed from the final result. Tests pass across the gateway (45 tests, fake subgraphs). **Open**: `@requires` / `@provides`, nested composite keys, re-fetching after a mutation, and a real run with a Reaxt example that has read fields.

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

## 2026-11-19 追記(続き4): `@external`・`@provides`・`@requires` に対応(4姉妹共通) / `@external`, `@provides` and `@requires` supported (shared by the four sisters)

### 日本語
`open-cosmo` のゲートウェイの `@key` 結合が、Apollo Federation の残りの主なディレクティブにも対応しました。

- **`@external`**: サブグラフAが `@external` と宣言したフィールドは、Aでは解決できないものとして扱い、実際に持つサブグラフBへの結合(`_entities`)の対象にします。
- **`@provides`**: 親のフィールドが `@provides(fields: "...")` で供給すると宣言したフィールドだけは、`@external` でもAに任せ、結合しません(余計な往復を減らします)。
- **`@requires`**: サブグラフBのフィールドが `@requires(fields: "a b")` で他のフィールドを必要とするとき、(1) そのフィールドが選ばれたときだけ、`a` と `b` をサブグラフAの問い合わせに足して先に取得し、(2) エンティティ表現(representation)に鍵と一緒に入れて B の `_entities` へ送ります。クライアントが選んでいない `a` と `b` は、最後に結果から取り除きます。Aが必須フィールドを供給できないときは、結合せずに従来どおり転送します。
- **設計の根拠(調査)**: Apollo の [Subgraph 仕様](https://www.apollographql.com/docs/graphos/reference/federation/subgraph-spec/)(`_entities` は表現と同じ順で返す、`@requires` のフィールドは表現に含める)と、[apollographql/router](https://github.com/apollographql/router) の計画器(必須フィールドは、要求したフィールドが選ばれたときだけ取得する修正、`FetchNode` の `requires`)を参考にしました。コードは流用せず、Rust で一から実装しています。
- **試験**: ゲートウェイ47件・federation 15件に成功(擬似サブグラフ)。
- **未了**: 必須フィールドが「さらに別のサブグラフ」にある場合(3者以上の連鎖)、`@skip` / `@include` つきの条件、変更系の後の再取得。

### English
The `@key` join in the `open-cosmo` gateway now also handles the other main Apollo Federation directives.

- **`@external`**: a field that subgraph A marks `@external` is treated as not resolvable by A, and is joined (through `_entities`) from the subgraph B that really owns it.
- **`@provides`**: a field that the parent declares as supplied by `@provides(fields: "...")` stays with A even though it is `@external`, so no extra round trip is made.
- **`@requires`**: when a field of subgraph B uses `@requires(fields: "a b")`, the gateway (1) adds `a` and `b` to the query sent to subgraph A, only when that field is selected, and (2) sends them in the entity representation together with the key to B's `_entities`. Fields the client did not select are removed from the final result. If A cannot supply a required field, the query is forwarded as before without a join.
- **Design sources (research)**: the Apollo [Subgraph specification](https://www.apollographql.com/docs/graphos/reference/federation/subgraph-spec/) (`_entities` returns entities in the order of the representations; `@requires` fields belong in the representation) and the planner in [apollographql/router](https://github.com/apollographql/router) (required fields are fetched only when the requiring field is selected; `FetchNode` has `requires`). No code was copied; this is a fresh Rust implementation.
- **Tests**: gateway 47 and federation 15 pass (fake subgraphs).
- **Open**: required fields that live in yet another subgraph (chains of three or more), `@skip` / `@include` conditions, and re-fetching after a mutation.

## 2026-11-19 追記(続き5): 変更系(mutation)が返したエンティティの結合に対応(4姉妹共通) / Joins on entities returned by a mutation (shared by the four sisters)

### 日本語
`open-cosmo` のゲートウェイが、**変更系(mutation)が返したエンティティ**にも `@key` 結合を行うようになりました。たとえば `mutation { createUser { name reviews } }` で、`createUser` は users サブグラフが実行し、`reviews`(別のサブグラフにしかないフィールド)は、返ってきた `id` を使って reviews サブグラフの `_entities`(読み取り専用)から足します。

- **変更は1回だけ**: mutation は元のサブグラフで1回だけ実行し、結合のための追加の呼び出しは読み取り専用の `_entities` だけです(二重実行しません)。試験で、users 側が `mutation` を1回だけ、reviews 側が `_entities` だけを受けたことを確認しました。
- **従来どおり拒否**: 変更系が複数のサブグラフにまたがる問い合わせは、拒否します(原子的にできないため)。
- **試験**: ゲートウェイ48件に成功(擬似サブグラフ)。
- **未了**: 必須フィールドが別のサブグラフにある3者以上の連鎖、`@skip` / `@include` つきの条件。

### English
The `open-cosmo` gateway now also performs `@key` joins on **entities returned by a mutation**. For example, in `mutation { createUser { name reviews } }` the users subgraph runs `createUser`, and `reviews` (a field only another subgraph defines) is added from the reviews subgraph's read-only `_entities`, using the returned `id`.

- **The mutation runs once**: it is executed once at its own subgraph, and the only extra calls are read-only `_entities` calls (nothing runs twice). The test confirms that the users side saw exactly one `mutation` and the reviews side saw only `_entities`.
- **Still rejected**: mutations that span several subgraphs (they cannot be made atomic).
- **Tests**: the gateway passes 48 tests (fake subgraphs).
- **Open**: chains of three or more subgraphs where a required field lives in yet another subgraph, and `@skip` / `@include` conditions.

## 2026-11-19 追記(続き6): `@skip` / `@include` と、3者以上の連鎖に対応(4姉妹共通) / `@skip` / `@include` and chains of three or more subgraphs (shared by the four sisters)

### 日本語
`open-cosmo` のゲートウェイの結合が、次の2つに対応しました。

- **`@skip(if:)` / `@include(if:)`**: 条件は、リテラルか、問い合わせの時点で分かっている変数の値で、ゲートウェイが**計画時に**判定します。除外されたフィールドは問い合わせから外し、そのフィールドのためだけに必要だった別のサブグラフへの呼び出しも行いません。これ以外のディレクティブや、値の分からない変数は、従来どおり結合せずに転送します。
- **3者以上の連鎖**: `@requires` の必須フィールドが、最初のサブグラフでも結合先でもない**さらに別のサブグラフ**にあるとき、その別のサブグラフへの結合を先に行い、得た値を結合先の表現(representation)に入れます(例: catalog → pricing で `price` を取得 → shipping の `shippingEstimate` に渡す)。結合には順位を付けて、必要な順に実行します(最大4段)。クライアントが選んでいない必須フィールドは、最後に結果から取り除きます。どのサブグラフも供給できないときは、結合せずに転送します。
- **試験**: ゲートウェイ51件に成功(擬似サブグラフ)。
- **未了**: 実際のサーバー(Reaxt 以外のサブグラフ)での `@key` / `@requires` の通し確認、`@skip` / `@include` をフラグメント内で使う場合(フラグメントは分割・結合の対象外のまま)。

### English
The join in the `open-cosmo` gateway now handles two more cases.

- **`@skip(if:)` / `@include(if:)`**: the condition is a literal or a variable whose value is already known, and the gateway decides it **at planning time**. An excluded field is dropped from the query, and a call to another subgraph that was needed only for that field is not made. Other directives, and variables with no known value, are forwarded as before, without a join.
- **Chains of three or more subgraphs**: when a `@requires` field lives in **yet another subgraph** (neither the first one nor the one being joined), the gateway first joins that third subgraph and puts the value into the representation sent onward (for example: catalog, then pricing supplies `price`, then shipping computes `shippingEstimate`). Joins get a rank and run in dependency order (up to four levels). Required fields the client did not select are removed from the final result. If no subgraph can supply a field, the query is forwarded without a join.
- **Tests**: the gateway passes 51 tests (fake subgraphs).
- **Open**: a run of `@key` / `@requires` against real servers other than Reaxt, and `@skip` / `@include` inside fragments (fragments are still outside the split and join planner).

## 2026-11-19 追記(続き7): 本物のフェデレーション実装2台で `@key` 結合を確認し、ルート型の名前の不具合を修正(4姉妹共通) / `@key` join verified against two real federation servers; root-type-name bug fixed (shared by the four sisters)

### 日本語
擬似サブグラフだけでなく、**独立した本物の GraphQL サーバー**でも確認しました。`async-graphql` のフェデレーション機能(`_service { sdl }` と `_entities` を自前で実装)で users と reviews の2台を立て、`open-cosmo` のゲートウェイ越しに `{ me { name reviews } }` を送ると、`{"name":"user-7","reviews":["review-of-7"]}` と正しく結合されました(試験はゲートウェイ52件に成功)。

- **見つかった不具合と修正**: 実際のサーバーは、ルート型の名前が `Query` とは限りません(今回は `QueryA`)。ゲートウェイは `Query` / `Mutation` 固定で探していたため、結合が働かず、元の問い合わせがそのまま転送されて失敗しました。内省(introspection)でルート型の名前を取得し、内部では `Query` / `Mutation` に正規化するようにして直しました。擬似サブグラフだけの試験では見つからなかった不具合です。
- **未了**: `@requires` と `@provides` を本物のサーバーで確認する試験(今回は `@key` のみ)、Reaxt 自身のエンティティ(`@key`)対応、フラグメント内のディレクティブ。

### English
We verified the join not only with fake subgraphs but also with **independent, real GraphQL servers**. Two servers (users and reviews) were built with `async-graphql`'s own federation support (its own `_service { sdl }` and `_entities`); sending `{ me { name reviews } }` through the `open-cosmo` gateway returned the correctly joined `{"name":"user-7","reviews":["review-of-7"]}` (the gateway passes 52 tests).

- **Bug found and fixed**: a real server's root type is not always called `Query` (here it was `QueryA`). The gateway looked for `Query` / `Mutation` by name, so no join happened and the original query was forwarded unchanged, which failed. The gateway now reads the root type names through introspection and normalizes them to `Query` / `Mutation` internally. The fake-subgraph tests alone could not have found this.
- **Open**: tests of `@requires` and `@provides` against real servers (this run covered `@key` only), entity (`@key`) support in Reaxt itself, and directives inside fragments.

## 2026-11-19 追記(続き8): 本物の SDL での `@external` / `@requires` / `@provides` の読み取りを確認、Reaxt のエンティティ対応は見送り(4姉妹共通) / Directives read from real SDL; Reaxt entities deliberately not added (shared by the four sisters)

### 日本語
- **確認**: `async-graphql` が実際に生成する SDL(`#[graphql(external)]`・`#[graphql(requires = "price")]`・`#[graphql(provides = "price")]` つき)を、ゲートウェイのモデルが正しく読むことを試験で確認しました(`@key`・`@external`・`@requires`・`@provides` の4種。ゲートウェイ53件に成功)。手書きの SDL だけでなく、実際の実装が出力する書式でも読めます。ただし、`@requires` と `@provides` を使った**結合の実行**を本物のサーバー2台で通すところまでは、まだ行っていません(実行の確認は擬似サブグラフでの試験のみ)。
- **判断(見送り)**: Reaxt 自身を `@key` のエンティティにすることは**しません**。Reaxt のアクションは `JSON` スカラーを返し、GraphQL の型(フィールドの構造)を持たないためです。型を持たせるには Reaxt のアクション定義に型情報を足す大きな設計変更が必要で、現状の用途(アクションの呼び出しと読み取り)には不要です。必要になったら、`Wire` 型から GraphQL の型を生成する形を検討します。
- **未了**: `@requires` / `@provides` を使った結合を、本物のサーバー2台で実行する試験、フラグメント内のディレクティブ。

### English
- **Verified**: the gateway model correctly reads the SDL that `async-graphql` really generates (with `#[graphql(external)]`, `#[graphql(requires = "price")]` and `#[graphql(provides = "price")]`): `@key`, `@external`, `@requires` and `@provides` (the gateway passes 53 tests). It reads the format produced by a real implementation, not only hand-written SDL. However, running a **join that uses `@requires` and `@provides`** against two real servers has not been done yet (that execution is covered only by fake-subgraph tests).
- **Decision (not added)**: Reaxt itself will **not** become a `@key` entity. Reaxt actions return the `JSON` scalar and have no GraphQL type (no field structure). Giving them one would need a large design change in the action definitions, and the current use (calling actions and reading values) does not need it. If it is ever needed, we will consider generating GraphQL types from the `Wire` types.
- **Open**: running joins that use `@requires` / `@provides` against two real servers, and directives inside fragments.


## 2026-11-19 いまの状況と、これから作る機能(4姉妹: open-cosmo・RPoem・Reaxt・RPoRea)

### 日本語(やさしい説明)
**4つは、いつも一緒に作ります。** `RPoem`は土台、`open-cosmo`は入口(玄関)、`Reaxt`は画面とサーバーの仕事を書く道具、`RPoRea`はRPoemとReaxtをくっつけたものです。1つを変えたら、ほかの3つへの影響も確かめ、4つの README・CLAUDE・PORTING・POLICY を同じ日に直します。

**もうできていること**
- `Reaxt`のサーバーが、Cosmo(複数のサーバーをまとめる仕組み)に参加できる(`POST /_reaxt/graphql`)。
- 入口(ゲートウェイ)が、1つの質問を複数のサーバーに分けて聞き、答えをまとめる。
- `@key`で、別々のサーバーが持つ同じ人・物の情報をつなぐ(入れ子や、4段までの連鎖もOK)。
- `@requires`と`@provides`(必要な項目を先に取る・持っている項目をそのまま使う)。
- `@skip`と`@include`(項目を出す・出さないの切り替え)。型条件のないフラグメントの中でも動く。
- `Reaxt`のアクションが型(項目の形)を持てる。`@key`のエンティティとして公開できる。
- 本物のサーバー2つを入口につないで、1つの質問で結合できることを確認した。

**これから作る機能(完成予定)**
1. 結合の計画(`@key`)でも、フラグメントを使えるようにする。
2. `... on 型`(型条件つきのフラグメント)に、安全な範囲で対応する。
3. 変更(書き込み)のあとに、最新の値を取り直す。
4. `Reaxt`の`Option`(あるかもしれない値)や、もっと複雑な型への対応。
5. `open-cosmo`のルーターへ、Reaxtのサーバーをもっと広く実際につなぐ試験。
6. `RPoRea`をRust以外(Pythonなど)から使う入口(C ABI)の、Mojoでの確認、Mac/Android/iPhoneへの広がり。

**進め方のきまり**: 終わっていない仕事は、**確認を取らず、自動で次々に進めます**。作ったら試験をして、本物のサーバーで確かめてから「できた」と書きます。秘密(パスワードなど)は書きません。新しいRESTは作りません。

### English (simple words, easy to read)
**We always build these four together.** `RPoem` is the base. `open-cosmo` is the front door. `Reaxt` is the tool for writing screens and server jobs. `RPoRea` joins RPoem and Reaxt. If we change one, we check the other three too, and we update the README, CLAUDE, PORTING and POLICY of all four on the same day.

**What already works**
- A `Reaxt` server can join Cosmo (a way to combine many servers) at `POST /_reaxt/graphql`.
- The front door (gateway) splits one question across many servers and puts the answers together.
- `@key` joins the same person or thing that lives in different servers (nested joins and chains up to four steps work).
- `@requires` and `@provides` (fetch needed fields first; reuse fields a server already has).
- `@skip` and `@include` (show or hide a field). They also work inside fragments that have no type condition.
- `Reaxt` actions can have types (the shape of the fields) and can be shown as `@key` entities.
- We ran two real servers behind the gateway and joined them with one question.

**What we plan to build (the finish line)**
1. Let the join planner (`@key`) use fragments too.
2. Support `... on Type` (fragments with a type condition) in a safe way.
3. Re-read the newest values after a write.
4. Support `Option` (a value that may be missing) and richer types in `Reaxt`.
5. Wider real tests that connect Reaxt servers to the `open-cosmo` router.
6. Use `RPoRea` from other languages (the C ABI): check it with Mojo, then Mac, Android and iPhone.

**How we work**: we keep going on unfinished jobs **without asking for confirmation**. We test, check on real servers, and only then say "done". We never write secrets (such as passwords). We do not add new REST APIs.


## 2026-11-19 多言語版 / Translations (8 languages; 日本語・English are above / AI翻訳、ネイティブ確認前)

### 简体中文
**这四个项目始终一起开发。** `RPoem`是地基,`open-cosmo`是大门,`Reaxt`是编写界面和服务器任务的工具,`RPoRea`把RPoem和Reaxt合在一起。改动其中一个,就要检查另外三个,并在同一天更新四个项目的README、CLAUDE、PORTING、POLICY。
- **已经完成**:Reaxt服务器可加入Cosmo(`POST /_reaxt/graphql`);网关把一个问题分给多个服务器再合并答案;`@key`连接不同服务器里的同一个人或物(可嵌套、最多4步);`@requires`、`@provides`、`@skip`、`@include`(也可在无类型条件的片段里使用);Reaxt动作可以有类型并作为`@key`实体公开;用两个真实服务器验证了一次提问合并结果。
- **计划完成**:① 连接计划支持片段(已完成);② 安全支持`... on 类型`;③ 写入后重新读取最新值;④ Reaxt支持`Option`和更复杂的类型;⑤ 更广泛的真实连接测试;⑥ 从其他语言(C ABI)使用RPoRea,先用Mojo验证,再到Mac、Android、iPhone。
- **做事规则**:未完成的工作**无需确认,自动继续**;先测试、在真实服务器上确认后才说"完成";不写入密码等机密;不新增REST。

### 繁體中文
**這四個專案永遠一起開發。** `RPoem`是地基,`open-cosmo`是大門,`Reaxt`是撰寫畫面與伺服器工作的工具,`RPoRea`把RPoem與Reaxt結合在一起。改動其中一個,就要檢查另外三個,並在同一天更新四個專案的README、CLAUDE、PORTING、POLICY。
- **已經完成**:Reaxt伺服器可加入Cosmo(`POST /_reaxt/graphql`);閘道把一個問題分給多個伺服器再合併答案;`@key`連接不同伺服器裡的同一個人或物(可巢狀、最多4步);`@requires`、`@provides`、`@skip`、`@include`(也可用於沒有型別條件的片段);Reaxt動作可以有型別並作為`@key`實體公開;以兩個真實伺服器驗證了一次提問合併結果。
- **預計完成**:① 連接計畫支援片段(已完成);② 安全支援`... on 型別`;③ 寫入後重新讀取最新值;④ Reaxt支援`Option`與更複雜的型別;⑤ 更廣泛的真實連線測試;⑥ 從其他語言(C ABI)使用RPoRea,先以Mojo驗證,再到Mac、Android、iPhone。
- **做事規則**:未完成的工作**不需確認,自動繼續**;先測試、在真實伺服器上確認後才說「完成」;不寫入密碼等機密;不新增REST。

### 한국어
**이 네 프로젝트는 항상 함께 만듭니다.** `RPoem`은 바탕, `open-cosmo`는 현관문, `Reaxt`는 화면과 서버 작업을 쓰는 도구, `RPoRea`는 RPoem과 Reaxt를 합친 것입니다. 하나를 바꾸면 나머지 셋도 확인하고, 같은 날 네 프로젝트의 README, CLAUDE, PORTING, POLICY를 고칩니다.
- **이미 된 것**: Reaxt 서버가 Cosmo에 참여(`POST /_reaxt/graphql`), 게이트웨이가 질문 하나를 여러 서버에 나눠 묻고 답을 합침, `@key`로 서로 다른 서버의 같은 사람·물건을 연결(중첩, 최대 4단계), `@requires`·`@provides`·`@skip`·`@include`(타입 조건 없는 프래그먼트 안에서도 동작), Reaxt 액션에 타입을 주고 `@key` 엔티티로 공개, 실제 서버 2대로 한 번의 질문 결합 확인.
- **만들 예정**: ① 결합 계획에서 프래그먼트 지원(완료), ② `... on 타입`의 안전한 지원, ③ 쓰기 뒤 최신 값 다시 읽기, ④ Reaxt의 `Option`과 더 복잡한 타입, ⑤ 더 넓은 실제 연결 시험, ⑥ 다른 언어(C ABI)에서 RPoRea 사용(Mojo 확인 후 Mac·Android·iPhone).
- **작업 규칙**: 끝나지 않은 일은 **확인 없이 자동으로 계속**합니다. 시험하고 실제 서버에서 확인한 뒤에만 "완료"라고 씁니다. 비밀번호 같은 비밀은 쓰지 않고, 새 REST는 만들지 않습니다.

### Deutsch
**Diese vier Projekte entwickeln wir immer zusammen.** `RPoem` ist das Fundament, `open-cosmo` die Eingangstür, `Reaxt` das Werkzeug für Oberflächen und Server-Aufgaben, `RPoRea` verbindet RPoem und Reaxt. Ändern wir eines, prüfen wir die anderen drei und aktualisieren README, CLAUDE, PORTING und POLICY aller vier am selben Tag.
- **Schon fertig**: Reaxt-Server können Cosmo beitreten (`POST /_reaxt/graphql`); das Gateway verteilt eine Frage auf mehrere Server und fügt die Antworten zusammen; `@key` verbindet dieselbe Person oder Sache aus verschiedenen Servern (verschachtelt, bis zu 4 Schritte); `@requires`, `@provides`, `@skip`, `@include` (auch in Fragmenten ohne Typbedingung); Reaxt-Aktionen haben Typen und können als `@key`-Entitäten angeboten werden; mit zwei echten Servern wurde eine Frage erfolgreich zusammengeführt.
- **Geplant**: ① Fragmente im Verbindungsplan (erledigt); ② `... on Typ` sicher unterstützen; ③ nach dem Schreiben neue Werte lesen; ④ `Option` und komplexere Typen in Reaxt; ⑤ breitere Tests mit echten Verbindungen; ⑥ RPoRea aus anderen Sprachen (C ABI) nutzen, erst Mojo, dann Mac, Android, iPhone.
- **Arbeitsregel**: Unfertiges machen wir **ohne Rückfrage automatisch weiter**. Wir testen, prüfen auf echten Servern und schreiben erst dann „fertig“. Keine Geheimnisse (z. B. Passwörter) in Dateien, keine neuen REST-APIs.

### Français
**Nous développons toujours ces quatre projets ensemble.** `RPoem` est la base, `open-cosmo` la porte d'entrée, `Reaxt` l'outil pour écrire écrans et tâches serveur, `RPoRea` réunit RPoem et Reaxt. Si l'un change, nous vérifions les trois autres et mettons à jour README, CLAUDE, PORTING et POLICY des quatre le même jour.
- **Déjà fait** : un serveur Reaxt peut rejoindre Cosmo (`POST /_reaxt/graphql`) ; la passerelle répartit une question sur plusieurs serveurs et assemble les réponses ; `@key` relie la même personne ou chose située dans des serveurs différents (imbriqué, jusqu'à 4 étapes) ; `@requires`, `@provides`, `@skip`, `@include` (aussi dans les fragments sans condition de type) ; les actions Reaxt ont des types et deviennent des entités `@key` ; une question a été assemblée avec deux vrais serveurs.
- **Prévu** : ① fragments dans le plan de jointure (fait) ; ② `... on Type` de façon sûre ; ③ relire les valeurs après une écriture ; ④ `Option` et types plus riches dans Reaxt ; ⑤ tests de connexion réelle plus larges ; ⑥ utiliser RPoRea depuis d'autres langages (C ABI) : Mojo d'abord, puis Mac, Android, iPhone.
- **Règle de travail** : ce qui n'est pas fini continue **automatiquement, sans demander**. Nous testons, vérifions sur de vrais serveurs, puis seulement écrivons « terminé ». Pas de secrets (mots de passe…) dans les fichiers, pas de nouvelle API REST.

### Español
**Siempre desarrollamos estos cuatro proyectos juntos.** `RPoem` es la base, `open-cosmo` la puerta de entrada, `Reaxt` la herramienta para escribir pantallas y tareas del servidor, y `RPoRea` une RPoem y Reaxt. Si cambiamos uno, revisamos los otros tres y actualizamos README, CLAUDE, PORTING y POLICY de los cuatro el mismo día.
- **Ya funciona**: un servidor Reaxt puede unirse a Cosmo (`POST /_reaxt/graphql`); la pasarela reparte una pregunta entre varios servidores y junta las respuestas; `@key` une la misma persona o cosa que está en servidores distintos (anidado, hasta 4 pasos); `@requires`, `@provides`, `@skip`, `@include` (también en fragmentos sin condición de tipo); las acciones de Reaxt tienen tipos y se publican como entidades `@key`; se juntó una pregunta con dos servidores reales.
- **Planeado**: ① fragmentos en el plan de unión (hecho); ② `... on Tipo` de forma segura; ③ volver a leer los valores tras escribir; ④ `Option` y tipos más ricos en Reaxt; ⑤ pruebas de conexión real más amplias; ⑥ usar RPoRea desde otros lenguajes (C ABI): primero Mojo, luego Mac, Android e iPhone.
- **Regla de trabajo**: lo que falta se sigue **automáticamente, sin pedir confirmación**. Probamos, verificamos en servidores reales y solo entonces escribimos «terminado». Sin secretos (contraseñas) en los archivos y sin nuevas APIs REST.

### Português
**Sempre desenvolvemos estes quatro projetos juntos.** `RPoem` é a base, `open-cosmo` é a porta de entrada, `Reaxt` é a ferramenta para escrever telas e tarefas do servidor, e `RPoRea` une RPoem e Reaxt. Se mudamos um, verificamos os outros três e atualizamos README, CLAUDE, PORTING e POLICY dos quatro no mesmo dia.
- **Já funciona**: um servidor Reaxt pode entrar no Cosmo (`POST /_reaxt/graphql`); o gateway divide uma pergunta entre vários servidores e junta as respostas; `@key` liga a mesma pessoa ou coisa que está em servidores diferentes (aninhado, até 4 passos); `@requires`, `@provides`, `@skip`, `@include` (também em fragmentos sem condição de tipo); as ações do Reaxt têm tipos e viram entidades `@key`; uma pergunta foi juntada com dois servidores reais.
- **Planejado**: ① fragmentos no plano de junção (feito); ② `... on Tipo` de forma segura; ③ reler os valores após escrever; ④ `Option` e tipos mais ricos no Reaxt; ⑤ testes de conexão real mais amplos; ⑥ usar o RPoRea de outras linguagens (C ABI): primeiro Mojo, depois Mac, Android e iPhone.
- **Regra de trabalho**: o que falta continua **automaticamente, sem pedir confirmação**. Testamos, verificamos em servidores reais e só então escrevemos «concluído». Sem segredos (senhas) nos arquivos e sem novas APIs REST.

### Русский
**Эти четыре проекта мы всегда делаем вместе.** `RPoem` — основа, `open-cosmo` — входная дверь, `Reaxt` — инструмент для экранов и серверных задач, `RPoRea` соединяет RPoem и Reaxt. Если меняем один, проверяем остальные три и в тот же день обновляем README, CLAUDE, PORTING и POLICY всех четырёх.
- **Уже работает**: сервер Reaxt может присоединиться к Cosmo (`POST /_reaxt/graphql`); шлюз делит один вопрос между серверами и собирает ответы; `@key` связывает одного и того же человека или предмет из разных серверов (вложенно, до 4 шагов); `@requires`, `@provides`, `@skip`, `@include` (в том числе во фрагментах без условия типа); действия Reaxt имеют типы и открываются как сущности `@key`; на двух настоящих серверах один вопрос успешно объединён.
- **Планируется**: ① фрагменты в плане соединения (готово); ② безопасная поддержка `... on Тип`; ③ перечитывание значений после записи; ④ `Option` и более сложные типы в Reaxt; ⑤ более широкие проверки с реальными соединениями; ⑥ использование RPoRea из других языков (C ABI): сначала Mojo, затем Mac, Android и iPhone.
- **Правило работы**: незавершённое мы продолжаем **автоматически, не спрашивая подтверждения**. Сначала проверяем на настоящих серверах и только потом пишем «готово». Секреты (пароли) в файлы не пишем, новых REST-API не создаём.
