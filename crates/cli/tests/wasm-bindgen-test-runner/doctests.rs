//! Tests for doctest support in wasm-bindgen-test-runner.
//!
//! Doctests export a `main` function instead of `__wbgt_*` test exports.
//! These tests verify that doctests are properly detected and executed
//! in various modes (Node.js, browser main thread, dedicated worker).
//!
//! Doctests are built from source using `cargo +nightly test --doc` with
//! `--persist-doctests` to capture the generated wasm files.
//!
//! These tests require nightly Rust and will be skipped if nightly is not available.

use super::{Project, REPO_ROOT, TARGET_DIR};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

/// Check if nightly toolchain is available. Cached for performance.
fn has_nightly() -> bool {
    static HAS_NIGHTLY: OnceLock<bool> = OnceLock::new();
    *HAS_NIGHTLY.get_or_init(|| {
        Command::new("cargo")
            .arg("+nightly")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// Check if Deno is available. Cached for performance.
fn has_deno() -> bool {
    static HAS_DENO: OnceLock<bool> = OnceLock::new();
    *HAS_DENO.get_or_init(|| {
        Command::new("deno")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

/// Skip a test if nightly is not available. Panics if nightly IS available
/// (meaning the test should have worked but failed for another reason).
macro_rules! require_nightly_or_skip {
    ($result:expr) => {
        match $result {
            Ok(path) => path,
            Err(e) => {
                if has_nightly() {
                    panic!("Nightly is available but doctest build failed: {e}");
                } else {
                    eprintln!("Skipping test: nightly toolchain not available");
                    return;
                }
            }
        }
    };
}

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
    pub fn run_wasm_bindgen_test_runner(&self, wasm_path: &Path) -> anyhow::Result<Output> {
        self.run_wasm_bindgen_test_runner_with_env(wasm_path, &[])
    }

    /// Run wasm-bindgen-test-runner on a specific wasm file with custom environment variables.
    pub fn run_wasm_bindgen_test_runner_with_env(
        &self,
        wasm_path: &Path,
        envs: &[(&str, &str)],
    ) -> anyhow::Result<Output> {
        let runner = REPO_ROOT.join("crates").join("cli").join("Cargo.toml");
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("run")
            .arg("--manifest-path")
            .arg(&runner)
            .arg("--bin")
            .arg("wasm-bindgen-test-runner")
            .arg("--")
            .arg(wasm_path);
        for (key, value) in envs {
            cmd.env(key, value);
        }
        let output = cmd.output()?;
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

    /// Build lib tests and return the output of the cargo test command.
    /// This runs `cargo test --target wasm32-unknown-unknown --lib`.
    pub fn build_and_run_lib_tests(&mut self) -> anyhow::Result<Output> {
        self.cargo_toml_for_both_test_types();

        let runner = REPO_ROOT.join("crates").join("cli").join("Cargo.toml");
        let output = std::process::Command::new("cargo")
            .current_dir(&self.root)
            .arg("test")
            .arg("--target")
            .arg("wasm32-unknown-unknown")
            .arg("--lib")
            .env("CARGO_TARGET_DIR", &*TARGET_DIR)
            .env(
                "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER",
                format!(
                    "cargo run --manifest-path {} --bin wasm-bindgen-test-runner --",
                    runner.display()
                ),
            )
            .output()?;
        Ok(output)
    }

    /// Build doctests and return the path to the generated wasm file.
    /// Uses both rlib and cdylib so that both lib tests and doctests can be built.
    pub fn build_doctest_with_libtests(&mut self, doctest_line: u32) -> anyhow::Result<PathBuf> {
        self.cargo_toml_for_both_test_types();

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

    /// Generate a Cargo.toml suitable for both lib tests and doctests.
    /// Uses both rlib (for doctests) and cdylib (for lib tests).
    fn cargo_toml_for_both_test_types(&mut self) {
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

[dev-dependencies]
{}

[lib]
crate-type = ["rlib", "cdylib"]

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

    let wasm_path = require_nightly_or_skip!(project.build_doctest(1));

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

    let wasm_path = require_nightly_or_skip!(project.build_doctest(1));

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

    let wasm_path = require_nightly_or_skip!(project.build_doctest(1));

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

/// Test that a doctest configured for Node.js ES module mode runs correctly.
#[test]
fn test_doctest_node_experimental() {
    let mut project = Project::new("test_doctest_node_experimental");
    project.file(
        "src/lib.rs",
        r#"//! ```
//! wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_node_experimental);
//! wasm_bindgen_test::console_log!("Hello from node experimental doctest!");
//! ```
"#,
    );

    let wasm_path = require_nightly_or_skip!(project.build_doctest(1));

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
        stdout.contains("Hello from node experimental doctest!")
            || stderr.contains("Hello from node experimental doctest!"),
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

/// Test that a doctest configured for shared worker runs correctly.
#[test]
fn test_doctest_shared_worker() {
    let mut project = Project::new("test_doctest_shared_worker");
    project.file(
        "src/lib.rs",
        r#"//! ```
//! wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_shared_worker);
//! wasm_bindgen_test::console_log!("Hello from shared worker doctest!");
//! ```
"#,
    );

    let wasm_path = require_nightly_or_skip!(project.build_doctest(1));

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
        stdout.contains("Hello from shared worker doctest!")
            || stderr.contains("Hello from shared worker doctest!"),
        "Expected doctest console.log output from shared worker.\nstdout:\n{stdout}\nstderr:\n{stderr}"
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

/// Test that a doctest configured for service worker runs correctly.
#[test]
fn test_doctest_service_worker() {
    let mut project = Project::new("test_doctest_service_worker");
    project.file(
        "src/lib.rs",
        r#"//! ```
//! wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_service_worker);
//! wasm_bindgen_test::console_log!("Hello from service worker doctest!");
//! ```
"#,
    );

    let wasm_path = require_nightly_or_skip!(project.build_doctest(1));

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
        stdout.contains("Hello from service worker doctest!")
            || stderr.contains("Hello from service worker doctest!"),
        "Expected doctest console.log output from service worker.\nstdout:\n{stdout}\nstderr:\n{stderr}"
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

/// Test that a doctest runs correctly in Deno.
/// Deno mode is triggered via WASM_BINDGEN_USE_DENO environment variable.
#[test]
fn test_doctest_deno() {
    if !has_deno() {
        eprintln!("Skipping test: Deno not available");
        return;
    }

    // For Deno, we use a plain doctest (no configure macro) and set env var
    let mut project = Project::new("test_doctest_deno");
    project.file(
        "src/lib.rs",
        r#"//! ```
//! wasm_bindgen_test::console_log!("Hello from deno doctest!");
//! ```
"#,
    );

    let wasm_path = require_nightly_or_skip!(project.build_doctest(1));

    let output = project
        .run_wasm_bindgen_test_runner_with_env(&wasm_path, &[("WASM_BINDGEN_USE_DENO", "1")])
        .expect("Failed to run wasm-bindgen-test-runner");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Doctest should have been detected and run
    assert!(
        stdout.contains("running 1 doctest") || stderr.contains("running 1 doctest"),
        "Expected 'running 1 doctest' in output.\nstdout:\n{stdout}\nstderr:\n{stderr}"
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

/// Test that doctests are detected when a project also has lib tests.
/// This reproduces a bug where `cargo test` wouldn't detect doctests
/// when the project also contains wasm_bindgen_test lib tests.
///
/// This test builds a crate with both crate-type = ["rlib", "cdylib"] to support
/// both lib tests (which need cdylib) and doctests (which need rlib).
#[test]
fn test_doctest_with_libtests() {
    // Create a project that has BOTH a doctest AND a lib test
    let mut project = Project::new("test_doctest_with_libtests");
    project.file(
        "src/lib.rs",
        r#"//! Module with both doctests and lib tests
//!
//! ```
//! wasm_bindgen_test::console_log!("Hello from doctest!");
//! ```

/// A function that does nothing
pub fn do_nothing() {}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_lib() {
        console_log!("Hello from lib test!");
    }
}
"#,
    );

    // Build the doctest using the combined config (rlib + cdylib)
    // Doctest is at line 3 (the ```rust line)
    let wasm_path = require_nightly_or_skip!(project.build_doctest_with_libtests(3));

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

/// Test the full workflow: run lib tests first, then doctests.
/// This more closely simulates what happens when a user runs `cargo test` which
/// would run lib tests, then `cargo test --doc` for doctests.
#[test]
fn test_lib_then_doctest_sequence() {
    let mut project = Project::new("test_lib_then_doctest_sequence");
    project.file(
        "src/lib.rs",
        r#"//! Module with both doctests and lib tests
//!
//! ```
//! wasm_bindgen_test::console_log!("Hello from sequential doctest!");
//! ```

/// A function that does nothing
pub fn do_nothing() {}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_lib_sequential() {
        console_log!("Hello from sequential lib test!");
    }
}
"#,
    );

    // Step 1: Run lib tests first
    let lib_output = project
        .build_and_run_lib_tests()
        .expect("Failed to build/run lib tests");

    let lib_stdout = String::from_utf8_lossy(&lib_output.stdout);
    let lib_stderr = String::from_utf8_lossy(&lib_output.stderr);

    // Lib test should have run
    assert!(
        lib_stdout.contains("running 1 test") || lib_stderr.contains("running 1 test"),
        "Expected 'running 1 test' for lib tests.\nstdout:\n{lib_stdout}\nstderr:\n{lib_stderr}"
    );

    assert!(
        lib_output.status.success(),
        "Expected lib tests to pass.\nstdout:\n{lib_stdout}\nstderr:\n{lib_stderr}"
    );

    // Step 2: Now build and run doctests
    let wasm_path = require_nightly_or_skip!(project.build_doctest_with_libtests(3));

    let doc_output = project
        .run_wasm_bindgen_test_runner(&wasm_path)
        .expect("Failed to run wasm-bindgen-test-runner on doctest");

    let doc_stdout = String::from_utf8_lossy(&doc_output.stdout);
    let doc_stderr = String::from_utf8_lossy(&doc_output.stderr);

    // Doctest should have been detected and run (not mistaken for a lib test)
    assert!(
        doc_stdout.contains("running 1 doctest") || doc_stderr.contains("running 1 doctest"),
        "Expected 'running 1 doctest' in output after running lib tests.\n\
         The doctest should be detected even after lib tests were run.\n\
         stdout:\n{doc_stdout}\nstderr:\n{doc_stderr}"
    );

    // Console output should appear
    assert!(
        doc_stdout.contains("Hello from sequential doctest!")
            || doc_stderr.contains("Hello from sequential doctest!"),
        "Expected doctest console.log output.\nstdout:\n{doc_stdout}\nstderr:\n{doc_stderr}"
    );

    assert!(
        doc_output.status.success(),
        "Expected doctest to pass.\nstdout:\n{doc_stdout}\nstderr:\n{doc_stderr}"
    );
}

/// Test running `cargo test` (without --doc) when the project has both doctests and lib tests.
/// This verifies that both lib tests AND doctests run when using `cargo test` without `--doc`.
///
/// Note: Doctests run through rustdoc, which captures the wasm-bindgen-test-runner's output
/// and displays results in its own format. The runner IS invoked, but its "running 1 doctest"
/// message may not appear directly in cargo's output.
#[test]
fn test_cargo_test_without_doc_flag() {
    let mut project = Project::new("test_cargo_test_without_doc_flag");
    project.file(
        "src/lib.rs",
        r#"//! Module with both doctests and lib tests
//!
//! ```
//! wasm_bindgen_test::console_log!("Hello from no-flag doctest!");
//! ```

/// A function that does nothing
pub fn do_nothing() {}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_lib_no_flag() {
        console_log!("Hello from no-flag lib test!");
    }
}
"#,
    );

    // Set up cargo.toml with both rlib and cdylib
    project.file(
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

[dev-dependencies]
{}

[lib]
crate-type = ["rlib", "cdylib"]

[workspace]

[profile.dev]
codegen-units = 1
"#,
            project.name,
            project.deps.replace("{root}", REPO_ROOT.to_str().unwrap()),
            project
                .dev_deps
                .replace("{root}", REPO_ROOT.to_str().unwrap()),
        ),
    );

    // Run cargo test WITHOUT --doc (simulating what users typically do)
    let runner = REPO_ROOT.join("crates").join("cli").join("Cargo.toml");
    let output = std::process::Command::new("cargo")
        .current_dir(&project.root)
        .arg("+nightly")
        .arg("test")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        // NOTE: No --doc flag - this is the scenario we're testing
        .arg("-Zbuild-std=std,panic_abort")
        .env("CARGO_TARGET_DIR", &*TARGET_DIR)
        .env(
            "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER",
            format!(
                "cargo run --manifest-path {} --bin wasm-bindgen-test-runner --",
                runner.display()
            ),
        )
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            if has_nightly() {
                panic!("Nightly is available but cargo test failed to start: {e}");
            } else {
                eprintln!("Skipping test: nightly toolchain not available");
                return;
            }
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check if nightly with build-std is working
    if !output.status.success() && !has_nightly() {
        eprintln!("Skipping test: nightly toolchain or build-std not available");
        return;
    }

    // The lib test should run
    assert!(
        stdout.contains("running 1 test") || stderr.contains("running 1 test"),
        "Expected lib tests to run with 'running 1 test'.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Doctests should also run (shown as "Doc-tests" in cargo output)
    assert!(
        stdout.contains("Doc-tests") || stderr.contains("Doc-tests"),
        "Expected doctests to run ('Doc-tests' section).\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // The test should succeed
    assert!(
        output.status.success(),
        "Expected cargo test to succeed.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// Test that merged doctests (Rust's new doctest format) are detected correctly.
/// Merged doctests use a different naming convention than the older `__doctest_main_*` format.
/// They use `doctest_bundle_*::__doctest_*` and `doctest_runner_*::__doctest_*` patterns.
///
/// This test compiles a project with BOTH lib tests AND doctests using edition 2024.
/// When `cargo test` runs (without `--doc`), it produces merged doctest wasm files.
/// The test verifies our test runner correctly detects these merged doctests.
#[test]
fn test_merged_doctest_detection() {
    let mut project = Project::new("test_merged_doctest_detection");
    // Need both a lib test AND a doctest to trigger merged format
    project.file(
        "src/lib.rs",
        r#"//! ```
//! wasm_bindgen_test::console_log!("Hello from merged doctest!");
//! ```

pub fn do_nothing() {}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_lib() {
        console_log!("Hello from lib test!");
    }
}
"#,
    );

    // Build the merged doctest (without --persist-doctests)
    let wasm_path = require_nightly_or_skip!(project.build_merged_doctest());

    // Now run our test runner on the captured wasm to see if it detects the doctest
    let output = project
        .run_wasm_bindgen_test_runner(&wasm_path)
        .expect("Failed to run wasm-bindgen-test-runner");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The merged doctest should be detected and run, not show "no tests to run!"
    assert!(
        !stdout.contains("no tests to run!"),
        "Merged doctest was not detected. The runner said 'no tests to run!'.\n\
         This indicates the detection logic doesn't recognize the merged doctest format.\n\
         The merged format uses function names like `doctest_bundle_*::__doctest_*::main`\n\
         instead of the older `__doctest_main_*` pattern.\n\
         stdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // Should show it's running doctests
    assert!(
        stdout.contains("running 1 doctest") || stderr.contains("running 1 doctest"),
        "Expected 'running 1 doctest' in output.\nstdout:\n{stdout}\nstderr:\n{stderr}"
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

/// Build a merged doctest wasm file (the format used in Rust 2024 edition).
/// Merged doctests use function names like `doctest_runner_2024::main` instead of
/// the older `__doctest_main_*` format. This format is triggered when running
/// `cargo test` (without `--doc`) on a project with both lib tests and doctests
/// using edition 2024.
///
/// This method:
/// 1. Creates a project with both lib tests and doctests using edition 2024
/// 2. Uses a wrapper script to capture wasm files passed to the runner
/// 3. Returns the path to the doctest wasm (distinguished by having `doctest_runner` in function names)
impl Project {
    pub fn build_merged_doctest(&mut self) -> anyhow::Result<PathBuf> {
        // Set up Cargo.toml for edition 2024 with both rlib and cdylib
        self.cargo_toml_for_merged_doctest();

        // Create wrapper script that captures ALL wasm files
        let capture_dir = self.root.join("captured");
        fs::create_dir_all(&capture_dir)?;

        let wrapper_script = self.root.join("capture_wasm.sh");
        let wrapper_content = format!(
            r#"#!/bin/bash
# Capture each wasm to a numbered file
COUNT=$(ls "{capture_dir}"/*.wasm 2>/dev/null | wc -l | tr -d ' ')
cp "$1" "{capture_dir}/wasm_$COUNT.wasm"
# Print success so tests pass
echo "test result: ok. 1 passed"
exit 0
"#,
            capture_dir = capture_dir.display()
        );
        fs::write(&wrapper_script, wrapper_content)?;

        // Make the wrapper executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&wrapper_script, fs::Permissions::from_mode(0o755))?;
        }

        // Run cargo test WITHOUT --doc (this triggers merged doctest format in edition 2024)
        let output = std::process::Command::new("cargo")
            .current_dir(&self.root)
            .arg("+nightly")
            .arg("test")
            .arg("--target")
            .arg("wasm32-unknown-unknown")
            // NOTE: No --doc flag - we want the merged format
            .arg("-Zbuild-std=std,panic_abort")
            .env("CARGO_TARGET_DIR", &*TARGET_DIR)
            .env(
                "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER",
                wrapper_script.to_str().unwrap(),
            )
            .output()?;

        // Find the captured wasm files
        let wasm_files: Vec<_> = fs::read_dir(&capture_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "wasm"))
            .collect();

        if wasm_files.is_empty() {
            anyhow::bail!(
                "No wasm files captured.\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // The doctest wasm should be the second one (first is lib test)
        // In merged format, it will have `doctest_runner_20XX` in function names
        // For now, just return the last captured wasm (doctests run after lib tests)
        let doctest_wasm = wasm_files
            .into_iter()
            .max_by_key(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|n| n.strip_prefix("wasm_"))
                    .and_then(|n| n.strip_suffix(".wasm"))
                    .and_then(|n| n.parse::<u32>().ok())
                    .unwrap_or(0)
            })
            .ok_or_else(|| anyhow::anyhow!("No wasm files found"))?;

        Ok(doctest_wasm)
    }

    /// Generate a Cargo.toml for merged doctests (edition 2024, both rlib and cdylib).
    fn cargo_toml_for_merged_doctest(&mut self) {
        self.file(
            "Cargo.toml",
            &format!(
                r#"
[package]
name = "{}"
authors = []
version = "1.0.0"
edition = "2024"

[dependencies]
{}

[dev-dependencies]
{}

[lib]
crate-type = ["rlib", "cdylib"]

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

/// Test running cargo test --doc when the project has both doctests and lib tests.
/// This test runs the doctest via cargo with the test runner configured, which is
/// the way users actually run doctests.
///
/// Note: Rustdoc displays "running 1 test" in its output, not "running 1 doctest".
/// The wasm-bindgen-test-runner IS invoked and DOES output "running 1 doctest",
/// but rustdoc captures this output and shows its own format instead.
#[test]
fn test_cargo_test_doc_with_libtests() {
    let mut project = Project::new("test_cargo_test_doc_with_libtests");
    project.file(
        "src/lib.rs",
        r#"//! Module with both doctests and lib tests
//!
//! ```
//! wasm_bindgen_test::console_log!("Hello from cargo doctest!");
//! ```

/// A function that does nothing
pub fn do_nothing() {}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_lib_cargo() {
        console_log!("Hello from cargo lib test!");
    }
}
"#,
    );

    // Set up cargo.toml with both rlib and cdylib
    project.file(
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

[dev-dependencies]
{}

[lib]
crate-type = ["rlib", "cdylib"]

[workspace]

[profile.dev]
codegen-units = 1
"#,
            project.name,
            project.deps.replace("{root}", REPO_ROOT.to_str().unwrap()),
            project
                .dev_deps
                .replace("{root}", REPO_ROOT.to_str().unwrap()),
        ),
    );

    // Run cargo test --doc with the test runner
    let runner = REPO_ROOT.join("crates").join("cli").join("Cargo.toml");
    let output = std::process::Command::new("cargo")
        .current_dir(&project.root)
        .arg("+nightly")
        .arg("test")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--doc")
        .arg("-Zbuild-std=std,panic_abort")
        .env("CARGO_TARGET_DIR", &*TARGET_DIR)
        .env(
            "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER",
            format!(
                "cargo run --manifest-path {} --bin wasm-bindgen-test-runner --",
                runner.display()
            ),
        )
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            if has_nightly() {
                panic!("Nightly is available but cargo test --doc failed to start: {e}");
            } else {
                eprintln!("Skipping test: nightly toolchain not available");
                return;
            }
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check if nightly with build-std is working
    if !output.status.success() && !has_nightly() {
        eprintln!("Skipping test: nightly toolchain or build-std not available");
        return;
    }

    // The doctest should run - rustdoc shows "running 1 test" (not "1 doctest")
    // because rustdoc captures the runner output and displays its own format
    assert!(
        stdout.contains("running 1 test") || stderr.contains("running 1 test"),
        "Expected 'running 1 test' when running cargo test --doc.\n\
         stdout:\n{stdout}\nstderr:\n{stderr}"
    );

    // The test should pass
    assert!(
        stdout.contains("test result: ok") || stderr.contains("test result: ok"),
        "Expected doctest to pass.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    assert!(
        output.status.success(),
        "Expected cargo test --doc to succeed.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}
