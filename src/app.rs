//! ルート表+ページ関数+描画方式。ページは`Fn(&Params)->VNode`(サーバー/WASM共通の純関数)。
use crate::{matches, sort_routes, ssr, RenderMode, Route};
use rs_react::VNode;

pub type Params = Vec<(String, String)>;
type Page = Box<dyn Fn(&Params) -> VNode + Send + Sync>;

pub struct Response {
    pub status: u16,
    pub body: String,
}

#[derive(Default)]
pub struct App {
    pages: Vec<(Route, Page)>,
    title: String,
}

impl App {
    pub fn new(title: impl Into<String>) -> Self {
        App { title: title.into(), pages: Vec::new() }
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

    /// 静的ルートを動的ルートより先に照合する並びにする。登録の最後に1回呼ぶ。
    pub fn finish(mut self) -> Self {
        let mut order: Vec<Route> = self.pages.iter().map(|(r, _)| r.clone()).collect();
        sort_routes(&mut order);
        self.pages.sort_by_key(|(r, _)| {
            order.iter().position(|x| x.pattern == r.pattern).unwrap_or(usize::MAX)
        });
        self
    }

    pub fn handle(&self, path: &str) -> Response {
        let path = path.split(['?', '#']).next().unwrap_or("/");
        for (route, page) in &self.pages {
            if let Some(params) = matches(route, path) {
                let tree = page(&params);
                return Response { status: 200, body: self.document(route.mode, &tree) };
            }
        }
        let nf = VNode::element("h1").child(VNode::text("404 Not Found")).build();
        Response { status: 404, body: self.document(RenderMode::Ssr, &nf) }
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
    #[test]
    fn dispatch_params_404_and_modes() {
        let app = App::new("T<")
            .page("[slug].rs", RenderMode::Ssr, |_| VNode::element("i").build())
            .page("users/[id].rs", RenderMode::SsrHydrate, |p| {
                VNode::element("b").child(VNode::text(p[0].1.clone())).build()
            })
            .page("about.rs", RenderMode::Csr, |_| VNode::element("i").build())
            .finish();
        let r = app.handle("/users/<7>?x=1");
        assert_eq!(r.status, 200);
        assert!(r.body.contains("<b>&lt;7&gt;</b>"));
        assert!(r.body.contains("boot.js"));
        assert!(r.body.contains("<title>T&lt;</title>"));
        assert!(app.handle("/about").body.contains("<div id=\"reaxt-root\"></div>"));
        assert!(!app.handle("/x").body.contains("boot.js"));
        assert_eq!(app.handle("/a/b/c").status, 404);
    }
}
