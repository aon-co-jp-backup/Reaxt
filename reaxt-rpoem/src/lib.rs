//! Reaxt の `App` を RPoem の `Route` に載せるアダプタ。
//!
//! RPoem のルーターはセグメント数が一致するパターンしか照合できず(ワイルドカード・フォールバック無し)、
//! Reaxt の `*slug`(残り全部)は深さ 1..=MAX_CATCH_ALL_DEPTH の `:a1/:a2/...` に展開して登録する。
//! 実際の描画は常に要求の実パスを `App::handle` に渡すので、パラメータ名の違いは影響しない。
use http_body_util::{BodyExt, Limited};
use open_runo_poem_compat::{get, handler_fn, hyper_compat, post, Route};
use reaxt::app::{App, Request};
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
    // ページ(GET)
    for pat in app.patterns() {
        for rp in expand(&pat) {
            r = r.at(&rp, get(host_handler(app.clone())));
        }
    }
    // Connectメソッド(POST)。同じパスにページがあっても別メソッドとして登録される。
    for p in app.method_paths() {
        r = r.at(&p, post(host_handler(app.clone())));
    }
    r
}

fn host_handler(app: Arc<App>) -> open_runo_poem_compat::Handler {
    handler_fn(move |req, _params| {
        let app = app.clone();
        async move {
            let method = req.method().as_str().to_string();
            let path = req
                .uri()
                .path_and_query()
                .map(|p| p.as_str().to_string())
                .unwrap_or_else(|| "/".to_string());
            let headers: Vec<(String, String)> = req
                .headers()
                .iter()
                .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.as_str().to_string(), v.to_string())))
                .collect();
            // 本文は上限つきで読む(超えたら413)
            let body = match Limited::new(req.into_body(), reaxt::backend::MAX_BODY).collect().await {
                Ok(c) => String::from_utf8_lossy(&c.to_bytes()).into_owned(),
                Err(_) => {
                    return open_runo_poem_compat::empty_status(open_runo_poem_compat::StatusCode::PAYLOAD_TOO_LARGE)
                }
            };
            let res = app.dispatch(&Request { method: &method, path: &path, headers: &headers, body: &body });
            let mut b = hyper::Response::builder()
                .status(res.status)
                .header("content-type", res.content_type)
                .header("x-content-type-options", "nosniff");
            for (k, v) in &res.headers {
                b = b.header(k.as_str(), v.as_str());
            }
            b.body(hyper_compat::fixed_body(res.body.into())).unwrap_or_else(|_| {
                open_runo_poem_compat::empty_status(open_runo_poem_compat::StatusCode::INTERNAL_SERVER_ERROR)
            })
        }
    })
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
    async fn connect_post_guard_and_route_rules_over_rpoem() {
        use reaxt::app::RouteRule;
        use reaxt::backend::{Denied, Module};
        let app = Arc::new(
            App::new("be")
                .page("index.rs", RenderMode::Ssr, |_| VNode::element("p").build())
                .route_rule("/", RouteRule::default().cache(30))
                .module(
                    Module::new()
                        .guard(|c| if c.header("x-key") == Some("k") { Ok(()) } else { Err(Denied::new(401, "no key")) })
                        .method("demo.v1.Echo", "Say", |_, b| Ok(format!("{{\"got\":{b}}}"))),
                )
                .finish(),
        );
        let srv = TestServer::start(route(app)).await;
        let p = |h: &'static [(&'static str, &'static str)], body: &str| {
            srv.request(Method::POST, "/demo.v1.Echo/Say", h, Some(body.as_bytes().to_vec()))
        };
        let ok = p(&[("x-key", "k"), ("content-type", "application/json")], "{\"a\":1}").await;
        assert_eq!((ok.status, ok.text().as_str()), (200, "{\"got\":{\"a\":1}}"));
        assert_eq!(ok.header("content-type"), Some("application/json; charset=utf-8"));
        let denied = p(&[], "{}").await;
        assert_eq!(denied.status, 401);
        assert!(denied.text().contains("unauthenticated"));
        let page = srv.request(Method::GET, "/", &[], None).await;
        assert_eq!(page.header("cache-control"), Some("public, max-age=30"));
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
