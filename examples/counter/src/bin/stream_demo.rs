//! ストリーミングSSRの差し替えをブラウザで確かめる最小サーバー(チャンクを少しずつ送る)。
//! 使い方: stream_demo [ポート]  → http://127.0.0.1:PORT/
use reaxt::react::VNode;
use reaxt::stream::{chunks, slot, to_document, Pending};
use std::io::{Read, Write};

fn main() {
    let port = std::env::args().nth(1).unwrap_or_else(|| "8766".into());
    let l = std::net::TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind");
    println!("http://127.0.0.1:{port}/");
    for s in l.incoming() {
        let mut s = s.unwrap();
        let mut buf = [0u8; 1024];
        let _ = s.read(&mut buf);
        let shell = VNode::element("main")
            .child(VNode::element("h1").child(VNode::text("stream")).build())
            .child(slot(0, VNode::element("p").attr("id", "a").child(VNode::text("loading A")).build()))
            .child(slot(1, VNode::element("p").attr("id", "b").child(VNode::text("loading B")).build()))
            .build();
        let cs = chunks(
            &shell,
            vec![
                Pending::new(|| VNode::element("p").attr("id", "a").child(VNode::text("done A")).build()),
                Pending::new(|| VNode::element("p").attr("id", "b").child(VNode::text("done B <ok>")).build()),
            ],
        );
        let _ = s.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/html; charset=utf-8\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n");
        let doc = to_document(&cs);
        // 先頭(<html>〜シェル)→各差し替えチャンク→末尾 の順に、間を空けて送る
        let head = format!("<!doctype html><html><head><meta charset=\"utf-8\"></head><body>{}", cs[0]);
        let mut parts: Vec<String> = vec![head];
        parts.extend(cs[1..].iter().cloned());
        parts.push("</body></html>".into());
        assert_eq!(parts.concat(), doc);
        for p in parts {
            let _ = write!(s, "{:x}\r\n{}\r\n", p.len(), p);
            let _ = s.flush();
            std::thread::sleep(std::time::Duration::from_millis(400));
        }
        let _ = s.write_all(b"0\r\n\r\n");
    }
}
