//! reaxt コマンド。
mod build;
mod scaffold;

use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;

const USAGE: &str = "reaxt — Rustだけのフルスタック枠組み

使い方:
  reaxt new <名前> [--path <Reaxtのディレクトリ> | --git <URL>]   プロジェクトを作る
  reaxt build [--debug]        WASMをビルドして pkg/ を作る(既定はrelease)
  reaxt dev [--port N]         ビルド+サーバー起動+変更の監視(既定ポート3000)
  reaxt doctor                 必要な道具の確認
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("エラー: {e}");
            1
        }
    };
    std::process::exit(code);
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("new") => cmd_new(&args[1..]),
        Some("build") => build::build(&PathBuf::from("."), !args.iter().any(|a| a == "--debug")),
        Some("dev") => cmd_dev(flag(args, "--port").unwrap_or_else(|| "3000".into())),
        Some("doctor") => cmd_doctor(),
        _ => {
            print!("{USAGE}");
            Ok(())
        }
    }
}

fn cmd_doctor() -> Result<(), String> {
    let mut bad = 0;
    for (name, r) in build::doctor() {
        match r {
            Ok(v) => println!("  OK   {name}: {v}"),
            Err(fix) => {
                bad += 1;
                println!("  NG   {name}: {fix}");
            }
        }
    }
    if bad == 0 {
        Ok(())
    } else {
        Err(format!("{bad}件の不足があります"))
    }
}

fn cmd_new(args: &[String]) -> Result<(), String> {
    let name = args.first().filter(|a| !a.starts_with("--")).ok_or("プロジェクト名を指定してください: reaxt new <名前>")?;
    scaffold::check_name(name)?;
    let dep = match (flag(args, "--path"), flag(args, "--git")) {
        (Some(p), _) => {
            let abs = std::fs::canonicalize(&p).map_err(|e| format!("--path {p}: {e}"))?;
            // Windowsの拡張パス接頭辞(\\?\)とバックスラッシュを除く
            let s = abs.to_string_lossy().trim_start_matches(r"\\?\").replace('\\', "/");
            format!("{{ path = \"{s}\" }}")
        }
        (None, Some(g)) => format!("{{ git = \"{g}\" }}"),
        (None, None) => "{ git = \"https://github.com/aon-co-jp/Reaxt\" }".to_string(),
    };
    let wbg = std::process::Command::new("wasm-bindgen")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| build::parse_cli_version(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_else(|| {
            eprintln!("注意: wasm-bindgen が見つからないので 0.2.126 を仮置きしました(`reaxt doctor`)");
            "0.2.126".into()
        });
    let dir = PathBuf::from(name);
    scaffold::write(&dir, &scaffold::files(&scaffold::Opts { name: name.clone(), dep, wbg }))?;
    println!("{name}/ を作りました。\n  cd {name}\n  reaxt build\n  reaxt dev");
    Ok(())
}

fn spawn_server(port: &str) -> Result<Child, String> {
    Command::new("cargo")
        .args(["run", "--quiet", "--bin", "serve", "--", "pkg", port])
        .spawn()
        .map_err(|e| format!("サーバーを起動できません: {e}"))
}

fn cmd_dev(port: String) -> Result<(), String> {
    let root = PathBuf::from(".");
    build::build(&root, false)?;
    let mut child = spawn_server(&port)?;
    println!("監視中(Ctrl+Cで終了)。変更を検知すると再ビルドして再起動します。");
    let mut seen = build::fingerprint(&root);
    loop {
        std::thread::sleep(Duration::from_millis(700));
        let now = build::fingerprint(&root);
        if now == seen {
            continue;
        }
        // 保存途中の揺れを避けて少し待ち、静まってから1回だけビルドする
        std::thread::sleep(Duration::from_millis(400));
        seen = build::fingerprint(&root);
        println!("変更を検知。再ビルドします…");
        let _ = child.kill();
        let _ = child.wait();
        match build::build(&root, false) {
            Ok(()) => child = spawn_server(&port)?,
            Err(e) => {
                eprintln!("ビルド失敗: {e}\n直すと自動で再試行します。");
                // 失敗中は子プロセスが無いので、ダミーの短命プロセスで置き換える
                child = Command::new("cargo").arg("--version").stdout(std::process::Stdio::null()).spawn().map_err(|e| e.to_string())?;
                // 直ったら次の変更で再ビルドされる
            }
        }
    }
}
