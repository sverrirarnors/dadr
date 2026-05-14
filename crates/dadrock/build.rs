use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use walkdir::WalkDir;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(dadrock_no_js_build)");
    println!("cargo:rerun-if-env-changed=DADROCK_REQUIRE_JS_BUILD");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let web_dir = manifest_dir.join("web");

    if !web_dir.exists() {
        println!("cargo:warning=crates/dadrock/web does not exist; skipping JS build");
        return;
    }

    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/bun.lock");
    println!("cargo:rerun-if-changed=web/svelte.config.js");
    println!("cargo:rerun-if-changed=web/vite.config.ts");
    println!("cargo:rerun-if-changed=web/tsconfig.json");

    for entry in WalkDir::new(web_dir.join("src"))
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }

    if env::var("CARGO_FEATURE_NO_JS_BUILD").is_ok() {
        println!("cargo:warning=skipping JS build (CARGO_FEATURE_NO_JS_BUILD set)");
        println!("cargo:rustc-cfg=dadrock_no_js_build");
        return;
    }

    if which("bun").is_none() {
        if env::var_os("DADROCK_REQUIRE_JS_BUILD").is_some() {
            panic!(
                "bun not found on PATH. Install Bun (https://bun.sh) and retry, \
                 or unset DADROCK_REQUIRE_JS_BUILD to use the no-JS fallback."
            );
        }
        println!("cargo:warning=bun not found on PATH; using no-JS web fallback");
        println!("cargo:rustc-cfg=dadrock_no_js_build");
        return;
    }

    if !web_dir.join("node_modules").exists() {
        run(&web_dir, "bun", &["install", "--frozen-lockfile"]);
    }
    run(&web_dir, "bun", &["run", "build"]);
}

fn run(cwd: &Path, program: &str, args: &[&str]) {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap_or_else(|err| panic!("failed to spawn `{program} {}`: {err}", args.join(" ")));
    if !status.success() {
        panic!("`{program} {}` failed with {status}", args.join(" "));
    }
}

fn which(program: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
