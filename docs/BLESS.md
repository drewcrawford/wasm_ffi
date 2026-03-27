# Blessing CLI Reference Fixtures

## Problem

`wasm-bindgen-cli` snapshot tests can fail in CI even after a local rebless.
The common symptom is a `.wat` diff that only swaps type indices or multivalue
shim signatures, for example:

- `__wbindgen_malloc` / `__wbindgen_realloc` type numbers swap
- `static_accessor_*` imports move from one type index to another
- multivalue shims such as `echo_i128`, `result_i32`, or `default__concat`
  point at different but equivalent type entries

This is easy to hit when local blesses are done on macOS and CI is running on
Linux.

## Why this happens

The native CI job uses a floating stable toolchain in
`.github/workflows/main.yml`:

- `test_native`
- `runs-on: ubuntu-latest`
- `dtolnay/rust-toolchain@stable`

That means two things can drift:

1. the exact stable Rust release
2. the host environment used to generate the `.wat` fixtures

In practice, the host matters too. A macOS bless with the same nominal Rust
version can still disagree with Ubuntu CI.

Concrete example from March 26, 2026:

- failing job: `Run native tests`
- run: `23623598927`
- job: `68807904901`
- host: `ubuntu-24.04`
- toolchain: `stable-x86_64-unknown-linux-gnu`
- rustc: `1.94.1 (e408947bf 2026-03-25)`

## Preferred local bless

If your local machine matches CI closely enough, use the repo shortcut:

```sh
just test-cli-overwrite
```

Equivalent command:

```sh
BLESS=1 cargo test -p wasm-bindgen-cli -- --skip headless_streaming_tests
```

Then validate in the same environment:

```sh
cargo test -p wasm-bindgen-cli
git diff --check
```

## Working procedure when CI still fails

When local bless passes but CI still fails, treat the failing Linux job as the
source of truth.

### 1. Find the failing native test job

From the Actions run page, look for:

- workflow: `CI`
- job: `Run native tests`

With `gh`:

```sh
gh run view <run-id> --repo drewcrawford/wasm_ffi
```

### 2. Extract the exact compiler from the job log

Do not assume `stable` means the same thing as your local machine.

```sh
gh api /repos/drewcrawford/wasm_ffi/actions/jobs/<job-id>/logs | \
  rg 'stable-x86_64-unknown-linux-gnu updated|rustc 1\.'
```

This should show the exact `rustc` line used by CI.

### 3. If you can run a matching Linux x86_64 environment, bless there

Use an environment that matches CI as closely as possible:

- `ubuntu-24.04`
- `x86_64`
- the exact Rust version from step 2

Then run:

```sh
BLESS=1 cargo test -p wasm-bindgen-cli -- --skip headless_streaming_tests
cargo test -p wasm-bindgen-cli
git diff --check
```

This is the cleanest fix.

### 4. If you cannot reproduce the CI host locally, use the job log diff

This was the reliable fallback when macOS `rustc 1.94.1` disagreed with Ubuntu
`rustc 1.94.1`.

Save the raw job log:

```sh
gh api /repos/drewcrawford/wasm_ffi/actions/jobs/<job-id>/logs > /tmp/job.log
```

Then inspect the failing sections:

```sh
rg -n 'failures:|---- reference::runtest::test_' /tmp/job.log
```

The snapshot diff is shown inline. Update the `.wat` fixture to match the
`right >` side of the diff for each failing test.

Typical failures live in:

- `crates/cli/tests/reference/async-number.wat`
- `crates/cli/tests/reference/async-void.wat`
- `crates/cli/tests/reference/echo.wat`
- `crates/cli/tests/reference/function-attrs.wat`
- `crates/cli/tests/reference/int128.wat`
- `crates/cli/tests/reference/js-namespace-export.wat`
- `crates/cli/tests/reference/result.wat`
- `crates/cli/tests/reference/web-sys.wat`

### 5. Validate what you can locally

Always run:

```sh
git diff --check
```

Be careful with local test interpretation:

- a macOS `cargo test -p wasm-bindgen-cli` run may fail after a Linux-specific
  bless
- that does not mean the bless is wrong
- the relevant validator is the matching Linux CI job

## Practical rule

If the failing diff is only type-index churn in `.wat` reference files, do not
debug the code generator first. Check the CI host and toolchain first.

## Long-term improvement

The underlying reason this keeps recurring is that CI snapshots are generated
with a floating stable toolchain on `ubuntu-latest`. Pinning the toolchain for
snapshot-producing jobs would reduce churn significantly.
