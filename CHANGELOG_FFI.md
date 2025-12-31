# worker-logs-capture

Capture console.log/debug/info/warn/error from user-spawned Workers and SharedWorkers in browser tests.

Previously, only console output from the main thread and the test-runner's own worker was captured. Now logs from any worker created by test code are also forwarded to CLI output.

# CI

Remove codspeed CI
Re-bless with latest nightly

# realtime-headless-output

Add realtime output to headless browser mode.

PR: https://github.com/wasm-bindgen/wasm-bindgen/pull/4845

# add-headless_output-benchmark

Add a logging benchmark to headless browser mode.

WASM_BINDGEN_TEST_TIMEOUT=500 cargo bench --target=wasm32-unknown-unknown

# improve-logging-performance-in

Improve logging performance by orders of magnitude

PR: https://github.com/wasm-bindgen/wasm-bindgen/pull/4860
