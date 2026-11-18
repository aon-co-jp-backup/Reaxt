//! 開発用の最小HTTP/1.1サーバー(GET/HEADのみ、依存ゼロ)。本番は RPoem に載せる(段階1b)。
use crate::app::App;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

pub fn serve(app: Arc<App>, listener: TcpListener) {
    for conn in listener.incoming().flatten() {
        let app = app.clone();
        std::thread::spawn(move || {
            let _ = handle(&app, conn);
        });
    }
}

fn handle(app: &App, mut conn: TcpStream) -> std::io::Result<()> {
    conn.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut rd = BufReader::new(conn.try_clone()?);
    let mut line = String::new();
    rd.by_ref().take(8192).read_line(&mut line)?;
    let mut it = line.split_whitespace();
    let (method, target) = (it.next().unwrap_or(""), it.next().unwrap_or("/"));
    let (status, body) = if method != "GET" && method != "HEAD" {
        (405, String::from("method not allowed"))
    } else {
        let r = app.handle(target);
        (r.status, r.body)
    };
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    write!(
        conn,
        "HTTP/1.1 {status} {reason}\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nx-content-type-options: nosniff\r\nconnection: close\r\n\r\n{}",
        body.len(),
        if method == "HEAD" { "" } else { &body }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RenderMode;
    use crate::react::VNode;

    #[test]
    fn real_http_roundtrip() {
        let app = Arc::new(
            App::new("e2e")
                .page("index.rs", RenderMode::Ssr, |_| {
                    VNode::element("h1").child(VNode::text("こんにちは")).build()
                })
                .finish(),
        );
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        std::thread::spawn(move || serve(app, l));
        let get = |m: &str, p: &str| {
            let mut s = TcpStream::connect(addr).unwrap();
            write!(s, "{m} {p} HTTP/1.1\r\nhost: x\r\n\r\n").unwrap();
            let mut o = String::new();
            s.read_to_string(&mut o).unwrap();
            o
        };
        let ok = get("GET", "/");
        assert!(ok.starts_with("HTTP/1.1 200"));
        assert!(ok.contains("<h1>こんにちは</h1>"));
        assert!(get("GET", "/nope/x/y").starts_with("HTTP/1.1 404"));
        assert!(get("POST", "/").starts_with("HTTP/1.1 405"));
    }
}
