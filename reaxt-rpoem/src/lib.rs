//! Reaxt の `App` を RPoem の `Route` に載せるアダプタ。
//!
//! RPoem のルーターはセグメント数が一致するパターンしか照合できず(ワイルドカード・フォールバック無し)、
//! Reaxt の `*slug`(残り全部)は深さ 1..=MAX_CATCH_ALL_DEPTH の `:a1/:a2/...` に展開して登録する。
//! 実際の描画は常に要求の実パスを `App::handle` に渡すので、パラメータ名の違いは影響しない。
use open_runo_poem_compat::{get, handler_fn, hyper_compat, Route};
use reaxt::app::App;
use std::sync::Arc;

/// `*slug` を展開する最大の深さ。
pub const MAX_CATCH_ALL_DEPTH: usize = 8;

/// Reaxt パターンを RPoem パターンの一覧へ展開する(純関数)。
pub fn expand(pattern: &str) -> Vec<String> {
    match pattern.find("/*") {
        None => vec![pattern.to_string()],
        Some(i) => {
            let prefix = &pattern[..i];
            (1..=MAX_CATCH_ALL_DEPTH)
                .map(|d| {
                    let tail: Vec<String> = (1..=d).map(|k| format!(":a{k}")).collect();
                    format!("{prefix}/{}", tail.join("/"))
                })
                .collect()
        }
    }
}

/// `App` を載せた RPoem `Route` を返す(`Server::new(..).run(route)` や `nest` にそのまま渡せる)。
pub fn route(app: Arc<App>) -> Route {
    let mut r = Route::new();
    for pat in app.patterns() {
        for rp in expand(&pat) {
            let app = app.clone();
            let h = handler_fn(move |req, _params| {
                let app = app.clone();
                async move {
                    let path = req
                        .uri()
                        .path_and_query()
                        .map(|p| p.as_str().to_string())
                        .unwrap_or_else(|| "/".to_string());
                    let res = app.handle(&path);
                    hyper::Response::builder()
                        .status(res.status)
                        .header("content-type", "text/html; charset=utf-8")
                        .header("x-content-type-options", "nosniff")
                        .body(hyper_compat::fixed_body(res.body.into()))
                        .unwrap_or_else(|_| open_runo_poem_compat::empty_status(open_runo_poem_compat::StatusCode::INTERNAL_SERVER_ERROR))
                }
            });
            r = r.at(&rp, get(h));
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use open_runo_poem_compat::testing::TestServer;
    use open_runo_poem_compat::Method;
    use reaxt::react::VNode;
    use reaxt::RenderMode;

    #[test]
    fn expands_catch_all() {
        assert_eq!(expand("/users/:id"), vec!["/users/:id"]);
        let e = expand("/docs/*p");
        assert_eq!(e.len(), MAX_CATCH_ALL_DEPTH);
        assert_eq!(e[0], "/docs/:a1");
        assert_eq!(e[2], "/docs/:a1/:a2/:a3");
    }

    #[tokio::test]
    async fn serves_reaxt_pages_over_rpoem() {
        let app = Arc::new(
            App::new("rp")
                .page("index.rs", RenderMode::Ssr, |_| VNode::element("h1").child(VNode::text("top")).build())
                .page("users/[id].rs", RenderMode::SsrHydrate, |p| {
                    VNode::element("b").child(VNode::text(p[0].1.clone())).build()
                })
                .page("docs/[...p].rs", RenderMode::Ssr, |p| {
                    VNode::element("i").child(VNode::text(p[0].1.clone())).build()
                })
                .finish(),
        );
        let srv = TestServer::start(route(app)).await;
        let top = srv.request(Method::GET, "/", &[], None).await;
        assert_eq!(top.status, 200);
        assert!(top.text().contains("<h1>top</h1>"));
        assert_eq!(top.header("x-content-type-options"), Some("nosniff"));
        let u = srv.request(Method::GET, "/users/%3C7%3E", &[], None).await;
        assert!(u.text().contains("<b>%3C7%3E</b>"));
        assert!(u.text().contains("boot.js"));
        let d = srv.request(Method::GET, "/docs/a/b/c", &[], None).await;
        assert!(d.text().contains("<i>a/b/c</i>"));
        assert_eq!(srv.request(Method::GET, "/nope", &[], None).await.status, 404);
    }
}
