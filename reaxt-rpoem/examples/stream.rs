//! RPoem上の非同期ストリーミングSSRを実際のHTTP・ブラウザで確かめる:
//! `cargo run -p reaxt-rpoem --example stream [ポート]` → /live(境界Aは1秒、Bは2秒かかる)
use open_runo_poem_compat::{Server, TcpListener};
use reaxt::app::App;
use reaxt::react::VNode;
use reaxt::stream::{slot, AsyncPending};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::args().nth(1).and_then(|p| p.parse().ok()).unwrap_or(8767);
    // 確認用: DOMの変化を時刻つきで window.__log に記録する(本番では不要)
    let probe = "<script>window.__log=[];new MutationObserver(function(){window.__log.push([Math.round(performance.now()),document.body?document.body.innerText.split(String.fromCharCode(10)).join('|'):''])}).observe(document,{childList:true,subtree:true,characterData:true});</script>";
    let app = Arc::new(
        App::new("stream")
            .head(probe)
            .stream_async(
                "/live",
                Arc::new(|| {
                    let shell = VNode::element("main")
                        .child(VNode::element("h1").child(VNode::text("shell")).build())
                        .child(slot(0, VNode::text("loading A")))
                        .child(slot(1, VNode::text("loading B")))
                        .build();
                    let p = vec![
                        AsyncPending::new(async {
                            tokio::time::sleep(Duration::from_millis(1000)).await;
                            VNode::text("done A")
                        }),
                        AsyncPending::new(async {
                            tokio::time::sleep(Duration::from_millis(2000)).await;
                            VNode::text("done B")
                        }),
                    ];
                    (shell, p)
                }),
            )
            .finish(),
    );
    println!("http://127.0.0.1:{port}/live");
    // RPoemの`run`は「起動して返す」形なので、返るJoinHandleを待ってプロセスを保つ
    let (_addr, handle) = Server::new(TcpListener::bind(([127, 0, 0, 1], port))).run(reaxt_rpoem::route(app)).await.unwrap();
    handle.await.unwrap();
}
