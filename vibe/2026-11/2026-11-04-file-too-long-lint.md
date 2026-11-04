---
name: file_too_long lint
overview: Replace the build-ceiling build-script check with file_too_long, a build-lints dylint lint that denies a Rust source file over 500 lines.
todos:
  - id: lint
    content: "file_too_long in lints/build-lints: a late pass that, in check_crate, counts the physical lines of every local .rs file in the source map under CARGO_MANIFEST_DIR and outside OUT_DIR, and denies one over 500 at its line 501"
    status: completed
  - id: ui-test
    content: "ui/file_too_long.rs (501 lines: blanks, comments, CRLF, unterminated last line) is flagged; ui/file_too_long_at_limit.rs (500 lines) is not"
    status: completed
  - id: remove
    content: "Delete crates/build-ceiling, the 33 build scripts that only ran it, every build-ceiling build-dependency, the build-xtask ceiling wiring check and its test, and the new-crate scaffolder's build.rs; drop the call from the desktop, server, and gateway build scripts"
    status: completed
isProject: false
---

# file_too_long lint

## Problem

The 500-line file limit was a build script in every workspace crate calling
`build_ceiling::check()`, a `build-ceiling` build-dependency (or a `#[path]`
include of its source in the crates that may name no `build-*` crate), and a
build-xtask check that every crate was wired. Three pieces of machinery for
one rule, and every new crate had to carry it.

## Chosen: a lint in build-lints

The lint sees exactly the files rustc compiles, so it needs no wiring and
no list of directories to walk, and a file no target compiles is not
counted. It covers build scripts, because `cargo dylint` lints them too.

- Owned files: `cnum == LOCAL_CRATE`, a real `.rs` path, under
  `CARGO_MANIFEST_DIR` and outside `OUT_DIR`. `include_str!` puts other
  files in the source map, and generated code is not the crate's to split.
- Counting: `str::lines` over the file's source, as `build-ceiling` did,
  so blank lines, comments, CRLF endings, and an unterminated last line
  all count.
- The diagnostic points at line 501 and carries the split rule as `help:`
  lines, ending with the stop note.
- The lint is emitted at crate level, so only a crate-level `allow` turns
  it off.

## Trade-off: cfg-gated files

A file compiled only under a `cfg` the CI `dylint` job's Linux target
does not set, such as `vfs/src/real/windows.rs` (47 lines) and
`gateway-discovery/src/sys/windows.rs` (79 lines), is no longer counted.
`build-ceiling` counted them on every machine.

## Verification

- `cargo test` in `lints/build-lints`: the UI test passes.
- A 501-line probe module and a 505-line probe `build.rs` in
  `shared-error-source` were each flagged by `cargo dylint`.
- `cargo dylint --all -- --locked --workspace --all-targets --all-features`:
  clean.
- `cargo nextest run -p build-xtask`: 246 passed. Both clippy gates with
  `CARGO_BUILD_WARNINGS=deny`, the docs gate, `cargo fmt --all --check`, and
  `docs-claims.mjs`: clean.
