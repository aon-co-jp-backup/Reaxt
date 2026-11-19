//! 2つ目のサブグラフ用のデモサーバー(Cosmoの複数サブグラフ試験用)。
//! `serve`(カウンター)とはGraphQLのフィールド名が違う(`demo_v1_Greeter_Visit`)ので、
//! ゲートウェイが「どのサーバーへ振り分けるか」を実機で確かめられる。
//! 使い方: serve_greeter [ポート]
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use reaxt::app::App;
    use reaxt_actions::ModuleActions;
    use std::sync::Arc;

    const VISIT: reaxt_actions::Action<(), u32> = reaxt_actions::Action::new("demo.v1.Greeter", "Visit");
    let port = std::env::args().nth(1).unwrap_or_else(|| "8766".into());
    let visits = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let v = visits.clone();
    let app = App::new("Reaxt greeter")
        .global_guard(reaxt_actions::csrf_guard)
        .module(reaxt::backend::Module::new().action(&VISIT, move |_, _: ()| {
            Ok(v.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1)
        }))
        .finish();
    let app = reaxt_actions::subgraph::install(app);
    let l = std::net::TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind");
    println!("http://127.0.0.1:{port}/");
    reaxt::server::serve(Arc::new(app), l);
}
