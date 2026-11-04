//! `file_too_long`: a Rust source file over 500 lines.

use std::env;
use std::path::{Path, PathBuf};

use clippy_utils::diagnostics::span_lint_and_then;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::{declare_lint, declare_lint_pass};
use rustc_span::{BytePos, FileName, Pos, SourceFile, Span};

declare_lint! {
    /// ### What it does
    ///
    /// Flags every Rust source file the crate compiles that has more than
    /// 500 physical lines. Blank lines and comments count. A file the crate
    /// does not own is not counted: one outside the crate's manifest
    /// directory, and one generated under `OUT_DIR`.
    ///
    /// ### Why is this bad?
    ///
    /// A reviewer reads a crate one file at a time. Past 500 lines a file
    /// holds more than one concern, and the split that separates them is
    /// cheapest while the file is still close to the limit.
    ///
    /// ### Example
    ///
    /// A `src/lib.rs` of 620 lines whose `#[cfg(test)] mod tests { .. }`
    /// takes 200 of them.
    ///
    /// Use instead:
    ///
    /// ```rust,ignore
    /// #[cfg(test)]
    /// mod tests;
    /// ```
    ///
    /// with the tests in `src/lib/tests.rs`.
    pub FILE_TOO_LONG,
    Deny,
    "a Rust source file has more than 500 lines"
}

declare_lint_pass!(FileTooLong => [FILE_TOO_LONG]);

/// The most physical lines a Rust file may have.
const LIMIT: usize = 500;

/// The split rule, one `help:` line each.
const HELP: [&str; 5] = [
    "split the file only into private child modules of that file",
    "open each child module with a `//!` line naming its one concern",
    "move an inline `#[cfg(test)]` module to its own file first, when one exists",
    "reach the parent's private items through `super::`, and give a moved item `pub(super)` \
     only where the parent uses it",
    "never widen anything to `pub(crate)` or `pub` to make a split compile",
];

/// The note the diagnostic ends with.
const STOP: &str = "if no cohesive group splits out without widening visibility, stop and say why";

impl<'tcx> LateLintPass<'tcx> for FileTooLong {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        let owner = Owner::from_env();
        let source_map = cx.sess().source_map();
        for file in source_map.files().iter() {
            if file.cnum != LOCAL_CRATE {
                continue;
            }
            let FileName::Real(name) = &file.name else {
                continue;
            };
            let Some(path) = name.local_path() else {
                continue;
            };
            if !owner.owns(path) {
                continue;
            }
            let Some(text) = &file.src else {
                continue;
            };
            let lines = text.lines().count();
            if lines > LIMIT {
                span_lint_and_then(
                    cx,
                    FILE_TOO_LONG,
                    first_line_over(file),
                    format!("this file has {lines} lines, over the limit of {LIMIT}"),
                    |diag| {
                        for help in HELP {
                            diag.help(help);
                        }
                        diag.note(STOP);
                    },
                );
            }
        }
    }
}

/// The span of the first line past the limit, without its line ending.
fn first_line_over(file: &SourceFile) -> Span {
    let start = file.line_bounds(LIMIT).start;
    let length = file
        .get_line(LIMIT)
        .map_or(0, |line| line.trim_end_matches(['\r', '\n']).len());
    let end = start + BytePos::from_usize(length);
    Span::with_root_ctxt(start, end)
}

/// Which files the crate being compiled owns.
struct Owner {
    /// Where relative source paths resolve from: rustc's working
    /// directory, the one cargo passed them relative to.
    cwd: Option<PathBuf>,
    /// The crate's manifest directory, when cargo set it.
    manifest_dir: Option<PathBuf>,
    /// The crate's build-script output, when it has a build script.
    out_dir: Option<PathBuf>,
}

impl Owner {
    fn from_env() -> Owner {
        Owner {
            cwd: env::current_dir().ok(),
            manifest_dir: env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from),
            out_dir: env::var_os("OUT_DIR").map(PathBuf::from),
        }
    }

    /// Whether `path` is a `.rs` file in the manifest directory and outside
    /// `OUT_DIR`. `include_str!` loads non-Rust files into the source map
    /// too; those are not counted.
    fn owns(&self, path: &Path) -> bool {
        if path.extension().is_none_or(|extension| extension != "rs") {
            return false;
        }
        let path = match &self.cwd {
            Some(cwd) if path.is_relative() => cwd.join(path),
            _ => path.to_path_buf(),
        };
        self.manifest_dir
            .as_ref()
            .is_none_or(|dir| path.starts_with(dir))
            && self
                .out_dir
                .as_ref()
                .is_none_or(|dir| !path.starts_with(dir))
    }
}
