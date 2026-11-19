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
