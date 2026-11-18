//! RPoem上のストリーミングSSRを実際のHTTPで確かめる: `cargo run -p reaxt-rpoem --example stream [ポート]`
use open_runo_poem_compat::{Server, TcpListener};
use reaxt::app::App;
use reaxt::react::VNode;
use reaxt::stream::{chunks, slot, Pending};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::args().nth(1).and_then(|p| p.parse().ok()).unwrap_or(8767);
    let app = Arc::new(
        App::new("stream")
            .stream(
                "/live",
                Arc::new(|| {
                    let shell = VNode::element("main").child(slot(0, VNode::text("wait"))).build();
                    chunks(&shell, vec![Pending::new(|| VNode::text("ready"))])
                }),
            )
            .finish(),
    );
    println!("http://127.0.0.1:{port}/live");
    // RPoemの`run`は「起動して返す」形なので、返るJoinHandleを待ってプロセスを保つ
    let (_addr, handle) = Server::new(TcpListener::bind(([127, 0, 0, 1], port))).run(reaxt_rpoem::route(app)).await.unwrap();
    handle.await.unwrap();
}
