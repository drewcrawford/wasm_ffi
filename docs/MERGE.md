# Merging `wasm-bindgen/main` into `wasm_ffi/main`

## Goal

Sync upstream `wasm-bindgen/main` into this fork while preserving fork-specific
behavior, tests, and release notes.

The default conflict policy is:

- prefer `wasm-bindgen/main`
- keep intentional `wasm_ffi` fork features
- do not lose `CHANGELOG_FFI.md`
- do not lose fork-only tests or test coverage

## Before you start

Do not do this work in a dirty checkout of your normal branch.

Use a separate worktree so you can:

- keep your main checkout untouched
- make large conflict resolution easier to review
- avoid overwriting unrelated local files

## Remote setup

This assumes:

- fork remote: `origin`
- upstream remote: `wasm-bindgen`

Fetch everything first:

```sh
git fetch --all --prune
```

If one remote is broken, do not stop blindly. Confirm that at least these refs
updated successfully:

```sh
git fetch origin main
git fetch wasm-bindgen main
```

## Recommended merge workflow

### 1. Start from the fork's main branch

Create an isolated worktree from `origin/main`:

```sh
git worktree add /tmp/wasm_ffi_merge origin/main -b merge-wasm-bindgen-main-YYYY-MM-DD
cd /tmp/wasm_ffi_merge
```

If you are using Codex-managed branches, prefer a `codex/` prefix.

### 2. Merge upstream with an upstream bias

Start the merge with a default preference for upstream:

```sh
git merge -X theirs wasm-bindgen/main
```

This is only the starting point. You still need to read every conflict and keep
fork behavior where it is intentional.

## Conflict resolution rules

### Default rule

When in doubt, keep the upstream `wasm-bindgen/main` version.

### Exceptions

Preserve fork-specific behavior in these categories:

- `CHANGELOG_FFI.md`
- fork-specific test runner behavior
- fork-specific doctest behavior
- worker log forwarding and custom worker handling
- `__wasm` export / duplicate-export fixes
- any test coverage added by the fork

### Files that are likely to need manual review

These were high-conflict files in the March 2026 merge and are likely to be
high-conflict again:

- `crates/cli-support/src/js/mod.rs`
- `crates/cli/src/wasm_bindgen_test_runner.rs`
- `crates/cli/src/wasm_bindgen_test_runner/server.rs`

Also review nearby test files whenever those files change.

## Post-merge verification

### 1. Check basic compilation and formatting

Run:

```sh
cargo test --workspace --lib --bins --tests --examples --no-run
cargo fmt --all --check
git diff --check
```

If a known upstream-only bench target fails outside wasm, note it separately
instead of folding it into merge fallout without evidence.

### 2. Verify fork-specific content was not lost

At minimum:

```sh
git diff origin/main -- CHANGELOG_FFI.md
git diff origin/main -- crates/cli/tests/wasm-bindgen-test-runner
git diff origin/main -- crates/test/tests
```

What to check:

- `CHANGELOG_FFI.md` is preserved
- no fork-only test files disappeared
- no fork-only test functions disappeared
- fork-specific runner/doctest paths are still present

If you want a stronger ancestry check:

```sh
git merge-base --is-ancestor origin/main HEAD
```

That confirms fork history was not dropped entirely, but it does not replace the
file-level checks above.

### 3. Rebless snapshots if needed

If reference fixtures fail, use:

```sh
just test-cli-overwrite
```

or:

```sh
BLESS=1 cargo test -p wasm-bindgen-cli -- --skip headless_streaming_tests
```

Then validate:

```sh
cargo test -p wasm-bindgen-cli
git diff --check
```

If CI still fails on `.wat` fixture churn after a local bless, follow
[`docs/BLESS.md`](/private/tmp/wasm_ffi_main_docs_2026_03_26/docs/BLESS.md).
That document explains how to extract the exact CI toolchain and, if necessary,
use the Linux CI job log as the source of truth.

## Commit strategy

Recommended flow:

1. create one merge commit with the conflict resolution
2. if rebless or typo follow-ups are required, fold them back into the merge
   commit before handing the branch off

That keeps review focused on one final merged state instead of a merge commit
plus several cleanup commits.

## Practical checklist

Before you consider the merge done, confirm all of the following:

- upstream `wasm-bindgen/main` is merged into a branch based on `origin/main`
- conflict resolution prefers upstream by default
- `CHANGELOG_FFI.md` is still present and correct
- fork-specific tests and featurework are still present
- merge fallout in known hot files was reviewed manually
- formatting and `git diff --check` pass
- CLI reference fixtures were reblessed if needed
- CI-specific bless drift was handled using `docs/BLESS.md` when necessary

## Long-term improvement

This workflow is harder than it should be because snapshot-producing CI jobs use
a floating stable compiler on `ubuntu-latest`. Pinning the toolchain for those
jobs would reduce post-merge fixture churn.
