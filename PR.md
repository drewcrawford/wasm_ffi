# Add Node.js `worker_threads` support for atomics builds

Adds `initSync({ module, memory, thread_stack_size })` to Node.js targets when building with atomics/shared memory enabled, allowing worker threads to initialize with a shared WebAssembly module and memory.

## Problem

When targeting Node.js with atomics enabled, worker threads have no way to initialize with a shared WebAssembly module and memory. The web target has `initSync({ module, memory })`, but Node.js targets were missing this - blocking libraries like `wasm_safe_thread` and `wasm_thread` from supporting Node.js.

## Solution

Generate `initSync({ module, memory, thread_stack_size })` for Node.js targets (both CommonJS and ESM) with atomics enabled. This follows the same pattern as the web target:

1. Main thread: Use `wasm_bindgen::module()` and `wasm_bindgen::memory()` Rust intrinsics to get the module and memory
2. Main thread: Pass module/memory to worker via `postMessage`
3. Worker thread: Call `initSync({ module, memory, thread_stack_size })` to initialize with shared state

### Usage with wasm_safe_thread

```rust
// Main thread - passes module/memory to worker via postMessage
let worker = wasm_safe_thread_spawn_worker(
    work,
    wasm_bindgen::module(),   // Rust intrinsic
    wasm_bindgen::memory(),   // Rust intrinsic
    name,
    shim_name,
    exit_state_ptr,
);
```

```javascript
// Worker thread - receives module/memory and initializes
parentPort.on("message", async (msg) => {
  const [module, memory, work] = msg;
  const shim = await import(shimUrl);
  shim.initSync({ module, memory, thread_stack_size: 1048576 });
  // ... run work
});
```

### Additional JS exports

For convenience, also exports:
- `__wbg_wasm_module` - The compiled WebAssembly.Module
- `__wbg_memory` - The shared WebAssembly.Memory
- `__wbg_get_imports(memory)` - Get imports object with custom memory

These allow pure-JS worker spawning without using Rust intrinsics:

```javascript
// Main thread
import { __wbg_memory, __wbg_wasm_module } from './pkg/my_module.js';
worker.postMessage({ module: __wbg_wasm_module, memory: __wbg_memory });

// Worker thread
import { initSync } from './pkg/my_module.js';
initSync({ module: workerData.module, memory: workerData.memory });
```

## Additional Changes

- Consistent error format: Changed `throw 'invalid stack size'` to `throw new Error('invalid stack size')` across all targets for better stack traces
- TypeScript declarations: Full type definitions for `InitSyncOptions`, `initSync`, `__wbg_get_imports`, `__wbg_wasm_module`, `__wbg_memory`
- Auto-initialization on main thread only: Uses `isMainThread` check so workers don't auto-initialize

## Test Plan

- Added `examples/nodejs-threads` with tests for both CJS and ESM:
  - Main thread auto-initialization
  - Worker thread initialization with shared memory
  - Atomic counter synchronization across threads
  - Memory growth visibility across threads
- Updated reference tests for `--target nodejs` and `--target experimental-nodejs-module` with atomics

## Target Consistency

| Target | `initSync({ module, memory })` | Module/Memory Access |
|--------|-------------------------------|---------------------|
| Web | ✅ Existed | Rust intrinsics |
| Node.js CJS | ✅ This PR | Rust intrinsics + JS exports |
| Node.js ESM | ✅ This PR | Rust intrinsics + JS exports |
| Deno | ❓ Not tested | |
| Bundler | ❌ Needs bundler support | |

## Related

* Unblocks `wasm_safe_thread` and `wasm_thread` Node.js support
* #4882, #4879, #4921
