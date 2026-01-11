//! Execution of doctests (tests with a `main` function instead of `__wbgt_*` exports)
//!
//! Doctests are simpler than regular wasm-bindgen tests - they just have a `main`
//! function that should be called. Unlike regular tests, they don't use the
//! WasmBindgenTestContext infrastructure.

use std::path::Path;
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context, Error};

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
        // ES module format - async initialization
        format!(
            r#"
import {{ exit }} from 'node:process';
import init, * as wasm from './{module}.js';

async function main() {{
    // Initialize the wasm module
    const instance = await init();

    // Call the main function - doctests export a `main` function
    if (typeof instance.main === 'function') {{
        instance.main();
    }} else if (typeof wasm.main === 'function') {{
        wasm.main();
    }} else {{
        throw new Error('No main function found in doctest wasm module');
    }}
}}

main()
    .then(() => {{
        console.log('test result: ok. 1 passed; 0 failed');
        exit(0);
    }})
    .catch(e => {{
        console.error('Doctest failed:', e);
        console.log('test result: FAILED. 0 passed; 1 failed');
        exit(1);
    }});
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
