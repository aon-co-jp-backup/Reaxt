//! ルート表(Next風のファイル規約)+routeRules(Nuxt風)+バックエンド(Nest風)を1つにまとめる。
//! ページは`Fn(&Params)->VNode`(サーバー/WASM共通の純関数)。
use crate::backend::{Backend, Module};
use crate::react::VNode;
use crate::{matches, sort_routes, ssr, RenderMode, Route};

pub type Params = Vec<(String, String)>;
type Page = Box<dyn Fn(&Params) -> VNode + Send + Sync>;

pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// Nuxtの`routeRules`相当: パス接頭辞ごとの上書き。`/`終わりでなければ完全一致または`/`区切りの接頭辞。
#[derive(Default, Clone)]
pub struct RouteRule {
    pub mode: Option<RenderMode>,
    pub headers: Vec<(String, String)>,
    /// 指定すると`cache-control: public, max-age=N`を付ける(0なら`no-store`)。
    pub cache_secs: Option<u32>,
}

impl RouteRule {
    pub fn mode(mut self, m: RenderMode) -> Self {
        self.mode = Some(m);
        self
    }
    pub fn header(mut self, k: &str, v: &str) -> Self {
        self.headers.push((k.into(), v.into()));
        self
    }
    pub fn cache(mut self, secs: u32) -> Self {
        self.cache_secs = Some(secs);
        self
    }
}

pub struct Request<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub headers: &'a [(String, String)],
    pub body: &'a str,
}

#[derive(Default)]
pub struct App {
    pages: Vec<(Route, Page)>,
    rules: Vec<(String, RouteRule)>,
    backend: Backend,
    title: String,
}

impl App {
    pub fn new(title: impl Into<String>) -> Self {
        App { title: title.into(), ..Default::default() }
    }

    /// `pages/`規約のファイルパス(例 `users/[id].rs`)でページを登録する。
    pub fn page(
        mut self,
        file: &str,
        mode: RenderMode,
        f: impl Fn(&Params) -> VNode + Send + Sync + 'static,
    ) -> Self {
        let r = crate::route_from_path(file, mode)
            .unwrap_or_else(|| panic!("ページ規約に合わないパス: {file}"));
        self.pages.push((r, Box::new(f)));
        self
    }

    /// routeRules(接頭辞が長いものが優先)。
    pub fn route_rule(mut self, prefix: &str, rule: RouteRule) -> Self {
        self.rules.push((prefix.to_string(), rule));
        self
    }

    /// Nestのモジュール(provider・ガード・Connectメソッド)を登録する。
    pub fn module(mut self, m: Module) -> Self {
        self.backend.register(m);
        self
    }

    pub fn global_guard(
        mut self,
        g: impl Fn(&crate::backend::Ctx) -> Result<(), crate::backend::Denied> + Send + Sync + 'static,
    ) -> Self {
        self.backend.global_guard(g);
        self
    }

    /// 静的ルートを動的ルートより先に照合する並びにする。登録の最後に1回呼ぶ。
    pub fn finish(mut self) -> Self {
        let mut order: Vec<Route> = self.pages.iter().map(|(r, _)| r.clone()).collect();
        sort_routes(&mut order);
        self.pages.sort_by_key(|(r, _)| {
            order.iter().position(|x| x.pattern == r.pattern).unwrap_or(usize::MAX)
        });
        self.rules.sort_by_key(|(p, _)| std::cmp::Reverse(p.len()));
        self
    }

    /// ページのパターン(`/users/:id`、`/docs/*p`)。ホスト側(RPoem等)のルート登録用。
    pub fn patterns(&self) -> Vec<String> {
        self.pages.iter().map(|(r, _)| r.pattern.clone()).collect()
    }

    /// Connectメソッドのパス(`/pkg.Service/Method`)。
    pub fn method_paths(&self) -> Vec<String> {
        self.backend.paths()
    }

    fn rule_for(&self, path: &str) -> Option<&RouteRule> {
        self.rules.iter().find_map(|(p, r)| {
            let ok = path == p
                || (p.ends_with('/') && path.starts_with(p.as_str()))
                || path.strip_prefix(p.as_str()).map_or(false, |rest| rest.starts_with('/'));
            ok.then_some(r)
        })
    }

    /// GETでページを返す簡易入口(テスト・開発サーバー用)。
    pub fn handle(&self, path: &str) -> Response {
        self.dispatch(&Request { method: "GET", path, headers: &[], body: "" })
    }

    /// 全入口: Connectメソッド(POST)→ページ(GET/HEAD)→404。
    pub fn dispatch(&self, req: &Request) -> Response {
        if let Some((status, json)) = self.backend.call(req.method, req.path, req.headers, req.body) {
            return Response {
                status,
                content_type: "application/json; charset=utf-8",
                headers: vec![("cache-control".into(), "no-store".into())],
                body: json,
            };
        }
        let path = req.path.split(['?', '#']).next().unwrap_or("/");
        let rule = self.rule_for(path);
        let mut headers: Vec<(String, String)> = Vec::new();
        if let Some(r) = rule {
            headers.extend(r.headers.clone());
            if let Some(s) = r.cache_secs {
                headers.push((
                    "cache-control".into(),
                    if s == 0 { "no-store".into() } else { format!("public, max-age={s}") },
                ));
            }
        }
        if req.method != "GET" && req.method != "HEAD" {
            return Response { status: 405, content_type: "text/plain; charset=utf-8", headers, body: "method not allowed".into() };
        }
        for (route, page) in &self.pages {
            if let Some(params) = matches(route, path) {
                let mode = rule.and_then(|r| r.mode).unwrap_or(route.mode);
                let tree = page(&params);
                return Response { status: 200, content_type: "text/html; charset=utf-8", headers, body: self.document(mode, &tree) };
            }
        }
        let nf = VNode::element("h1").child(VNode::text("404 Not Found")).build();
        Response { status: 404, content_type: "text/html; charset=utf-8", headers, body: self.document(RenderMode::Ssr, &nf) }
    }

    fn document(&self, mode: RenderMode, tree: &VNode) -> String {
        let mut t = String::new();
        ssr::escape(&self.title, &mut t);
        // Ssr=JS/WASM無し。それ以外はハイドレーション用の読み込み口(実体は段階2で生成)
        let script = match mode {
            RenderMode::Ssr => "",
            _ => "<script type=\"module\" src=\"/_reaxt/boot.js\"></script>",
        };
        let body = match mode {
            RenderMode::Csr => "<div id=\"reaxt-root\"></div>".to_string(),
            _ => format!("<div id=\"reaxt-root\">{}</div>", ssr::render(tree)),
        };
        format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{t}</title></head><body data-reaxt-mode=\"{mode:?}\">{body}{script}</body></html>"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Denied;

    fn app() -> App {
        App::new("T<")
            .page("[slug].rs", RenderMode::Ssr, |_| VNode::element("i").build())
            .page("users/[id].rs", RenderMode::SsrHydrate, |p| {
                VNode::element("b").child(VNode::text(p[0].1.clone())).build()
            })
            .page("about.rs", RenderMode::Csr, |_| VNode::element("i").build())
            .page("admin/index.rs", RenderMode::Ssr, |_| VNode::element("i").build())
            .route_rule("/admin", RouteRule::default().header("x-robots-tag", "noindex").cache(0))
            .route_rule("/users", RouteRule::default().mode(RenderMode::Ssr).cache(60))
            .module(Module::new().method("demo.v1.Hello", "Say", |_, b| {
                if b.is_empty() { Err(Denied::new(400, "empty")) } else { Ok(format!("{{\"echo\":{b}}}")) }
            }))
            .finish()
    }

    #[test]
    fn dispatch_params_404_and_modes() {
        let app = app();
        let r = app.handle("/users/<7>?x=1");
        assert_eq!(r.status, 200);
        assert!(r.body.contains("<b>&lt;7&gt;</b>"));
        assert!(r.body.contains("<title>T&lt;</title>"));
        assert!(app.handle("/about").body.contains("<div id=\"reaxt-root\"></div>"));
        assert!(!app.handle("/x").body.contains("boot.js"));
        assert_eq!(app.handle("/a/b/c").status, 404);
    }

    #[test]
    fn route_rules_override_mode_headers_cache() {
        let app = app();
        let u = app.handle("/users/1");
        assert!(!u.body.contains("boot.js"), "routeRulesでSsrに上書きされ、JS無し");
        assert!(u.headers.contains(&("cache-control".into(), "public, max-age=60".into())));
        let a = app.handle("/admin");
        assert!(a.headers.contains(&("x-robots-tag".into(), "noindex".into())));
        assert!(a.headers.contains(&("cache-control".into(), "no-store".into())));
        assert!(app.handle("/administrator").headers.is_empty(), "接頭辞は/区切りのみ");
    }

    #[test]
    fn connect_method_and_405_for_pages() {
        let app = app();
        let post = |p: &str, b: &str| app.dispatch(&Request { method: "POST", path: p, headers: &[], body: b });
        let ok = post("/demo.v1.Hello/Say", "\"hi\"");
        assert_eq!((ok.status, ok.body.as_str()), (200, "{\"echo\":\"hi\"}"));
        assert!(ok.content_type.starts_with("application/json"));
        assert_eq!(post("/demo.v1.Hello/Say", "").status, 400);
        assert_eq!(post("/about", "x").status, 405);
        assert_eq!(app.method_paths(), vec!["/demo.v1.Hello/Say"]);
    }
}
