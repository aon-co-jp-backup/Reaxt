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
    const VISITS: reaxt_actions::Action<(), u32> = reaxt_actions::Action::new("demo.v1.Greeter", "GetVisits");
    // エンティティ: 型(`Visitor`)を持つので、Cosmoが `@key(fields: "id")` で他のサブグラフと結合できる
    #[derive(Debug)]
    struct VisitorKey {
        id: String,
    }
    reaxt_actions::wire_struct!(VisitorKey { id: String });
    #[derive(Debug)]
    struct Visitor {
        id: String,
        visits: u32,
    }
    reaxt_actions::wire_struct!(Visitor { id: String, visits: u32 });
    const VISITOR: reaxt_actions::Action<VisitorKey, Visitor> = reaxt_actions::Action::new("demo.v1.Greeter", "GetVisitor");
    let port = std::env::args().nth(1).unwrap_or_else(|| "8766".into());
    let visits = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let (v, v2, v3) = (visits.clone(), visits.clone(), visits.clone());
    let app = App::new("Reaxt greeter")
        .global_guard(reaxt_actions::csrf_guard)
        .module(
            reaxt::backend::Module::new()
                .action(&VISIT, move |_, _: ()| Ok(v.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1))
                // `Get*` は GraphQL の Query として公開される(読み取り用)
                .action(&VISITS, move |_, _: ()| Ok(v2.load(std::sync::atomic::Ordering::SeqCst)))
                .entity(&VISITOR, move |_, k: VisitorKey| Ok(Visitor { id: k.id, visits: v3.load(std::sync::atomic::Ordering::SeqCst) })),
        )
        .finish();
    let app = reaxt_actions::subgraph::install(app);
    let l = std::net::TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind");
    println!("http://127.0.0.1:{port}/");
    reaxt::server::serve(Arc::new(app), l);
}
