#![allow(clippy::disallowed_methods, reason = "tooling is exempt")]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use serde_json::Value;

#[derive(Parser)]
pub struct WebExamplesArgs {
    #[arg(long)]
    pub release: bool,
    #[arg(long, default_value = "8080")]
    pub port: u16,
    #[arg(long)]
    pub no_serve: bool,
}

struct ExampleTarget {
    name: String,
    required_features: Vec<String>,
}

fn check_program(binary: &str, install_hint: &str) -> Result<()> {
    match Command::new(binary).arg("--version").output() {
        Ok(output) if output.status.success() => Ok(()),
        _ => bail!("`{binary}` not found. Install with: {install_hint}"),
    }
}

fn gpui_workspace_root() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("GPUI_REPO_PATH") {
        let root = PathBuf::from(path);
        if root.join("Cargo.toml").is_file() && root.join("crates/gpui/examples").is_dir() {
            return Ok(root);
        }
        bail!(
            "GPUI_REPO_PATH={} does not look like the extracted gpui workspace",
            root.display()
        );
    }

    let default_root = std::env::current_dir()
        .context("failed to read current directory")?
        .join("../gpui");
    if default_root.join("Cargo.toml").is_file()
        && default_root.join("crates/gpui/examples").is_dir()
    {
        return Ok(default_root);
    }

    bail!("could not locate the extracted gpui workspace; set GPUI_REPO_PATH to the gpui repo root")
}

fn discover_examples(cargo: &str, gpui_manifest: &Path) -> Result<Vec<ExampleTarget>> {
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--manifest-path",
            gpui_manifest
                .to_str()
                .context("gpui manifest path is not valid UTF-8")?,
            "--no-deps",
            "--format-version",
            "1",
        ])
        .output()
        .context("failed to run cargo metadata for gpui")?;
    if !output.status.success() {
        bail!(
            "cargo metadata for gpui failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let metadata: Value =
        serde_json::from_slice(&output.stdout).context("failed to parse cargo metadata JSON")?;
    let packages = metadata["packages"]
        .as_array()
        .context("cargo metadata did not return packages")?;
    let package = packages
        .iter()
        .find(|package| package["name"].as_str() == Some("gpui"))
        .context("could not find gpui package in cargo metadata")?;
    let targets = package["targets"]
        .as_array()
        .context("gpui package did not contain targets")?;

    let mut examples: Vec<ExampleTarget> = targets
        .iter()
        .filter(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind.as_str() == Some("example")))
        })
        .filter_map(|target| {
            let name = target["name"].as_str()?.to_owned();
            if name.starts_with("native_") {
                return None;
            }
            let required_features = target["required-features"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|feature| feature.as_str().map(ToOwned::to_owned))
                .collect();
            Some(ExampleTarget {
                name,
                required_features,
            })
        })
        .collect();

    if examples.is_empty() {
        bail!("no cargo example targets found for gpui");
    }

    examples.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(examples)
}

pub fn run_web_examples(args: WebExamplesArgs) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let profile = if args.release { "release" } else { "debug" };
    let out_dir = "target/web-examples";
    let cargo_target_dir = "target/web-examples/cargo-target";
    let gpui_root = gpui_workspace_root()?;
    let gpui_manifest = gpui_root.join("Cargo.toml");

    check_program("wasm-bindgen", "cargo install wasm-bindgen-cli")?;

    let examples = discover_examples(&cargo, &gpui_manifest)?;
    eprintln!(
        "Building {} example(s) for wasm32-unknown-unknown ({profile})...\n",
        examples.len()
    );

    std::fs::create_dir_all(out_dir).context("failed to create output directory")?;

    // Run wasm-bindgen on each .wasm that was produced.
    let mut succeeded: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for example in &examples {
        eprintln!("[{}] Building...", example.name);

        let mut cmd = Command::new(&cargo);
        cmd.args([
            "build",
            "--manifest-path",
            gpui_manifest
                .to_str()
                .context("gpui manifest path is not valid UTF-8")?,
            "--target",
            "wasm32-unknown-unknown",
            "--target-dir",
            cargo_target_dir,
            "-p",
            "gpui",
            "--example",
            &example.name,
        ]);
        if !example.required_features.is_empty() {
            cmd.args(["--features", &example.required_features.join(",")]);
        }
        if args.release {
            cmd.arg("--release");
        }
        // 🙈
        cmd.env("RUSTC_BOOTSTRAP", "1");

        let status = cmd.status().context("failed to run cargo build")?;
        if !status.success() {
            eprintln!("[{}] SKIPPED (build failed)", example.name);
            failed.push(example.name.clone());
            continue;
        }

        let wasm_path = format!(
            "{cargo_target_dir}/wasm32-unknown-unknown/{profile}/examples/{}.wasm",
            example.name
        );
        if !Path::new(&wasm_path).exists() {
            eprintln!("[{}] SKIPPED (build output missing)", example.name);
            failed.push(example.name.clone());
            continue;
        }

        eprintln!("[{}] Running wasm-bindgen...", example.name);

        let example_dir = format!("{out_dir}/{}", example.name);
        std::fs::create_dir_all(&example_dir)
            .with_context(|| format!("failed to create {example_dir}"))?;

        let status = Command::new("wasm-bindgen")
            .args([
                &wasm_path,
                "--target",
                "web",
                "--no-typescript",
                "--out-dir",
                &example_dir,
                "--out-name",
                &example.name,
            ])
            // 🙈
            .env("RUSTC_BOOTSTRAP", "1")
            .status()
            .context("failed to run wasm-bindgen")?;
        if !status.success() {
            eprintln!("[{}] SKIPPED (wasm-bindgen failed)", example.name);
            failed.push(example.name.clone());
            continue;
        }

        // Write per-example index.html.
        let html_path = format!("{example_dir}/index.html");
        std::fs::File::create(&html_path)
            .and_then(|mut file| file.write_all(make_example_html(&example.name).as_bytes()))
            .with_context(|| format!("failed to write {html_path}"))?;

        eprintln!("[{}] OK", example.name);
        succeeded.push(example.name.clone());
    }

    if succeeded.is_empty() {
        bail!("all {} examples failed to build", examples.len());
    }

    let example_names: Vec<&str> = succeeded.iter().map(|s| s.as_str()).collect();
    let index_path = format!("{out_dir}/index.html");
    std::fs::File::create(&index_path)
        .and_then(|mut file| file.write_all(make_gallery_html(&example_names).as_bytes()))
        .context("failed to write index.html")?;

    if args.no_serve {
        return Ok(());
    }

    // Serve with COEP/COOP headers required for WebGPU / SharedArrayBuffer.
    eprintln!("Serving on http://127.0.0.1:{}...", args.port);

    let server_script = format!(
        r#"
import http.server
class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory="{out_dir}", **kwargs)
    def end_headers(self):
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        super().end_headers()
http.server.HTTPServer(("127.0.0.1", {port}), Handler).serve_forever()
"#,
        port = args.port,
    );

    let status = Command::new("python3")
        .args(["-c", &server_script])
        .status()
        .context("failed to run python3 http server (is python3 installed?)")?;
    if !status.success() {
        bail!("python3 http server exited with: {status}");
    }

    Ok(())
}

fn make_example_html(name: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>GPUI Web: {name}</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        html, body {{
            width: 100%; height: 100%; overflow: hidden;
            background: #1e1e2e; color: #cdd6f4;
            font-family: system-ui, -apple-system, sans-serif;
        }}
        canvas {{ display: block; width: 100%; height: 100%; }}
        #loading {{
            position: fixed; inset: 0;
            display: flex; align-items: center; justify-content: center;
            font-size: 1.25rem; opacity: 0.6;
        }}
        #loading.hidden {{ display: none; }}
    </style>
</head>
<body>
    <div id="loading">Loading {name}…</div>
    <script type="module">
        import init from './{name}.js';
        await init();
        document.getElementById('loading').classList.add('hidden');
    </script>
</body>
</html>
"#
    )
}

fn make_gallery_html(examples: &[&str]) -> String {
    let mut buttons = String::new();
    for name in examples {
        buttons.push_str(&format!(
            "                <button class=\"example-btn\" data-name=\"{name}\">{name}</button>\n"
        ));
    }

    let first = examples.first().copied().unwrap_or("hello_web");

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>GPUI Web Examples</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        html, body {{
            width: 100%; height: 100%; overflow: hidden;
            background: #1e1e2e; color: #cdd6f4;
            font-family: system-ui, -apple-system, sans-serif;
        }}
        #app {{ display: flex; width: 100%; height: 100%; }}

        #sidebar {{
            width: 240px; min-width: 240px;
            background: #181825;
            border-right: 1px solid #313244;
            display: flex; flex-direction: column;
        }}
        #sidebar-header {{
            padding: 16px 14px 12px;
            font-size: 0.8rem; font-weight: 700;
            text-transform: uppercase; letter-spacing: 0.08em;
            color: #a6adc8; border-bottom: 1px solid #313244;
        }}
        #sidebar-header span {{
            font-size: 1rem; text-transform: none; letter-spacing: normal;
            color: #cdd6f4; display: block; margin-top: 2px;
        }}
        #example-list {{
            flex: 1; overflow-y: auto; padding: 8px 0;
        }}
        .example-btn {{
            display: block; width: 100%;
            padding: 8px 14px; border: none;
            background: transparent; color: #bac2de;
            font-size: 0.85rem; text-align: left;
            cursor: pointer;
            font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace;
        }}
        .example-btn:hover {{ background: #313244; color: #cdd6f4; }}
        .example-btn.active {{ background: #45475a; color: #f5e0dc; font-weight: 600; }}

        #main {{ flex: 1; display: flex; flex-direction: column; min-width: 0; }}
        #toolbar {{
            height: 40px; display: flex; align-items: center;
            padding: 0 16px; gap: 12px;
            background: #1e1e2e; border-bottom: 1px solid #313244;
            font-size: 0.8rem; color: #a6adc8;
        }}
        #current-name {{
            font-weight: 600; color: #cdd6f4;
            font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace;
        }}
        #open-tab {{
            margin-left: auto; padding: 4px 10px;
            border: 1px solid #585b70; border-radius: 4px;
            background: transparent; color: #a6adc8;
            font-size: 0.75rem; cursor: pointer;
            text-decoration: none;
        }}
        #open-tab:hover {{ background: #313244; color: #cdd6f4; }}
        #viewer {{ flex: 1; border: none; width: 100%; background: #11111b; }}
    </style>
</head>
<body>
    <div id="app">
        <div id="sidebar">
            <div id="sidebar-header">
                GPUI Examples
                <span>{count} available</span>
            </div>
            <div id="example-list">
{buttons}            </div>
        </div>
        <div id="main">
            <div id="toolbar">
                <span id="current-name">{first}</span>
                <a id="open-tab" href="./{first}/" target="_blank">Open in new tab ↗</a>
            </div>
            <iframe id="viewer" src="./{first}/"></iframe>
        </div>
    </div>
    <script>
        const buttons = document.querySelectorAll('.example-btn');
        const viewer  = document.getElementById('viewer');
        const nameEl  = document.getElementById('current-name');
        const openEl  = document.getElementById('open-tab');

        function select(name) {{
            buttons.forEach(b => b.classList.toggle('active', b.dataset.name === name));
            viewer.src = './' + name + '/';
            nameEl.textContent = name;
            openEl.href = './' + name + '/';
            history.replaceState(null, '', '#' + name);
        }}

        buttons.forEach(b => b.addEventListener('click', () => select(b.dataset.name)));

        const hash = location.hash.slice(1);
        if (hash && [...buttons].some(b => b.dataset.name === hash)) {{
            select(hash);
        }} else {{
            select('{first}');
        }}
    </script>
</body>
</html>
"##,
        count = examples.len(),
    )
}
