//! `reaxt new`: プロジェクトの雛形を作る(純関数+書き出し)。
use std::path::Path;

pub struct Opts {
    /// プロジェクト名(= パッケージ名)。
    pub name: String,
    /// `reaxt = ...` の右辺(例 `{ git = "..." }` / `{ path = "..." }`)。
    pub dep: String,
    /// `wasm-bindgen` のバージョン(インストール済みのCLIと一致させる)。
    pub wbg: String,
}

/// パッケージ名の検査(小文字・数字・`-`・`_`、先頭は英字)。
pub fn check_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(format!("プロジェクト名 `{name}` は使えません(小文字・数字・-・_ のみ、先頭は英字)"))
    }
}

/// ライブラリ名(`-` は `_` になる)。wasmのファイル名もこれ。
pub fn lib_name(name: &str) -> String {
    name.replace('-', "_")
}

fn fill(t: &str, o: &Opts) -> String {
    t.replace("{{name}}", &o.name)
        .replace("{{lib}}", &lib_name(&o.name))
        .replace("{{dep}}", &o.dep)
        .replace("{{wbg}}", &o.wbg)
}

const CARGO: &str = r#"[package]
name = "{{name}}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
reaxt = {{dep}}

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "={{wbg}}"
"#;

const LIB: &str = r##"//! サーバー(SSR)とブラウザ(WASM)で同じ関数を使う。
use reaxt::hydrate::on_click;
use reaxt::react::{use_state, VNode};

pub fn view() -> VNode {
    let (n, set) = use_state(|| 0i32);
    let (k, v) = on_click("inc", move || set.update(|c| c + 1));
    VNode::element("main")
        .child(VNode::element("h1").child(VNode::text("{{name}}")).build())
        .child(VNode::element("p").attr("id", "count").child(VNode::text(format!("count={n}"))).build())
        .child(VNode::element("button").attr("id", "inc").attr(k, v).child(VNode::text("+1")).build())
        .build()
}

#[cfg(target_arch = "wasm32")]
mod boot {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        reaxt::wasm::mount("#reaxt-root", 1, super::view).map(|_| ())
    }
}
"##;

const SERVE: &str = r#"//! 開発・本番共通のサーバー。使い方: serve [pkgディレクトリ] [ポート]
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use reaxt::app::{App, RouteRule};
    use reaxt::react::hooks::{drop_instance, run_component};
    use reaxt::RenderMode;
    use std::sync::Arc;

    let mut a = std::env::args().skip(1);
    let pkg = a.next().unwrap_or_else(|| "pkg".into());
    let port = a.next().unwrap_or_else(|| "3000".into());
    let read = |f: &str| std::fs::read(format!("{pkg}/{f}")).unwrap_or_else(|e| panic!("{pkg}/{f}: {e}(先に reaxt build を実行してください)"));

    let app = App::new("{{name}}")
        .page("index.rs", RenderMode::SsrHydrate, |_| {
            let t = run_component(1, {{lib}}::view);
            drop_instance(1);
            t
        })
        .asset(
            "/_reaxt/boot.js",
            "text/javascript; charset=utf-8",
            b"import init from \"/_reaxt/pkg/{{lib}}.js\";\ninit();\n".to_vec(),
        )
        .asset("/_reaxt/pkg/{{lib}}.js", "text/javascript; charset=utf-8", read("{{lib}}.js"))
        .asset("/_reaxt/pkg/{{lib}}_bg.wasm", "application/wasm", read("{{lib}}_bg.wasm"))
        .route_rule("/_reaxt/", RouteRule::default().cache(0))
        .finish();
    let l = std::net::TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind");
    println!("http://127.0.0.1:{port}/");
    reaxt::server::serve(Arc::new(app), l);
}
"#;

const GITIGNORE: &str = "target\npkg\n";

const README: &str = r#"# {{name}}

Reaxt プロジェクト(Rustだけのフルスタック)。

```bash
reaxt doctor   # 必要な道具の確認
reaxt build    # WASMをビルドして pkg/ へ
reaxt dev      # ビルド+サーバー起動+変更の監視(http://127.0.0.1:3000/)
```
"#;

/// 生成するファイル一覧(相対パス, 中身)。
pub fn files(o: &Opts) -> Vec<(String, String)> {
    vec![
        ("Cargo.toml".into(), fill(CARGO, o)),
        ("src/lib.rs".into(), fill(LIB, o)),
        ("src/bin/serve.rs".into(), fill(SERVE, o)),
        (".gitignore".into(), GITIGNORE.into()),
        ("README.md".into(), fill(README, o)),
    ]
}

/// 空でない既存ディレクトリには書かない(上書き事故の防止)。
pub fn write(dir: &Path, files: &[(String, String)]) -> Result<(), String> {
    if dir.exists() && std::fs::read_dir(dir).map_err(|e| e.to_string())?.next().is_some() {
        return Err(format!("{} は空ではありません", dir.display()));
    }
    for (rel, body) in files {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, body).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> Opts {
        Opts { name: "my-app".into(), dep: "{ path = \"/x/Reaxt\" }".into(), wbg: "0.2.126".into() }
    }

    #[test]
    fn names() {
        assert!(check_name("my-app_2").is_ok());
        for bad in ["", "My", "2app", "a b", "a/b", "../x"] {
            assert!(check_name(bad).is_err(), "{bad}");
        }
        assert_eq!(lib_name("my-app"), "my_app");
    }

    #[test]
    fn placeholders_are_all_filled() {
        for (path, body) in files(&opts()) {
            assert!(!body.contains("{{"), "{path} に未置換の印が残っている");
        }
        let f: std::collections::HashMap<_, _> = files(&opts()).into_iter().collect();
        assert!(f["Cargo.toml"].contains("reaxt = { path = \"/x/Reaxt\" }"));
        assert!(f["Cargo.toml"].contains("wasm-bindgen = \"=0.2.126\""));
        assert!(f["src/bin/serve.rs"].contains("my_app::view"));
        assert!(f["src/bin/serve.rs"].contains("/_reaxt/pkg/my_app_bg.wasm"));
    }

    #[test]
    fn refuses_non_empty_directory_and_writes_into_new_one() {
        let base = std::env::temp_dir().join(format!("reaxt-cli-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        write(&base, &files(&opts())).unwrap();
        assert!(base.join("src/bin/serve.rs").exists());
        assert!(write(&base, &files(&opts())).is_err(), "2回目は拒否");
        std::fs::remove_dir_all(&base).unwrap();
    }
}
