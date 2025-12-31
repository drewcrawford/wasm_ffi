
let imports = {};
import * as import0 from './reference_test_bg.js';
imports['./reference_test_bg.js'] = import0;
import { readFileSync } from 'node:fs';
import { isMainThread } from 'node:worker_threads';

const __bg = imports['./reference_test_bg.js'];
let wasm;
let wasmModule;
let __initialized = false;

export function initSync(opts = {}) {
    if (__initialized) return wasm;

    let { module, memory, thread_stack_size } = opts;

    if (module === undefined) {
        const wasmUrl = new URL('reference_test_bg.wasm', import.meta.url);
        module = readFileSync(wasmUrl);
    }

    if (!(module instanceof WebAssembly.Module)) {
        wasmModule = new WebAssembly.Module(module);
    } else {
        wasmModule = module;
    }

    const wasmImports = __bg.__wbg_get_imports(memory);
    const instance = new WebAssembly.Instance(wasmModule, wasmImports);
    wasm = instance.exports;

    __bg.__wbg_set_wasm(wasm, wasmModule);

    if (typeof thread_stack_size !== 'undefined' && (typeof thread_stack_size !== 'number' || thread_stack_size === 0 || thread_stack_size % 65536 !== 0)) { throw new Error('invalid stack size'); }
    wasm.__wbindgen_start(thread_stack_size);

    __initialized = true;
    return wasm;
}

// Auto-initialize for backwards compatibility (only on main thread)
// Worker threads should call initSync({ module, memory }) explicitly
if (isMainThread) {
    initSync();
}

export { wasm as __wasm, wasmModule as __wbindgen_wasm_module };

export * from "./reference_test_bg.js";