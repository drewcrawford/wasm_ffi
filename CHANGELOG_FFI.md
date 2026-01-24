# doctest-support

Actually run doctests in wasm-bindgen-test-runner. Previously, doctests were silently skipped because they export a `main` function instead of `__wbgt_*` test exports. The runner would print "no tests to run!" and exit 0, causing the tooling to report the doctest as passed.

Now doctests are detected and executed properly. Supports:
- Node.js CommonJS (default, no configuration needed)
- Node.js ES modules (`wasm_bindgen_test_configure!(run_in_node_experimental)`)
- Deno (`WASM_BINDGEN_USE_DENO=1` environment variable)
- Browser main thread (`wasm_bindgen_test_configure!(run_in_browser)`)
- Dedicated worker (`wasm_bindgen_test_configure!(run_in_dedicated_worker)`)
- Shared worker (`wasm_bindgen_test_configure!(run_in_shared_worker)`)
- Service worker (`wasm_bindgen_test_configure!(run_in_service_worker)`)

# nodejs-threads

Add Node.js `worker_threads` support for atomics builds. Supports both CommonJS (`--target nodejs`) and ESM (`--target experimental-nodejs-module`) targets. When targeting Node.js with atomics enabled, wasm-bindgen now generates:

- `initSync({ module, memory, thread_stack_size })` - Initialize WASM synchronously with shared module and memory
- `__wbg_get_imports(memory)` - Get imports object with optional custom memory
- Auto-initialization on main thread only (backwards compatible)

This enables spawning worker threads that share memory with the main thread.

See `examples/nodejs-threads` for a complete example.

# worker-panic-capture

Improve panic capture from worker threads in browser tests. Panics from dedicated workers and shared workers are now properly captured and reported.

# tty-detection

Shell status messages (e.g., "Loading page elements...") are now suppressed when stdout is not a TTY. This produces cleaner output in CI environments and when piping output.

# worker-logs-capture

Capture console.log/debug/info/warn/error from user-spawned Workers and SharedWorkers in browser tests.

Previously, only console output from the main thread and the test-runner's own worker was captured. Now logs from any worker created by test code are also forwarded to CLI output.

# CI

Remove codecov and codspeed CI workflows
Re-bless with latest nightly
Add bench_wasm CI job for running WASM benchmarks

# realtime-headless-output

Add realtime output to headless browser mode.

PR: https://github.com/wasm-bindgen/wasm-bindgen/pull/4845

# add-headless_output-benchmark

Add a logging benchmark to headless browser mode.

WASM_BINDGEN_TEST_TIMEOUT=500 cargo bench --target=wasm32-unknown-unknown

# improve-logging-performance-in

Improve logging performance by orders of magnitude

PR: https://github.com/wasm-bindgen/wasm-bindgen/pull/4860
