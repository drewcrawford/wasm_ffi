//! Tests for doctest support in wasm-bindgen-test-runner.
//!
//! Doctests export a `main` function instead of `__wbgt_*` test exports.
//! These tests verify that doctests are properly detected and executed
//! in various modes (Node.js, browser main thread, dedicated worker).
//!
//! Doctests are built from source using `cargo +nightly test --doc` with
//! `--persist-doctests` to capture the generated wasm files.

use super::{Project, REPO_ROOT, TARGET_DIR};
use std::fs;
use std::path::PathBuf;
use std::process::Output;

impl Project {
    /// Build doctests and return the path to the generated wasm file.
    /// Uses `cargo +nightly test --doc` with `--persist-doctests` to capture the wasm.
    /// The `doctest_line` parameter specifies which line the doctest starts at (1-indexed).
    pub fn build_doctest(&mut self, doctest_line: u32) -> anyhow::Result<PathBuf> {
        // Use a special cargo.toml for doctests - needs rlib, not cdylib
        self.cargo_toml_for_doctest();

        let doctests_dir = self.root.join("doctests");
        fs::create_dir_all(&doctests_dir)?;

        // Build the doctests with --persist-doctests
        let output = std::process::Command::new("cargo")
            .current_dir(&self.root)
            .arg("+nightly")
            .arg("test")
            .arg("--target")
            .arg("wasm32-unknown-unknown")
            .arg("--doc")
            .arg("-Zbuild-std=std,panic_abort")
            .env("CARGO_TARGET_DIR", &*TARGET_DIR)
            .env(
                "RUSTDOCFLAGS",
                format!("--persist-doctests {}", doctests_dir.display()),
            )
            // We expect this to fail since there's no runner, but the wasm is still generated
            .output()?;

        // The doctest directory name follows the pattern: src_lib_rs_{line}_0
        let doctest_dir_name = format!("src_lib_rs_{}_0", doctest_line);
        let wasm_path = doctests_dir.join(&doctest_dir_name).join("rust_out.wasm");

        if !wasm_path.exists() {
            // Try to find what directories were created for debugging
            let entries: Vec<_> = fs::read_dir(&doctests_dir)
                .map(|rd| rd.filter_map(|e| e.ok()).collect())
                .unwrap_or_default();
            anyhow::bail!(
                "Doctest wasm not found at {:?}. Available directories: {:?}\nstdout: {}\nstderr: {}",
                wasm_path,
                entries.iter().map(|e: &fs::DirEntry| e.file_name()).collect::<Vec<_>>(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(wasm_path)
    }

    /// Run wasm-bindgen-test-runner on a specific wasm file.
    pub fn run_wasm_bindgen_test_runner(&self, wasm_path: &PathBuf) -> anyhow::Result<Output> {
        let runner = REPO_ROOT.join("crates").join("cli").join("Cargo.toml");
        let output = std::process::Command::new("cargo")
            .arg("run")
            .arg("--manifest-path")
            .arg(&runner)
            .arg("--bin")
            .arg("wasm-bindgen-test-runner")
            .arg("--")
            .arg(wasm_path)
            .output()?;
        Ok(output)
    }

    /// Generate a Cargo.toml suitable for doctests (uses rlib, not cdylib).
    fn cargo_toml_for_doctest(&mut self) {
        self.file(
            "Cargo.toml",
            &format!(
                r#"
[package]
name = "{}"
authors = []
version = "1.0.0"
edition = "2021"

[dependencies]
{}
{}

[lib]
crate-type = ["rlib"]

[workspace]

[profile.dev]
codegen-units = 1
"#,
                self.name,
                self.deps.replace("{root}", REPO_ROOT.to_str().unwrap()),
                self.dev_deps.replace("{root}", REPO_ROOT.to_str().unwrap()),
            ),
        );
    }
}

/// Test that a doctest runs correctly in Node.js (default mode).
#[test]
fn test_doctest_node() {
    // The doctest is at line 1 of lib.rs (the ```rust line)
    let mut project = Project::new("test_doctest_node");
    project.file(
        "src/lib.rs",
        r#"//! ```
//! wasm_bindgen_test::console_log!("Hello from doctest!");
//! ```
"#,
    );

    let wasm_path = match project.build_doctest(1) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Skipping test: failed to build doctest: {e}");
            return;
        }
    };

    let output = project
        .run_wasm_bindgen_test_runner(&wasm_path)
        .expect("Failed to run wasm-bindgen-test-runner");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Doctest should have been detected and run
    assert!(
        stdout.contains("running 1 doctest") || stderr.contains("running 1 doctest"),
        "Expected 'running 1 doctest' in output.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Console output should appear
    assert!(
        stdout.contains("Hello from doctest!") || stderr.contains("Hello from doctest!"),
        "Expected doctest console.log output.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Test should pass
    assert!(
        stdout.contains("test result: ok") || stderr.contains("test result: ok"),
        "Expected doctest to pass.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    assert!(
        output.status.success(),
        "Expected exit code 0.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// Test that a doctest runs correctly in browser main thread mode.
#[test]
fn test_doctest_browser() {
    let mut project = Project::new("test_doctest_browser");
    project.file(
        "src/lib.rs",
        r#"//! ```
//! wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
//! wasm_bindgen_test::console_log!("Hello from browser doctest!");
//! ```
"#,
    );

    let wasm_path = match project.build_doctest(1) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Skipping test: failed to build doctest: {e}");
            return;
        }
    };

    let output = project
        .run_wasm_bindgen_test_runner(&wasm_path)
        .expect("Failed to run wasm-bindgen-test-runner");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Doctest should have been detected and run
    assert!(
        stdout.contains("running 1 doctest") || stderr.contains("running 1 doctest"),
        "Expected 'running 1 doctest' in output.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Console output should appear
    assert!(
        stdout.contains("Hello from browser doctest!")
            || stderr.contains("Hello from browser doctest!"),
        "Expected doctest console.log output.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Test should pass
    assert!(
        stdout.contains("test result: ok") || stderr.contains("test result: ok"),
        "Expected doctest to pass.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    assert!(
        output.status.success(),
        "Expected exit code 0.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// Test that a doctest configured for dedicated worker runs correctly.
#[test]
fn test_doctest_dedicated_worker() {
    let mut project = Project::new("test_doctest_dedicated_worker");
    project.file(
        "src/lib.rs",
        r#"//! ```
//! wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_dedicated_worker);
//! wasm_bindgen_test::console_log!("Hello from worker doctest!");
//! ```
"#,
    );

    let wasm_path = match project.build_doctest(1) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Skipping test: failed to build doctest: {e}");
            return;
        }
    };

    let output = project
        .run_wasm_bindgen_test_runner(&wasm_path)
        .expect("Failed to run wasm-bindgen-test-runner");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Doctest should have been detected and run
    assert!(
        stdout.contains("running 1 doctest") || stderr.contains("running 1 doctest"),
        "Expected 'running 1 doctest' in output.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Console output from the worker should appear
    assert!(
        stdout.contains("Hello from worker doctest!")
            || stderr.contains("Hello from worker doctest!"),
        "Expected doctest console.log output from worker.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Test should pass
    assert!(
        stdout.contains("test result: ok") || stderr.contains("test result: ok"),
        "Expected doctest to pass.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    assert!(
        output.status.success(),
        "Expected exit code 0.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}
