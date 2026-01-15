//! Execution of doctests (tests with a `main` function instead of `__wbgt_*` exports)
//!
//! Doctests are simpler than regular wasm-bindgen tests - they just have a `main`
//! function that should be called. Unlike regular tests, they don't use the
//! WasmBindgenTestContext infrastructure.

use std::path::Path;
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context, Error};
use tempfile::tempdir;

/// Execute a doctest in Node.js by calling its `main` function.
pub fn execute_node(module: &str, tmpdir: &Path, module_format: bool) -> Result<(), Error> {
    let js_to_execute = if !module_format {
        // CommonJS format - wasm is loaded synchronously
        format!(
            r#"
const {{ exit }} = require('node:process');
const wasm = require('./{module}.js');

// For Node.js CommonJS, wasm-bindgen exports __wasm containing the wasm exports
// The module is already initialized synchronously
try {{
    if (typeof wasm.__wasm.main === 'function') {{
        wasm.__wasm.main();
    }} else {{
        throw new Error('No main function found in doctest wasm module');
    }}
    console.log('test result: ok. 1 passed; 0 failed');
    exit(0);
}} catch (e) {{
    console.error('Doctest failed:', e);
    console.log('test result: FAILED. 0 passed; 1 failed');
    exit(1);
}}
"#
        )
    } else {
        // ES module format - module is auto-initialized on import
        // wasm exports are accessed via wasm.__wasm (same as CommonJS)
        format!(
            r#"
import {{ exit }} from 'node:process';
import * as wasm from './{module}.js';

// For Node.js ES modules, wasm-bindgen exports __wasm containing the wasm exports
// The module is already initialized when imported
try {{
    if (typeof wasm.__wasm.main === 'function') {{
        wasm.__wasm.main();
    }} else {{
        throw new Error('No main function found in doctest wasm module');
    }}
    console.log('test result: ok. 1 passed; 0 failed');
    exit(0);
}} catch (e) {{
    console.error('Doctest failed:', e);
    console.log('test result: FAILED. 0 passed; 1 failed');
    exit(1);
}}
"#
        )
    };

    let js_path = if module_format {
        // For ES modules, need package.json with type: module
        let package_json = tmpdir.join("package.json");
        fs::write(&package_json, r#"{"type": "module"}"#).unwrap();
        tmpdir.join("run.mjs")
    } else {
        tmpdir.join("run.cjs")
    };
    fs::write(&js_path, js_to_execute).context("failed to write JS file")?;

    // Augment `NODE_PATH` so imports work correctly
    let path = env::var("NODE_PATH").unwrap_or_default();
    let mut path = env::split_paths(&path).collect::<Vec<_>>();
    path.push(env::current_dir().unwrap());
    path.push(tmpdir.to_path_buf());
    let extra_node_args = env::var("NODE_ARGS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let status = Command::new("node")
        .env("NODE_PATH", env::join_paths(&path).unwrap())
        .args(&extra_node_args)
        .arg(&js_path)
        .status()
        .context("failed to find or execute Node.js")?;

    if !status.success() {
        bail!("Node failed with exit_code {}", status.code().unwrap_or(1))
    }

    Ok(())
}

/// Execute a doctest in Node.js using fallback mode (without wasm-bindgen processing).
///
/// This is used when wasm-bindgen CLI fails to process the wasm file (e.g., when the
/// doctest imports wasm-bindgen types but doesn't actually use them at runtime).
/// We provide stub implementations for wasm-bindgen imports and execute the wasm directly.
pub fn execute_node_fallback(wasm_path: &Path) -> Result<(), Error> {
    let tmpdir = tempdir()?;
    let tmpdir_path = tmpdir.path();

    // Copy the wasm file to the temp directory
    let wasm_dest = tmpdir_path.join("doctest.wasm");
    fs::copy(wasm_path, &wasm_dest).context("failed to copy wasm file")?;

    // JavaScript that loads the wasm with stub imports and calls main()
    let js_to_execute = r#"
const { exit } = require('node:process');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');

// Stub imports for wasm-bindgen functions that may be imported but not called
const stubImports = {
    __wbindgen_placeholder__: new Proxy({}, {
        get: (target, prop) => {
            // Return a stub function for any requested import
            return (...args) => {
                // __wbindgen_describe is called at build time, not runtime - no-op
                if (prop === '__wbindgen_describe') return;
                // For other functions, if they're actually called at runtime,
                // the test should fail
                throw new Error(`wasm-bindgen stub called: ${prop}. This doctest requires wasm-bindgen-test support.`);
            };
        }
    }),
    __wbindgen_externref_xform__: new Proxy({}, {
        get: (target, prop) => {
            return (...args) => {
                throw new Error(`externref stub called: ${prop}. This doctest requires wasm-bindgen-test support.`);
            };
        }
    }),
    // Provide a minimal env if needed
    env: {}
};

async function run() {
    try {
        const wasmPath = join(__dirname, 'doctest.wasm');
        const wasmBytes = readFileSync(wasmPath);
        const wasmModule = await WebAssembly.compile(wasmBytes);

        // Get the imports the module needs
        const moduleImports = WebAssembly.Module.imports(wasmModule);

        // Build import object with stubs for all required imports
        const imports = {};
        for (const imp of moduleImports) {
            if (!imports[imp.module]) {
                imports[imp.module] = stubImports[imp.module] || {};
            }
        }

        const instance = await WebAssembly.instantiate(wasmModule, imports);

        if (typeof instance.exports.main !== 'function') {
            throw new Error('No main function found in doctest wasm module');
        }

        instance.exports.main();

        console.log('test result: ok. 1 passed; 0 failed');
        console.log('');
        console.log('note: This doctest ran in fallback mode without wasm-bindgen.');
        console.log('      Console output from the test was not captured.');
        exit(0);
    } catch (e) {
        console.error('Doctest failed:', e.message || e);
        console.log('test result: FAILED. 0 passed; 1 failed');
        console.log('');
        console.log('note: This doctest ran in fallback mode without wasm-bindgen.');
        console.log('      For better error messages, add wasm_bindgen_test imports.');
        exit(1);
    }
}

run();
"#;

    let js_path = tmpdir_path.join("run.cjs");
    fs::write(&js_path, js_to_execute).context("failed to write JS file")?;

    let extra_node_args = env::var("NODE_ARGS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let status = Command::new("node")
        .current_dir(tmpdir_path)
        .args(&extra_node_args)
        .arg(&js_path)
        .status()
        .context("failed to find or execute Node.js")?;

    if !status.success() {
        bail!("Node failed with exit_code {}", status.code().unwrap_or(1))
    }

    Ok(())
}

/// Execute a doctest in Deno by calling its `main` function.
pub fn execute_deno(module: &str, tmpdir: &Path) -> Result<(), Error> {
    // Deno uses ES modules - import the wasm-bindgen generated module
    // and access exports via __wasm (same as regular Deno tests)
    let js_to_execute = format!(
        r#"import * as wasm from "./{module}.js";

try {{
    if (typeof wasm.__wasm.main === 'function') {{
        wasm.__wasm.main();
    }} else {{
        throw new Error('No main function found in doctest wasm module');
    }}
    console.log("test result: ok. 1 passed; 0 failed");
}} catch (e) {{
    console.error("Doctest failed:", e);
    console.log("test result: FAILED. 0 passed; 1 failed");
    Deno.exit(1);
}}
"#
    );

    let js_path = tmpdir.join("run.js");
    fs::write(&js_path, &js_to_execute).context("failed to write JS file")?;

    let status = Command::new("deno")
        .arg("run")
        .arg("--allow-read")
        .arg(&js_path)
        .status()
        .context("failed to find or execute Deno")?;

    if !status.success() {
        bail!("Deno failed with exit_code {}", status.code().unwrap_or(1))
    }

    Ok(())
}
