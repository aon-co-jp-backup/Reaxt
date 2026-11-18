//! Reaxt: ルート単位でSSR/CSR/Islandsを選べる、Rustだけのフルスタック枠組みの核。
//! 設計は DESIGN.md。ここは「ファイル規約→ルート表→描画方式」の純粋ロジック(依存ゼロ)。

pub mod react;
pub mod app;
pub mod backend;
pub mod server;
pub mod ssr;

/// ルートごとの描画方式(Nextのroute segment config / NuxtのrouteRulesに相当)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// サーバーでHTML生成のみ(JS/WASM無し)
    Ssr,
    /// サーバーで生成+WASMでハイドレーション
    SsrHydrate,
    /// 静的な殻+ブラウザ(WASM)で描画
    Csr,
    /// 静的HTML+`island`印の部分だけWASM
    Islands,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub pattern: String,
    pub mode: RenderMode,
    pub params: Vec<String>,
}

/// `pages/`配下のファイルパスからルートを作る。
/// 例: `users/[id].rs`→`/users/:id`、`index.rs`→`/`、`blog/[...slug].rs`→`/blog/*slug`。
/// 先頭`_`のファイル(部品)はルートにしない。`.rs`以外は None。
pub fn route_from_path(path: &str, mode: RenderMode) -> Option<Route> {
    let p = path.replace('\\', "/");
    let stem = p.strip_suffix(".rs")?;
    let mut segs: Vec<&str> = stem.split('/').filter(|s| !s.is_empty()).collect();
    if segs.last().map_or(false, |s| s.starts_with('_')) {
        return None;
    }
    if segs.last() == Some(&"index") {
        segs.pop();
    }
    let mut params = Vec::new();
    let mut out = String::new();
    for s in segs {
        out.push('/');
        if let Some(r) = s.strip_prefix("[...").and_then(|x| x.strip_suffix(']')) {
            out.push('*');
            out.push_str(r);
            params.push(r.to_string());
        } else if let Some(r) = s.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
            out.push(':');
            out.push_str(r);
            params.push(r.to_string());
        } else {
            out.push_str(s);
        }
    }
    if out.is_empty() {
        out.push('/');
    }
    Some(Route { pattern: out, mode, params })
}

/// 要求パスを照合し、パラメータを返す(静的が動的より優先されるよう、呼び出し側で静的順に並べる)。
pub fn matches(route: &Route, url_path: &str) -> Option<Vec<(String, String)>> {
    let pat: Vec<&str> = route.pattern.split('/').filter(|s| !s.is_empty()).collect();
    let url: Vec<&str> = url_path.split('/').filter(|s| !s.is_empty()).collect();
    let mut caps = Vec::new();
    let mut i = 0;
    for (k, p) in pat.iter().enumerate() {
        if let Some(n) = p.strip_prefix('*') {
            if k + 1 != pat.len() || i >= url.len() {
                return None;
            }
            caps.push((n.to_string(), url[i..].join("/")));
            return Some(caps);
        }
        let u = *url.get(i)?;
        if let Some(n) = p.strip_prefix(':') {
            caps.push((n.to_string(), u.to_string()));
        } else if *p != u {
            return None;
        }
        i += 1;
    }
    (i == url.len()).then_some(caps)
}

/// 静的セグメントが多いルートを先に試す並び順。
pub fn sort_routes(routes: &mut [Route]) {
    routes.sort_by_key(|r| {
        let dynamic = r.pattern.matches([':', '*']).count();
        (dynamic, std::cmp::Reverse(r.pattern.len()))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use RenderMode::*;

    #[test]
    fn file_conventions() {
        assert_eq!(route_from_path("index.rs", Ssr).unwrap().pattern, "/");
        assert_eq!(route_from_path("users/[id].rs", Csr).unwrap().pattern, "/users/:id");
        assert_eq!(route_from_path("blog\\[...slug].rs", Islands).unwrap().pattern, "/blog/*slug");
        assert!(route_from_path("_layout.rs", Ssr).is_none());
        assert!(route_from_path("a.txt", Ssr).is_none());
    }

    #[test]
    fn matching_and_priority() {
        let mut rs = vec![
            route_from_path("[a].rs", Ssr).unwrap(),
            route_from_path("about.rs", Ssr).unwrap(),
            route_from_path("docs/[...p].rs", Ssr).unwrap(),
        ];
        sort_routes(&mut rs);
        assert_eq!(rs[0].pattern, "/about");
        let m = matches(&rs[2], "/docs/a/b").or_else(|| matches(&rs[1], "/docs/a/b")).unwrap();
        assert_eq!(m, vec![("p".into(), "a/b".into())]);
        assert_eq!(matches(&rs[0], "/about"), Some(vec![]));
        assert!(matches(&rs[0], "/x/y").is_none());
    }
}
