//! `reaxt build` / `dev` / `doctor` の部品。判定は純関数、実行は`Command`。
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::UNIX_EPOCH;

/// `Cargo.toml` の `[package]` の `name`。
pub fn package_name(toml: &str) -> Option<String> {
    let mut in_pkg = false;
    for line in toml.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_pkg = l == "[package]";
        } else if in_pkg {
            if let Some(rest) = l.strip_prefix("name") {
                let v = rest.trim_start().strip_prefix('=')?.trim();
                return Some(v.trim_matches('"').to_string());
            }
        }
    }
    None
}

/// `Cargo.lock` から `wasm-bindgen` クレートのバージョン。
pub fn lock_wbg_version(lock: &str) -> Option<String> {
    let mut lines = lock.lines();
    while let Some(l) = lines.next() {
        if l.trim() == "name = \"wasm-bindgen\"" {
            let v = lines.next()?.trim().strip_prefix("version = ")?;
            return Some(v.trim_matches('"').to_string());
        }
    }
    None
}

/// `wasm-bindgen --version` の出力(`wasm-bindgen 0.2.126`)からバージョン。
pub fn parse_cli_version(out: &str) -> Option<String> {
    out.split_whitespace().nth(1).map(|s| s.to_string())
}

/// ビルドされたwasmの場所(`<root>/target/wasm32-unknown-unknown/<profile>/<lib>.wasm`)。
pub fn wasm_path(root: &Path, release: bool, lib: &str) -> PathBuf {
    root.join("target").join("wasm32-unknown-unknown").join(if release { "release" } else { "debug" }).join(format!("{lib}.wasm"))
}

/// 変更検知用の指紋(`src/`と`Cargo.toml`のファイル数と更新時刻の合算)。
pub fn fingerprint(root: &Path) -> u128 {
    fn walk(p: &Path, acc: &mut u128) {
        let Ok(rd) = std::fs::read_dir(p) else { return };
        for e in rd.flatten() {
            let path = e.path();
            if path.is_dir() {
                walk(&path, acc);
            } else if let Ok(m) = e.metadata() {
                let t = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos());
                *acc = acc.wrapping_add(t).wrapping_add(m.len() as u128).wrapping_add(1);
            }
        }
    }
    let mut acc = 0u128;
    walk(&root.join("src"), &mut acc);
    if let Ok(m) = std::fs::metadata(root.join("Cargo.toml")) {
        let t = m.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos());
        acc = acc.wrapping_add(t);
    }
    acc
}

fn out_of(cmd: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(cmd).args(args).stdin(Stdio::null()).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// 道具の確認。(名前, 結果, 直し方) の一覧。
pub fn doctor() -> Vec<(String, Result<String, String>)> {
    let mut v = Vec::new();
    v.push(("cargo".into(), out_of("cargo", &["--version"]).ok_or_else(|| "見つかりません。https://rustup.rs".to_string())));
    let targets = out_of("rustup", &["target", "list", "--installed"]).unwrap_or_default();
    v.push((
        "wasm32-unknown-unknown".into(),
        if targets.lines().any(|l| l.trim() == "wasm32-unknown-unknown") {
            Ok("導入済み".into())
        } else {
            Err("rustup target add wasm32-unknown-unknown".into())
        },
    ));
    v.push((
        "wasm-bindgen".into(),
        out_of("wasm-bindgen", &["--version"]).ok_or_else(|| "cargo install wasm-bindgen-cli(プロジェクトのwasm-bindgenと同じ版)".to_string()),
    ));
    v
}

/// wasmをビルドして `pkg/` へグルーを生成する。
pub fn build(root: &Path, release: bool) -> Result<(), String> {
    let toml = std::fs::read_to_string(root.join("Cargo.toml")).map_err(|e| format!("Cargo.toml: {e}(プロジェクトのディレクトリで実行してください)"))?;
    let name = package_name(&toml).ok_or("Cargo.toml に [package] name がありません")?;
    let lib = name.replace('-', "_");

    let mut args = vec!["build", "--lib", "--target", "wasm32-unknown-unknown"];
    if release {
        args.push("--release");
    }
    let st = Command::new("cargo").args(&args).current_dir(root).status().map_err(|e| format!("cargo を起動できません: {e}"))?;
    if !st.success() {
        return Err("cargo build に失敗しました(上のエラーを確認してください。wasm32ターゲットが無ければ `reaxt doctor`)".into());
    }

    // wasm-bindgen CLI とライブラリの版が違うと実行時に壊れるので、先に照合する
    let cli = out_of("wasm-bindgen", &["--version"]).and_then(|o| parse_cli_version(&o)).ok_or("wasm-bindgen が見つかりません(`reaxt doctor`)")?;
    if let Some(lock) = std::fs::read_to_string(root.join("Cargo.lock")).ok().and_then(|l| lock_wbg_version(&l)) {
        if lock != cli {
            return Err(format!("wasm-bindgen の版が合いません: プロジェクト {lock} / CLI {cli}。`cargo install wasm-bindgen-cli --version {lock} --force`"));
        }
    }
    let wasm = wasm_path(root, release, &lib);
    if !wasm.exists() {
        return Err(format!("{} がありません(Cargo.toml の [lib] crate-type に cdylib が要ります)", wasm.display()));
    }
    let out = root.join("pkg");
    let _ = std::fs::remove_dir_all(&out);
    let st = Command::new("wasm-bindgen")
        .args(["--target", "web", "--out-dir"])
        .arg(&out)
        .arg(&wasm)
        .status()
        .map_err(|e| e.to_string())?;
    if !st.success() {
        return Err("wasm-bindgen に失敗しました".into());
    }
    println!("→ {} を作りました", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_toml_lock_and_cli() {
        assert_eq!(package_name("[package]\nname = \"my-app\"\nversion=\"1\"\n[lib]\nname = \"x\"").as_deref(), Some("my-app"));
        assert_eq!(package_name("[lib]\nname = \"x\""), None);
        let lock = "[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"wasm-bindgen\"\nversion = \"0.2.126\"\n";
        assert_eq!(lock_wbg_version(lock).as_deref(), Some("0.2.126"));
        assert_eq!(lock_wbg_version("[[package]]\nname = \"a\"\nversion = \"1\""), None);
        assert_eq!(parse_cli_version("wasm-bindgen 0.2.126").as_deref(), Some("0.2.126"));
        assert!(wasm_path(Path::new("/p"), true, "my_app").ends_with("wasm32-unknown-unknown/release/my_app.wasm")
            || wasm_path(Path::new("/p"), true, "my_app").to_string_lossy().replace('\\', "/").ends_with("wasm32-unknown-unknown/release/my_app.wasm"));
    }

    #[test]
    fn fingerprint_changes_when_a_source_changes() {
        let d = std::env::temp_dir().join(format!("reaxt-fp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("Cargo.toml"), "x").unwrap();
        std::fs::write(d.join("src/lib.rs"), "a").unwrap();
        let a = fingerprint(&d);
        assert_eq!(a, fingerprint(&d), "変更が無ければ同じ");
        std::fs::write(d.join("src/lib.rs"), "bb").unwrap();
        assert_ne!(a, fingerprint(&d), "内容(サイズ)が変われば変わる");
        std::fs::write(d.join("src/new.rs"), "c").unwrap();
        assert_ne!(fingerprint(&d), a);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
