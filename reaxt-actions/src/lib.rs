//! Server Actions: 1つの定義(`Action<I, O>`)をサーバーとブラウザで共有する。
//!
//! - サーバー: `Module::action(&ACTION, |ctx, input| ...)`(型付き。入力JSONは上限つきで読む)
//! - クライアント: `call(&client, &ACTION, &input, |result| ...)`(コールバック式。非同期の応答で状態を更新する)
//!
//! 通信はConnect単純呼び出し(`POST /pkg.Service/Method`、本文JSON)のみで、RESTは作らない。
pub mod subgraph;
use reaxt::backend::{Ctx, Denied, Module};
use rust_json::{parse_light_secure, LightLimits};
use std::marker::PhantomData;

#[cfg(target_arch = "wasm32")]
pub mod web;

/// 入力`I`・出力`O`の型つき呼び出し口の定義。`const`で作れて、両側で同じ値を使う。
pub struct Action<I, O> {
    pub service: &'static str,
    pub method: &'static str,
    _p: PhantomData<fn(I) -> O>,
}

impl<I, O> Action<I, O> {
    pub const fn new(service: &'static str, method: &'static str) -> Self {
        Action { service, method, _p: PhantomData }
    }
    pub fn path(&self) -> String {
        format!("/{}/{}", self.service, self.method)
    }
}

/// サーバー側の登録(`Module`の拡張)。
pub trait ModuleActions: Sized {
    fn action<I, O, F>(self, a: &Action<I, O>, f: F) -> Self
    where
        I: Wire + 'static,
        O: Wire + 'static,
        F: Fn(&Ctx, I) -> Result<O, Denied> + Send + Sync + 'static;

    /// `action` に加えて、出力 `O`(`wire_struct!` の構造体)を `@key` のエンティティとしてCosmoのサブグラフに公開する。
    /// 入力 `I`(`wire_struct!`)のフィールドが鍵。ゲートウェイの `_entities` 呼び出しで、この関数が1件ずつ解決する。
    fn entity<I, O, F>(self, a: &Action<I, O>, f: F) -> Self
    where
        I: Wire + 'static,
        O: Wire + 'static,
        F: Fn(&Ctx, I) -> Result<O, Denied> + Send + Sync + 'static;
}

impl ModuleActions for Module {
    fn action<I, O, F>(self, a: &Action<I, O>, f: F) -> Self
    where
        I: Wire + 'static,
        O: Wire + 'static,
        F: Fn(&Ctx, I) -> Result<O, Denied> + Send + Sync + 'static,
    {
        subgraph::register_output(&a.path(), O::gql_type());
        self.method(a.service, a.method, move |ctx, body| {
            let input = decode::<I>(body).map_err(|e| Denied::new(400, e))?;
            let out = f(ctx, input)?;
            Ok(out.to_wire())
        })
    }

    fn entity<I, O, F>(self, a: &Action<I, O>, f: F) -> Self
    where
        I: Wire + 'static,
        O: Wire + 'static,
        F: Fn(&Ctx, I) -> Result<O, Denied> + Send + Sync + 'static,
    {
        subgraph::register_entity(&a.path(), I::gql_type(), O::gql_type());
        self.action(a, f)
    }
}

/// CSRF対策のガード: ブラウザの`WebClient`が付ける`x-reaxt: 1`を必須にする。
/// カスタムヘッダは他オリジンのページからはCORSの事前確認なしに送れないため、
/// 勝手なサイトからのフォーム送信やfetchを弾ける。`App::global_guard(csrf_guard)`で全体に掛ける。
pub fn csrf_guard(ctx: &Ctx) -> Result<(), Denied> {
    if ctx.header("x-reaxt") == Some("1") {
        Ok(())
    } else {
        Err(Denied::new(403, "missing x-reaxt header"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    /// 通信できなかった(fetch失敗など)
    Transport(String),
    /// サーバーが返したエラー(Connectのcode/message)
    Server { status: u16, code: String, message: String },
    /// 応答のJSONが期待した型でなかった
    Decode(String),
}

pub type RawResult = Result<(u16, String), String>;

/// HTTPで`POST`する部品(ブラウザ=fetch、テスト・SSR=同一プロセス)。応答はコールバックで返す。
pub trait Client {
    fn post(&self, path: &str, body: String, cb: Box<dyn FnOnce(RawResult)>);
}

/// Actionの入出力型が実装する、serde不要のJSON変換(読みはRS-JSONの`light`、上限・重複キー拒否つき)。
/// `to_wire`はコンパクトなJSON文字列、`from_wire`は読み込み済みの値から作る。
pub trait Wire: Sized {
    fn to_wire(&self) -> String;
    fn from_wire(v: &LightValue) -> Result<Self, String>;
    /// この型のGraphQL上の形(Cosmoのサブグラフが、結果を `JSON` ではなく型付きで見せるのに使う)。
    /// 既定は `Json`(構造を持たない)。`wire_struct!` は `Object` を返す。
    fn gql_type() -> GqlType {
        GqlType::Json
    }
}

/// `Wire` 型のGraphQL上の形。
#[derive(Clone, Debug, PartialEq)]
pub enum GqlType {
    /// 構造を持たない(GraphQLの `JSON` スカラー)。
    Json,
    Int,
    Boolean,
    Str,
    List(Box<GqlType>),
    /// 名前付きフィールドの構造体(`wire_struct!`)。
    Object { name: &'static str, fields: Vec<(&'static str, GqlType)> },
}

impl Wire for () {
    fn to_wire(&self) -> String {
        "null".into()
    }
    fn from_wire(v: &LightValue) -> Result<Self, String> {
        if *v == LightValue::Null { Ok(()) } else { Err("null が必要です".into()) }
    }
}
macro_rules! wire_int {
    ($($t:ty),*) => {$(
        impl Wire for $t {
            fn to_wire(&self) -> String { self.to_string() }
            fn from_wire(v: &LightValue) -> Result<Self, String> {
                v.as_i64().and_then(|n| <$t>::try_from(n).ok()).ok_or_else(|| "整数が必要です(範囲外かも)".to_string())
            }
            fn gql_type() -> $crate::GqlType {
                $crate::GqlType::Int
            }
        }
    )*};
}
wire_int!(u8, u16, u32, i32, i64, usize);
impl Wire for bool {
    fn to_wire(&self) -> String {
        self.to_string()
    }
    fn from_wire(v: &LightValue) -> Result<Self, String> {
        v.as_bool().ok_or_else(|| "真偽値が必要です".into())
    }
    fn gql_type() -> GqlType {
        GqlType::Boolean
    }
}
impl Wire for String {
    fn to_wire(&self) -> String {
        rust_json::quote_json(self)
    }
    fn from_wire(v: &LightValue) -> Result<Self, String> {
        v.as_str().map(str::to_string).ok_or_else(|| "文字列が必要です".into())
    }
    fn gql_type() -> GqlType {
        GqlType::Str
    }
}
impl<T: Wire> Wire for Vec<T> {
    fn to_wire(&self) -> String {
        format!("[{}]", self.iter().map(Wire::to_wire).collect::<Vec<_>>().join(","))
    }
    fn from_wire(v: &LightValue) -> Result<Self, String> {
        v.as_array().ok_or("配列が必要です")?.iter().map(T::from_wire).collect()
    }
    fn gql_type() -> GqlType {
        GqlType::List(Box::new(T::gql_type()))
    }
}

/// 上限つきでJSON本文を読んで`T`にする。
pub fn decode<T: Wire>(json: &str) -> Result<T, String> {
    let v = parse_light_secure(json, &LightLimits::small()).map_err(|e| e.to_string())?;
    T::from_wire(&v)
}

/// 必須の文字列フィールドを取り出す(`from_wire`の実装用)。
pub fn str_field(v: &LightValue, k: &str) -> Result<String, String> {
    v.get(k).and_then(|x| x.as_str()).map(str::to_string).ok_or_else(|| format!("文字列フィールド {k} がありません"))
}
/// 必須の真偽値フィールド。
pub fn bool_field(v: &LightValue, k: &str) -> Result<bool, String> {
    v.get(k).and_then(|x| x.as_bool()).ok_or_else(|| format!("真偽値フィールド {k} がありません"))
}
/// 必須の整数フィールド。
pub fn int_field(v: &LightValue, k: &str) -> Result<i64, String> {
    v.get(k).and_then(|x| x.as_i64()).ok_or_else(|| format!("整数フィールド {k} がありません"))
}

/// 1つの文字列フィールドだけのオブジェクト(`{"k":"v"}`)を書き出す補助。
pub fn obj_json(fields: &[(&str, String)]) -> String {
    format!("{{{}}}", fields.iter().map(|(k, v)| format!("{}:{v}", rust_json::quote_json(k))).collect::<Vec<_>>().join(","))
}
pub use rust_json::quote_json;

/// 必須フィールドを任意の`Wire`型で取り出す(`wire_struct!`が使う)。
pub fn field<T: Wire>(v: &LightValue, k: &str) -> Result<T, String> {
    T::from_wire(v.get(k).ok_or_else(|| format!("フィールド {k} がありません"))?).map_err(|e| format!("{k}: {e}"))
}

/// 名前付きフィールドの構造体に`Wire`を実装する(各フィールドも`Wire`であること)。
/// 例: `wire_struct!(Add { by: u32 });` → `{"by":3}` との相互変換。
#[macro_export]
macro_rules! wire_struct {
    ($t:ident { $($f:ident : $ty:ty),* $(,)? }) => {
        impl $crate::Wire for $t {
            fn to_wire(&self) -> String {
                $crate::obj_json(&[$((stringify!($f), $crate::Wire::to_wire(&self.$f))),*])
            }
            fn from_wire(v: &$crate::LightValue) -> Result<Self, String> {
                Ok($t { $($f: $crate::field::<$ty>(v, stringify!($f))?),* })
            }
            fn gql_type() -> $crate::GqlType {
                $crate::GqlType::Object {
                    name: stringify!($t),
                    fields: vec![$((stringify!($f), <$ty as $crate::Wire>::gql_type())),*],
                }
            }
        }
    };
}
pub use rust_json::LightValue;

/// 型付きで呼び出す。`cb`は応答(または失敗)で1回だけ呼ばれる。
pub fn call<I, O>(client: &dyn Client, a: &Action<I, O>, input: &I, cb: impl FnOnce(Result<O, ActionError>) + 'static)
where
    I: Wire,
    O: Wire + 'static,
{
    client.post(
        &a.path(),
        input.to_wire(),
        Box::new(move |raw| {
            cb(match raw {
                Err(e) => Err(ActionError::Transport(e)),
                Ok((200, json)) => decode::<O>(&json).map_err(ActionError::Decode),
                Ok((status, json)) => match parse_light_secure(&json, &LightLimits::small()) {
                    Ok(e) => match (str_field(&e, "code"), str_field(&e, "message")) {
                        (Ok(code), Ok(message)) => Err(ActionError::Server { status, code, message }),
                        _ => Err(ActionError::Server { status, code: "unknown".into(), message: json }),
                    },
                    Err(_) => Err(ActionError::Server { status, code: "unknown".into(), message: json }),
                },
            })
        }),
    );
}

/// 同一プロセスの`App`を直接呼ぶクライアント(テスト・SSR時のデータ取得用)。
pub struct LocalClient<'a> {
    pub app: &'a reaxt::app::App,
    pub headers: Vec<(String, String)>,
}

impl Client for LocalClient<'_> {
    fn post(&self, path: &str, body: String, cb: Box<dyn FnOnce(RawResult)>) {
        let r = self.app.dispatch(&reaxt::app::Request { method: "POST", path, headers: &self.headers, body: &body });
        cb(Ok((r.status, r.body)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reaxt::app::App;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[derive(Debug, PartialEq)]
    struct Add {
        by: u32,
    }
    #[derive(Debug, PartialEq)]
    struct Total {
        total: u32,
    }
    const ADD: Action<Add, Total> = Action::new("demo.v1.Counter", "Add");
    crate::wire_struct!(Add { by: u32 });
    crate::wire_struct!(Total { total: u32 });

    #[derive(Debug, PartialEq)]
    struct Mixed {
        name: String,
        on: bool,
        ids: Vec<u8>,
    }
    crate::wire_struct!(Mixed { name: String, on: bool, ids: Vec<u8> });

    #[test]
    fn wire_struct_round_trips_and_rejects() {
        let m = Mixed { name: "a\"b\n".into(), on: true, ids: vec![1, 2, 3] };
        let json = m.to_wire();
        assert_eq!(decode::<Mixed>(&json).unwrap(), m);
        assert!(decode::<Mixed>("{\"name\":\"x\",\"on\":true}").unwrap_err().contains("ids"));
        assert!(decode::<Add>("{\"by\":-1}").is_err());
        assert!(decode::<Add>("{\"by\":1,\"by\":2}").is_err());
    }

    fn app() -> App {
        let counter = Arc::new(AtomicU32::new(0));
        App::new("t")
            .module(
                Module::new()
                    .guard(|c| if c.header("x-key") == Some("k") { Ok(()) } else { Err(Denied::new(401, "no key")) })
                    .action(&ADD, move |_, i: Add| {
                        if i.by > 100 {
                            return Err(Denied::new(400, "too big"));
                        }
                        Ok(Total { total: counter.fetch_add(i.by, Ordering::SeqCst) + i.by })
                    }),
            )
            .finish()
    }

    fn run<O: Wire + 'static>(c: &LocalClient, a: &Action<Add, O>, by: u32) -> Result<O, ActionError> {
        let out = Rc::new(RefCell::new(None));
        let o2 = out.clone();
        call(c, a, &Add { by }, move |r| *o2.borrow_mut() = Some(r));
        let r = out.borrow_mut().take().expect("callback called once");
        r
    }

    #[test]
    fn typed_round_trip_errors_and_guard() {
        let app = app();
        let ok = LocalClient { app: &app, headers: vec![("x-key".into(), "k".into())] };
        assert_eq!(run(&ok, &ADD, 2), Ok(Total { total: 2 }));
        assert_eq!(run(&ok, &ADD, 3), Ok(Total { total: 5 }), "サーバー側の状態が保たれる");
        match run(&ok, &ADD, 101) {
            Err(ActionError::Server { status: 400, code, message }) => {
                assert_eq!(code, "invalid_argument");
                assert_eq!(message, "too big");
            }
            other => panic!("{other:?}"),
        }
        let anon = LocalClient { app: &app, headers: vec![] };
        assert!(matches!(run(&anon, &ADD, 1), Err(ActionError::Server { status: 401, .. })));
        // 型が合わない応答はDecodeエラー
        const WRONG: Action<Add, Vec<String>> = Action::new("demo.v1.Counter", "Add");
        assert!(matches!(run(&ok, &WRONG, 1), Err(ActionError::Decode(_))));
    }

    #[test]
    fn csrf_guard_requires_custom_header() {
        let app = App::new("t")
            .global_guard(csrf_guard)
            .module(Module::new().action(&ADD, |_, i: Add| Ok(Total { total: i.by })))
            .finish();
        let no = LocalClient { app: &app, headers: vec![] };
        assert!(matches!(run(&no, &ADD, 1), Err(ActionError::Server { status: 403, .. })));
        let yes = LocalClient { app: &app, headers: vec![("X-Reaxt".into(), "1".into())] };
        assert_eq!(run(&yes, &ADD, 7), Ok(Total { total: 7 }));
    }

    #[test]
    fn malformed_and_oversized_inputs_are_rejected_by_rs_json() {
        let app = app();
        let post = |b: &str| app.dispatch(&reaxt::app::Request { method: "POST", path: "/demo.v1.Counter/Add", headers: &[("x-key".into(), "k".into())], body: b });
        assert_eq!(post("{\"by\":\"x\"}").status, 400);
        assert_eq!(post("not json").status, 400);
        let deep = format!("{}1{}", "[".repeat(100), "]".repeat(100));
        assert_eq!(post(&deep).status, 400, "深さ上限");
    }
}
