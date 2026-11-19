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
    let (method, target) = (it.next().unwrap_or("").to_string(), it.next().unwrap_or("/").to_string());
    let mut headers: Vec<(String, String)> = Vec::new();
    loop {
        let mut h = String::new();
        if rd.by_ref().take(8192).read_line(&mut h)? == 0 || h.trim().is_empty() || headers.len() > 64 {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let len: usize = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    let r = if len > crate::backend::MAX_BODY {
        crate::app::Response { status: 413, content_type: "text/plain; charset=utf-8", headers: vec![], bytes: None, chunks: None, streaming: None, body: "too large".into() }
    } else {
        let mut buf = vec![0u8; len];
        rd.read_exact(&mut buf)?;
        let body = String::from_utf8_lossy(&buf).into_owned();
        let r = app.dispatch(&crate::app::Request { method: &method, path: &target, headers: &headers, body: &body });
        if r.streaming.is_some() {
            // 同期ホストは非同期ストリーミングを駆動できない(待てない)ので、黙って未解決のまま返さず501にする
            crate::app::Response { status: 501, content_type: "text/plain; charset=utf-8", headers: vec![], bytes: None, chunks: None, streaming: None, body: "async streaming needs an async host (reaxt-rpoem)".into() }
        } else {
            r
        }
    };
    let reason = match r.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let mut head = format!(
        "HTTP/1.1 {} {reason}\r\ncontent-type: {}\r\ncontent-length: {}\r\nx-content-type-options: nosniff\r\nconnection: close\r\n",
        r.status,
        r.content_type,
        r.bytes.as_ref().map_or(r.body.len(), |b| b.len())
    );
    for (k, v) in &r.headers {
        if !k.contains(['\r', '\n']) && !v.contains(['\r', '\n']) {
            head.push_str(&format!("{k}: {v}\r\n"));
        }
    }
    conn.write_all(format!("{head}\r\n").as_bytes())?;
    if method != "HEAD" {
        match &r.bytes {
            Some(b) => conn.write_all(b)?,
            None => conn.write_all(r.body.as_bytes())?,
        }
    }
    Ok(())
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
