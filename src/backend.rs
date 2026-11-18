//! バックエンド層(Nest.js相当): DIコンテナ・ガード・モジュール・Connect単純呼び出し。
//!
//! API方針(REST全廃): メソッドは `POST /<パッケージ.Service>/<Method>`、本文はJSON(Connect unary)。
//! 本文のJSONの読み書きは呼び出し側(RS-JSONの`from_str_secure`等)に任せ、ここは文字列で受け渡す
//! (コアを依存ゼロに保つため)。上限は`MAX_BODY`で必ず掛ける。
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

pub const MAX_BODY: usize = 1 << 20;

/// DIコンテナ(型をキーにした共有インスタンス。Nestのprovider相当)。
#[derive(Default, Clone)]
pub struct Container(HashMap<TypeId, Arc<dyn Any + Send + Sync>>);

impl Container {
    pub fn provide<T: Send + Sync + 'static>(&mut self, v: T) {
        self.0.insert(TypeId::of::<T>(), Arc::new(v));
    }
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.0.get(&TypeId::of::<T>()).and_then(|a| a.clone().downcast::<T>().ok())
    }
}

/// ハンドラ/ガードに渡す文脈。
pub struct Ctx<'a> {
    pub container: &'a Container,
    pub headers: &'a [(String, String)],
}

impl Ctx<'_> {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.container.get::<T>()
    }
}

/// 失敗(HTTPステータス+メッセージ)。Connectのエラーコードへ対応づく。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Denied {
    pub status: u16,
    pub message: String,
}

impl Denied {
    pub fn new(status: u16, message: impl Into<String>) -> Self {
        Denied { status, message: message.into() }
    }
}

pub type Guard = Arc<dyn Fn(&Ctx) -> Result<(), Denied> + Send + Sync>;
pub type MethodFn = Arc<dyn Fn(&Ctx, &str) -> Result<String, Denied> + Send + Sync>;

struct Entry {
    handler: MethodFn,
    guards: Vec<Guard>,
}

/// Nestの`@Module`相当: provider・ガード・メソッドをまとめた単位。
#[derive(Default)]
pub struct Module {
    providers: Vec<Box<dyn FnOnce(&mut Container)>>,
    guards: Vec<Guard>,
    methods: Vec<(String, String, MethodFn)>,
}

impl Module {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn provide<T: Send + Sync + 'static>(mut self, v: T) -> Self {
        self.providers.push(Box::new(move |c| c.provide(v)));
        self
    }
    /// このモジュールのメソッド全てに掛かるガード(Nestの`@UseGuards`相当)。
    pub fn guard(mut self, g: impl Fn(&Ctx) -> Result<(), Denied> + Send + Sync + 'static) -> Self {
        self.guards.push(Arc::new(g));
        self
    }
    /// `service`は`pkg.Service`形式、`method`はメソッド名。
    pub fn method(
        mut self,
        service: &str,
        method: &str,
        f: impl Fn(&Ctx, &str) -> Result<String, Denied> + Send + Sync + 'static,
    ) -> Self {
        self.methods.push((service.into(), method.into(), Arc::new(f)));
        self
    }
}

#[derive(Default)]
pub struct Backend {
    container: Container,
    global_guards: Vec<Guard>,
    methods: HashMap<(String, String), Entry>,
}

impl Backend {
    pub fn register(&mut self, m: Module) {
        for p in m.providers {
            p(&mut self.container);
        }
        for (s, n, h) in m.methods {
            self.methods.insert((s, n), Entry { handler: h, guards: m.guards.clone() });
        }
    }
    pub fn global_guard(&mut self, g: impl Fn(&Ctx) -> Result<(), Denied> + Send + Sync + 'static) {
        self.global_guards.push(Arc::new(g));
    }
    pub fn container(&self) -> &Container {
        &self.container
    }
    /// 登録済みの `/<Service>/<Method>` 一覧(ホスト側のルート登録用)。
    pub fn paths(&self) -> Vec<String> {
        let mut v: Vec<String> = self.methods.keys().map(|(s, m)| format!("/{s}/{m}")).collect();
        v.sort();
        v
    }

    /// Connect単純呼び出し。該当メソッドが無ければ`None`(ページ側へ回す)。
    /// 戻り値は(ステータス, JSON本文)。
    pub fn call(&self, http_method: &str, path: &str, headers: &[(String, String)], body: &str) -> Option<(u16, String)> {
        let path = path.split(['?', '#']).next().unwrap_or("");
        let (svc, m) = path.strip_prefix('/')?.rsplit_once('/')?;
        let entry = self.methods.get(&(svc.to_string(), m.to_string()))?;
        if http_method != "POST" {
            return Some(err_json(&Denied::new(405, "POST only")));
        }
        if body.len() > MAX_BODY {
            return Some(err_json(&Denied::new(413, "body too large")));
        }
        let ctx = Ctx { container: &self.container, headers };
        for g in self.global_guards.iter().chain(entry.guards.iter()) {
            if let Err(d) = g(&ctx) {
                return Some(err_json(&d));
            }
        }
        Some(match (entry.handler)(&ctx, body) {
            Ok(json) => (200, json),
            Err(d) => err_json(&d),
        })
    }
}

fn code(status: u16) -> &'static str {
    match status {
        400 => "invalid_argument",
        401 => "unauthenticated",
        403 => "permission_denied",
        404 => "not_found",
        405 | 413 => "failed_precondition",
        429 => "resource_exhausted",
        _ => "internal",
    }
}

fn err_json(d: &Denied) -> (u16, String) {
    let mut msg = String::new();
    for c in d.message.chars() {
        match c {
            '"' => msg.push_str("\\\""),
            '\\' => msg.push_str("\\\\"),
            c if (c as u32) < 0x20 => msg.push(' '),
            c => msg.push(c),
        }
    }
    (d.status, format!("{{\"code\":\"{}\",\"message\":\"{}\"}}", code(d.status), msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Counter(std::sync::atomic::AtomicU32);

    fn backend() -> Backend {
        let mut b = Backend::default();
        b.register(
            Module::new()
                .provide(Counter(Default::default()))
                .guard(|c| match c.header("authorization") {
                    Some("Bearer ok") => Ok(()),
                    Some(_) => Err(Denied::new(403, "bad token")),
                    None => Err(Denied::new(401, "login required")),
                })
                .method("demo.v1.Counter", "Inc", |c, _| {
                    let n = c.get::<Counter>().ok_or(Denied::new(500, "no provider"))?;
                    Ok(format!("{{\"n\":{}}}", n.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1))
                }),
        );
        b
    }

    #[test]
    fn di_guard_and_errors() {
        let b = backend();
        let ok = [("Authorization".to_string(), "Bearer ok".to_string())];
        assert_eq!(b.call("POST", "/demo.v1.Counter/Inc", &ok, "{}"), Some((200, "{\"n\":1}".into())));
        assert_eq!(b.call("POST", "/demo.v1.Counter/Inc", &ok, "{}").unwrap().1, "{\"n\":2}");
        let r = b.call("POST", "/demo.v1.Counter/Inc", &[], "{}").unwrap();
        assert_eq!(r.0, 401);
        assert!(r.1.contains("unauthenticated"));
        let bad = [("authorization".to_string(), "Bearer x".to_string())];
        assert_eq!(b.call("POST", "/demo.v1.Counter/Inc", &bad, "").unwrap().0, 403);
        assert_eq!(b.call("GET", "/demo.v1.Counter/Inc", &ok, "").unwrap().0, 405);
        assert_eq!(b.call("POST", "/demo.v1.Counter/Inc", &ok, &"x".repeat(MAX_BODY + 1)).unwrap().0, 413);
        assert!(b.call("POST", "/demo.v1.Counter/Nope", &ok, "").is_none());
        assert!(b.call("POST", "/about", &ok, "").is_none());
        assert_eq!(b.paths(), vec!["/demo.v1.Counter/Inc"]);
    }
}
