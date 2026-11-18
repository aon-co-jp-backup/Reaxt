//! デモサーバー: SSRでカウンターを出し、wasmでハイドレーションする。
//! 使い方: serve [pkgディレクトリ] [ポート]   (pkg は wasm-bindgen --target web の出力)
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use reaxt::app::{App, RouteRule};
    use reaxt_actions::ModuleActions;
    use reaxt::react::hooks::{drop_instance, run_component};
    use reaxt::RenderMode;
    use std::sync::Arc;

    let mut a = std::env::args().skip(1);
    let pkg = a.next().unwrap_or_else(|| "examples/counter/pkg".into());
    let port = a.next().unwrap_or_else(|| "8765".into());
    let read = |f: &str| std::fs::read(format!("{pkg}/{f}")).unwrap_or_else(|e| panic!("{pkg}/{f}: {e}"));

    // サーバーが持つ状態(Server Actionが更新し、SSRが初期値として描く)
    let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let (c_page, c_act) = (counter.clone(), counter.clone());
    let app = App::new("Reaxt counter")
        .global_guard(reaxt_actions::csrf_guard)
        .module(reaxt::backend::Module::new().action(&reaxt_counter::INC, move |_, _: ()| {
            Ok(c_act.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1)
        }))
        .page("actions.rs", RenderMode::Islands, move |_| {
            reaxt_counter::actions_page(c_page.load(std::sync::atomic::Ordering::SeqCst))
        })
        .page("index.rs", RenderMode::SsrHydrate, |_| {
            // SSRではこの要求の間だけコンポーネント状態を作って捨てる(初期値0を描く)
            let t = run_component(1, reaxt_counter::view);
            drop_instance(1);
            t
        })
        .page("islands.rs", RenderMode::Islands, |_| reaxt_counter::islands_page())
        .page("portal.rs", RenderMode::Csr, |_| reaxt::react::VNode::element("div").build())
        .asset(
            "/_reaxt/boot.js",
            "text/javascript; charset=utf-8",
            b"import init from \"/_reaxt/pkg/reaxt_counter.js\";\ninit();\n".to_vec(),
        )
        .asset("/_reaxt/pkg/reaxt_counter.js", "text/javascript; charset=utf-8", read("reaxt_counter.js"))
        .asset("/_reaxt/pkg/reaxt_counter_bg.wasm", "application/wasm", read("reaxt_counter_bg.wasm"))
        .asset("/_reaxt/reaxt-jquery.js", "text/javascript; charset=utf-8", include_bytes!("../../../../clients/js/reaxt-jquery.js").to_vec())
        .asset("/_reaxt/jquery-demo.html", "text/html; charset=utf-8", include_bytes!("../../../../clients/js/demo-jquery.html").to_vec())
        .route_rule("/_reaxt/", RouteRule::default().cache(0))
        .finish();
    // Cosmo向けGraphQLサブグラフ(POST /_reaxt/graphql)を足す
    let app = reaxt_actions::subgraph::install(app);
    let l = std::net::TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind");
    println!("http://127.0.0.1:{port}/");
    reaxt::server::serve(Arc::new(app), l);
}
