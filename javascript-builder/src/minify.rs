/* *********************************************************************
 * This Original Work is copyright of 51 Degrees Mobile Experts Limited.
 * Copyright 2026 51 Degrees Mobile Experts Limited, Davidson House,
 * Forbury Square, Reading, Berkshire, United Kingdom RG1 3EU.
 *
 * This Original Work is licensed under the European Union Public Licence
 * (EUPL) v.1.2 and is subject to its terms as set out below.
 *
 * If a copy of the EUPL was not distributed with this file, You can obtain
 * one at https://opensource.org/licenses/EUPL-1.2.
 *
 * The 'Compatible Licences' set out in the Appendix to the EUPL (as may be
 * amended by the European Commission) shall be deemed incompatible for
 * the purposes of the Work and the provisions of the compatibility
 * clause in Article 5 of the EUPL shall not apply.
 *
 * If using the Work as, or as part of, a network application, by
 * including the attribution notice(s) required under Article 5 of the EUPL
 * in the end user terms of the application under an appropriate heading,
 * such notice(s) shall fulfill the requirements of that article.
 * ********************************************************************* */

//! JavaScript minification with the oxc toolchain.
//!
//! With the default-on `minify` feature, the rendered template is parsed,
//! minified and printed by the pure-Rust oxc parser, minifier and code
//! generator. A failure at any stage (a parse error, or a panic from an edge
//! case in the minifier) falls back to the original, unminified content and
//! flags it through [`MinifyOutcome::had_error`], matching the .NET builder's
//! revert-on-error behaviour, so valid JavaScript is always served.
//!
//! The oxc call is wrapped in [`std::panic::catch_unwind`] as defence in depth.
//! The previously used `minify-js` crate could perform a memory-unsafe
//! out-of-bounds write that surfaced on Windows as a process access violation
//! (exit 0xc0000005), which is not a recoverable unwind and so could not be
//! contained. oxc is safe Rust, so any failure is at worst a catchable panic
//! that this guard turns into the unminified fallback.
//!
//! Without the `minify` feature, and on wasm targets (where the oxc dependency
//! is not compiled, see the crate's target-specific dependency), the rendered
//! template is returned unchanged. That output is correct, just not compacted.

/// The outcome of producing the JavaScript to store for a request.
pub struct MinifyOutcome {
    /// The content to store. This is the minified JavaScript on success, or the
    /// original rendered content when minification was not performed or failed.
    pub content: String,
    /// True when minification was attempted but failed and the original content
    /// was kept. False on success and when the `minify` feature is off.
    pub had_error: bool,
}

/// Return the JavaScript to store for this request.
///
/// With the `minify` feature the content is minified by oxc, falling back to the
/// original content (with `had_error` set) on any failure. Without the feature
/// the content is returned unchanged.
pub fn minify(content: String) -> MinifyOutcome {
    minify_impl(content)
}

#[cfg(all(feature = "minify", not(target_family = "wasm")))]
fn minify_impl(content: String) -> MinifyOutcome {
    match try_minify(&content) {
        Some(minified) => MinifyOutcome {
            content: minified,
            had_error: false,
        },
        None => MinifyOutcome {
            content,
            had_error: true,
        },
    }
}

#[cfg(not(all(feature = "minify", not(target_family = "wasm"))))]
fn minify_impl(content: String) -> MinifyOutcome {
    // No minifier is compiled in. Serving the rendered template unchanged is by
    // design, not a failure, so had_error stays false.
    MinifyOutcome {
        content,
        had_error: false,
    }
}

/// Minify `source` with oxc on a dedicated thread, returning `None` on any parse
/// error or panic so the caller serves the unminified content.
///
/// oxc parses and minifies by recursing over the AST. On a thread with a small
/// stack a deep script could overflow it, which surfaces on Windows as a process
/// access violation (exit 0xc0000005) rather than a catchable panic. The Windows
/// main thread is only 1 MiB, which is where the rustdoc doctest harness runs, so
/// the work is moved to a thread with a generous stack. Joining that thread also
/// contains an ordinary panic from an edge case (the join returns an error),
/// which becomes the unminified fallback.
#[cfg(all(feature = "minify", not(target_family = "wasm")))]
fn try_minify(source: &str) -> Option<String> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn_scoped(scope, || minify_with_oxc(source))
            .ok()
            .and_then(|handle| handle.join().ok())
            .flatten()
    })
}

/// Parse, minify and print `source` with oxc. Returns `None` if parsing reported
/// errors, so only valid JavaScript is compacted.
#[cfg(all(feature = "minify", not(target_family = "wasm")))]
fn minify_with_oxc(source: &str) -> Option<String> {
    use oxc::allocator::Allocator;
    use oxc::codegen::{Codegen, CodegenOptions};
    use oxc::minifier::{Minifier, MinifierOptions};
    use oxc::parser::Parser;
    use oxc::span::SourceType;

    let allocator = Allocator::default();
    // The generated client script is a classic script, not a module.
    let source_type = SourceType::cjs();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if parsed.panicked || !parsed.diagnostics.is_empty() {
        return None;
    }

    let mut program = parsed.program;
    let minified = Minifier::new(MinifierOptions::default()).minify(&allocator, &mut program);

    let printed = Codegen::new()
        .with_options(CodegenOptions {
            minify: true,
            ..CodegenOptions::default()
        })
        .with_scoping(minified.scoping)
        .build(&program);
    Some(printed.code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "minify", not(target_family = "wasm")))]
    #[test]
    fn minify_compacts_valid_javascript() {
        let content = "function   add ( a , b )  {\n  return a   +   b ;\n}\n".to_owned();
        let outcome = minify(content.clone());
        assert!(!outcome.had_error, "valid input minifies without error");
        assert!(
            outcome.content.len() < content.len(),
            "minified output is smaller: {:?}",
            outcome.content
        );
        assert!(
            !outcome.content.contains("  "),
            "insignificant whitespace is removed: {:?}",
            outcome.content
        );
    }

    #[cfg(all(feature = "minify", not(target_family = "wasm")))]
    #[test]
    fn minify_falls_back_on_invalid_javascript() {
        // A parse error must not propagate; the original content is served and
        // the error is flagged.
        let content = "function (  {{ this is not valid ".to_owned();
        let outcome = minify(content.clone());
        assert!(outcome.had_error, "invalid input is flagged");
        assert_eq!(outcome.content, content, "the original content is kept");
    }

    #[cfg(not(all(feature = "minify", not(target_family = "wasm"))))]
    #[test]
    fn minify_returns_content_unchanged_without_the_feature() {
        let content = "var  x  =  1 ;".to_owned();
        let outcome = minify(content.clone());
        assert!(!outcome.had_error);
        assert_eq!(outcome.content, content);
    }
}
