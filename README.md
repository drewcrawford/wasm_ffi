# Fork manifesto

This is a fork of wasm-bindgen.  Goals:

a) high velocity; unblock first, ask questions later

b) support "modern" workloads like multicore atomics, log-based debugging, or whatever your browser of choice broke this week.

c) more predictable crates.io release schedule

d) landing pad for things we'd all like in wasm-bindgen, but may benefit from incubation inside a higher-velocity fork

e) identical license with both wasm-bindgen, and also rust. So legally any code could flow both to wasm-bindgen, and subsequently, to rust itself.

## Sunset

This fork will have at least three monthly releases:

1.x - January 2026


2.x - February 2026

3.x - March 2026, assuming breaking changes are needed

In April, we'll see if upstream has merged our patches or if we need to order another quarter of releases.

# SLA

This fork will release one new major version every month, in the first week of that month.  (Or a minor version, if no breaking changes were submitted.)

During the first week of a month I will update [my crates](https://crates.io/users/drewcrawford) to target the new release.

In addition I will rapidly fire point releases as soon as possible.

## PR SLA

Obviously, try upstream first or in parallel.  If you're reading this, you might be somehow disillusioned on that path.

If you send me a PR that seems to pass CI and merge cleanly, I guarantee that I will merge it.  In case you don't want your actual feature to get broken later, consider writing better automated tests, which would benefit everybody.

For non-breaking changes, this will likely be the fastest review process you'll ever encounter.

For breaking changes, the merge window will be the last week of each month, when the monthly breakage has been scheduled. Review criteria will be similar, if it appears to pass CI I will try to merge it in that window.

On some roughly monthly cadence I will attempt to merge upstream's changes.  But they've [failed my CI tests](https://github.com/wasm-bindgen/wasm-bindgen/pull/4875#issuecomment-3675267028) for over a week now, so I guess we'll see how that goes.

---

# Original wasm-bindgen README

<div align="center">

  <h1><code>wasm-bindgen</code></h1>

  <p>
    <strong>Facilitating high-level interactions between Wasm modules and JavaScript.</strong>
  </p>

  <p>
    <a href="https://github.com/wasm-bindgen/wasm-bindgen/actions/workflows/main.yml?query=branch%3Amain"><img src="https://github.com/wasm-bindgen/wasm-bindgen/actions/workflows/main.yml/badge.svg?branch=main" alt="Build Status" /></a>
    <a href="https://crates.io/crates/wasm-bindgen"><img src="https://img.shields.io/crates/v/wasm-bindgen.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/wasm-bindgen"><img src="https://img.shields.io/crates/d/wasm-bindgen.svg?style=flat-square" alt="Download" /></a>
    <a href="https://docs.rs/wasm-bindgen"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>

  <h3>
    <a href="https://wasm-bindgen.github.io/wasm-bindgen/">Guide (main branch)</a>
    <span> | </span>
    <a href="https://docs.rs/wasm-bindgen">API Docs</a>
    <span> | </span>
    <a href="https://github.com/wasm-bindgen/wasm-bindgen/blob/master/CONTRIBUTING.md">Contributing</a>
    <span> | </span>
    <a href="https://discord.gg/xMZ7CCY">Chat</a>
  </h3>

  <sub>Built with 🦀🕸 by <a href="https://rustwasm.github.io/">The Rust and WebAssembly Working Group</a></sub>
</div>

## Install `wasm-bindgen-cli`

You can install it using `cargo install`:

```
cargo install wasm-bindgen-cli
```

Or, you can download it from the
[release page](https://github.com/wasm-bindgen/wasm-bindgen/releases).

If you have [`cargo-binstall`](https://crates.io/crates/cargo-binstall) installed,
then you can install the pre-built artifacts by running:

```
cargo binstall wasm-bindgen-cli
```

## Example

Import JavaScript things into Rust and export Rust things to JavaScript.

```rust
use wasm_bindgen::prelude::*;

// Import the `window.alert` function from the Web.
#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

// Export a `greet` function from Rust to JavaScript, that alerts a
// hello message.
#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}
```

Use exported Rust things from JavaScript with ECMAScript modules!

```js
import { greet } from "./hello_world";

greet("World!");
```

## Features

* **Lightweight.** Only pay for what you use. `wasm-bindgen` only generates
  bindings and glue for the JavaScript imports you actually use and Rust
  functionality that you export. For example, importing and using the
  `document.querySelector` method doesn't cause `Node.prototype.appendChild` or
  `window.alert` to be included in the bindings as well.

* **ECMAScript modules.** Just import WebAssembly modules the same way you would
  import JavaScript modules. Future compatible with [WebAssembly modules and
  ECMAScript modules integration][wasm-es-modules].

* **Designed with the ["Web IDL bindings" proposal][webidl-bindings] in mind.**
  Eventually, there won't be any JavaScript shims between Rust-generated wasm
  functions and native DOM methods. Because the Wasm functions are statically
  type checked, some of those native methods' dynamic type checks should become
  unnecessary, promising to unlock even-faster-than-JavaScript DOM access.

[wasm-es-modules]: https://github.com/WebAssembly/esm-integration
[webidl-bindings]: https://github.com/WebAssembly/proposals/issues/8

## Guide

[**📚 Read the `wasm-bindgen` guide here! 📚**](https://wasm-bindgen.github.io/wasm-bindgen/)

## API Docs

- [wasm-bindgen](https://docs.rs/wasm-bindgen)
- [js-sys](https://docs.rs/js-sys)
- [web-sys](https://docs.rs/web-sys)
- [wasm-bindgen-futures](https://docs.rs/wasm-bindgen-futures)

## MSRV Policy

* Libraries that are released on [crates.io](https://crates.io) have a MSRV of v1.71.
* CLI tools and their corresponding support libraries have a MSRV of v1.82.

The project aims to maintain a 2-year MSRV policy for libraries (meaning we support Rust versions released within the last 2 years), but with a shorter MSRV policy for the CLI. Changes to the MSRV may be made in patch versions, and will be logged in the CHANGELOG and MSRV history below.

### MSRV History

| Version | Library MSRV | CLI MSRV | Date       |
|---------|--------------|----------|------------|
| 0.2.106 | 1.71         | 1.82     | 2025-11-xx |
| 0.2.103 | 1.57         | 1.82     | 2025-09-17 |
| 0.2.93  | 1.57         | 1.76     | 2024-08-13 |

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

## Contribution

**[See the "Contributing" section of the guide for information on hacking on `wasm-bindgen`!][contributing]**

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.

[contributing]: https://wasm-bindgen.github.io/wasm-bindgen/contributing/index.html
