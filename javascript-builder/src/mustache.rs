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

//! A tiny, pure-safe-Rust Mustache renderer scoped to the constructs the
//! bundled `JavaScriptResource.mustache` template actually uses.
//!
//! It is a deliberately small replacement for the `ramhorns` engine, which was
//! the only memory-unsafe dependency in the render path. Under concurrent test
//! execution its unsafe internals corrupted the process heap, surfacing on
//! Windows as an intermittent access violation (exit 0xC0000005) inside the
//! allocator. This renderer uses no `unsafe` and no third-party engine, so the
//! whole class of fault is gone.
//!
//! # Supported constructs
//!
//! The template only needs these tags, so only these are implemented:
//!
//! - `{{name}}` variables (rendered with no HTML escaping),
//! - `{{&name}}` and `{{{name}}}` unescaped variables (identical output to the
//!   plain form here, because escaping is off for every field),
//! - `{{#name}} ... {{/name}}` boolean sections, rendered when the flag is true,
//! - `{{^name}} ... {{/name}}` inverted sections, rendered when the flag is
//!   false.
//!
//! Lists, partials, lambdas, dotted paths and comments are not used by the
//! template and are not supported.
//!
//! # Output contract (byte-identical to the previous engine)
//!
//! - Every field is emitted verbatim, with no HTML escaping.
//! - Section tags themselves emit nothing, but all literal text around them
//!   (including indentation and line breaks) is preserved exactly. There is no
//!   Mustache "standalone tag" line stripping.
//! - The rendered output has its trailing whitespace trimmed once, at the end,
//!   matching the previous engine which trimmed the template tail.

use std::fmt::Display;

/// The values a render [`Context`] can supply for a field.
///
/// Variable tags read a [`Value`]; section tags read [`Context::flag`].
pub enum Value<'a> {
    /// A borrowed string value, emitted verbatim.
    Str(&'a str),
    /// An integer value, emitted as its decimal form (used for `_sequence`).
    Int(i64),
}

impl Value<'_> {
    fn write_to(&self, out: &mut String) {
        match self {
            Value::Str(s) => out.push_str(s),
            Value::Int(n) => {
                use std::fmt::Write;
                // Writing an integer into a String cannot fail.
                let _ = write!(out, "{n}");
            }
        }
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Self {
        Value::Str(s)
    }
}

/// The data a template is rendered against.
///
/// A variable tag `{{name}}` calls [`Context::value`]; a section tag
/// `{{#name}}` or `{{^name}}` calls [`Context::flag`]. Returning `None` from
/// either means the name is unknown, in which case a variable renders as empty
/// and a section is skipped, matching the previous engine.
pub trait Context {
    /// The value for a variable tag, or `None` if the name is unknown.
    fn value(&self, name: &str) -> Option<Value<'_>>;

    /// The boolean a section tag refers to, or `None` if the name is not a
    /// known boolean field.
    fn flag(&self, name: &str) -> Option<bool>;
}

/// A parsed node of the template.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    /// Literal text emitted verbatim.
    Text(String),
    /// A `{{name}}` / `{{&name}}` / `{{{name}}}` variable.
    Var(String),
    /// A `{{#name}} ... {{/name}}` section and its body.
    Section { name: String, body: Vec<Node> },
    /// A `{{^name}} ... {{/name}}` inverted section and its body.
    Inverted { name: String, body: Vec<Node> },
}

/// A parse error in the template source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// A `{{` was opened but never closed with `}}`.
    UnclosedTag,
    /// A `{{/name}}` closed a section that was not open, or closed the wrong
    /// section.
    MismatchedClose {
        /// The section the close tag named.
        found: String,
    },
    /// A section was opened with `{{#name}}` or `{{^name}}` but never closed.
    UnclosedSection {
        /// The section that was left open.
        name: String,
    },
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnclosedTag => write!(f, "unclosed tag: a {{{{ has no matching }}}}"),
            ParseError::MismatchedClose { found } => {
                write!(f, "mismatched closing tag {{{{/{found}}}}}")
            }
            ParseError::UnclosedSection { name } => {
                write!(f, "unclosed section {{{{#{name}}}}} (or inverted)")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// A parsed Mustache template, ready to render.
#[derive(Debug, Clone)]
pub struct Template {
    nodes: Vec<Node>,
}

impl Template {
    /// Parse the template source.
    ///
    /// Returns a [`ParseError`] if a tag is unclosed or a section close does not
    /// match the open. The bundled template is valid, so this does not fail in
    /// practice.
    pub fn parse(source: &str) -> Result<Self, ParseError> {
        let tokens = lex(source)?;
        let mut iter = tokens.into_iter().peekable();
        let nodes = parse_nodes(&mut iter, None)?;
        Ok(Template { nodes })
    }

    /// Render this template against `context`, returning the generated string
    /// with its trailing whitespace trimmed (matching the previous engine).
    pub fn render(&self, context: &dyn Context) -> String {
        // A starting capacity close to the previous engine's heuristic: the
        // source-ish size plus room for the substituted payloads. The exact
        // figure only affects reallocation, not the bytes produced.
        let mut out = String::with_capacity(self.estimated_capacity());
        render_nodes(&self.nodes, context, &mut out);
        // The previous engine trimmed the trailing whitespace of the final tail.
        let trimmed = out.trim_end();
        out.truncate(trimmed.len());
        out
    }

    /// A rough output-size estimate used only to pre-size the buffer.
    fn estimated_capacity(&self) -> usize {
        fn size(nodes: &[Node]) -> usize {
            nodes
                .iter()
                .map(|n| match n {
                    Node::Text(t) => t.len(),
                    Node::Var(_) => 16,
                    Node::Section { body, .. } | Node::Inverted { body, .. } => size(body),
                })
                .sum()
        }
        size(&self.nodes) + 256
    }
}

/// A lexer token: either literal text or a tag.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Text(String),
    /// A variable tag `{{name}}`, `{{&name}}` or `{{{name}}}`.
    Var(String),
    /// A section open `{{#name}}`.
    SectionOpen(String),
    /// An inverted-section open `{{^name}}`.
    InvertedOpen(String),
    /// A section close `{{/name}}`.
    Close(String),
}

/// Split the source into literal-text and tag tokens.
///
/// Triple-brace `{{{name}}}` is recognized before double-brace so the third
/// brace is consumed. The tag body is trimmed of surrounding ASCII whitespace,
/// matching how the previous engine tokenised names.
fn lex(source: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut text_start = 0;
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            // Flush any literal text accumulated before this tag.
            if i > text_start {
                tokens.push(Token::Text(source[text_start..i].to_owned()));
            }

            // Detect a triple brace so the closing third brace is consumed.
            let triple = i + 2 < bytes.len() && bytes[i + 2] == b'{';
            let open_len = if triple { 3 } else { 2 };
            let close_pat: &[u8] = if triple { b"}}}" } else { b"}}" };

            let body_start = i + open_len;
            let close_rel =
                find_subslice(&bytes[body_start..], close_pat).ok_or(ParseError::UnclosedTag)?;
            let body = &source[body_start..body_start + close_rel];
            let tag = classify_tag(body, triple);
            tokens.push(tag);

            i = body_start + close_rel + close_pat.len();
            text_start = i;
        } else {
            i += 1;
        }
    }

    if text_start < bytes.len() {
        tokens.push(Token::Text(source[text_start..].to_owned()));
    }

    Ok(tokens)
}

/// Turn a tag body (the text between the braces) into a token.
///
/// The first non-space character is the sigil: `#` section, `^` inverted, `/`
/// close, `&` unescaped variable. A triple-brace tag is always an unescaped
/// variable. Anything else is a plain variable. The remaining name is trimmed.
fn classify_tag(body: &str, triple: bool) -> Token {
    if triple {
        return Token::Var(body.trim().to_owned());
    }
    let trimmed = body.trim_start();
    let mut chars = trimmed.chars();
    match chars.next() {
        Some('#') => Token::SectionOpen(chars.as_str().trim().to_owned()),
        Some('^') => Token::InvertedOpen(chars.as_str().trim().to_owned()),
        Some('/') => Token::Close(chars.as_str().trim().to_owned()),
        Some('&') => Token::Var(chars.as_str().trim().to_owned()),
        _ => Token::Var(trimmed.trim().to_owned()),
    }
}

/// Find the first occurrence of `needle` within `haystack`, returning its start
/// offset.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Build the node tree from the token stream.
///
/// `open` is `Some(name)` while inside a section, so the matching close can be
/// validated. The function returns when it consumes the close for `open`, or at
/// end of input for the top level.
fn parse_nodes(
    tokens: &mut std::iter::Peekable<std::vec::IntoIter<Token>>,
    open: Option<&str>,
) -> Result<Vec<Node>, ParseError> {
    let mut nodes = Vec::new();

    while let Some(token) = tokens.next() {
        match token {
            Token::Text(text) => nodes.push(Node::Text(text)),
            Token::Var(name) => nodes.push(Node::Var(name)),
            Token::SectionOpen(name) => {
                let body = parse_nodes(tokens, Some(&name))?;
                nodes.push(Node::Section { name, body });
            }
            Token::InvertedOpen(name) => {
                let body = parse_nodes(tokens, Some(&name))?;
                nodes.push(Node::Inverted { name, body });
            }
            Token::Close(name) => {
                return match open {
                    Some(expected) if expected == name => Ok(nodes),
                    _ => Err(ParseError::MismatchedClose { found: name }),
                };
            }
        }
    }

    // End of input: every open section must have been closed.
    match open {
        Some(name) => Err(ParseError::UnclosedSection {
            name: name.to_owned(),
        }),
        None => Ok(nodes),
    }
}

/// Render a node list against the context into `out`.
fn render_nodes(nodes: &[Node], context: &dyn Context, out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(text),
            Node::Var(name) => {
                if let Some(value) = context.value(name) {
                    value.write_to(out);
                }
                // An unknown variable renders as empty, matching the previous
                // engine which wrote nothing for an unmatched field.
            }
            Node::Section { name, body } => {
                // A boolean section renders its body only when the flag is true.
                // An unknown flag skips the body.
                if context.flag(name) == Some(true) {
                    render_nodes(body, context, out);
                }
            }
            Node::Inverted { name, body } => {
                // An inverted section renders its body when the flag is false.
                // An unknown flag skips the body, matching the previous engine
                // (the template only inverts known boolean fields).
                if context.flag(name) == Some(false) {
                    render_nodes(body, context, out);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCtx;
    impl Context for TestCtx {
        fn value(&self, name: &str) -> Option<Value<'_>> {
            match name {
                "name" => Some(Value::Str("world")),
                "num" => Some(Value::Int(42)),
                _ => None,
            }
        }
        fn flag(&self, name: &str) -> Option<bool> {
            match name {
                "on" => Some(true),
                "off" => Some(false),
                _ => None,
            }
        }
    }

    fn render(src: &str) -> String {
        Template::parse(src).unwrap().render(&TestCtx)
    }

    #[test]
    fn plain_variable() {
        assert_eq!(render("hello {{name}}!"), "hello world!");
    }

    #[test]
    fn unescaped_variants_match_plain() {
        // No HTML escaping anywhere, so all three forms emit the raw value.
        assert_eq!(render("{{name}}|{{&name}}|{{{name}}}"), "world|world|world");
    }

    #[test]
    fn integer_value_is_bare() {
        assert_eq!(render("n={{num}};"), "n=42;");
    }

    #[test]
    fn unknown_variable_is_empty() {
        assert_eq!(render("[{{missing}}]"), "[]");
    }

    #[test]
    fn section_renders_when_true() {
        assert_eq!(render("{{#on}}yes{{/on}}"), "yes");
        assert_eq!(render("{{#off}}yes{{/off}}"), "");
    }

    #[test]
    fn inverted_renders_when_false() {
        assert_eq!(render("{{^off}}no{{/off}}"), "no");
        assert_eq!(render("{{^on}}no{{/on}}"), "");
    }

    #[test]
    fn nested_sections() {
        let src = "{{#on}}A{{^off}}B{{/off}}C{{/on}}";
        assert_eq!(render(src), "ABC");
    }

    #[test]
    fn literal_braces_without_pair_are_text() {
        // A single brace is literal text, not a tag.
        assert_eq!(render("a { b } c"), "a { b } c");
    }

    #[test]
    fn trailing_whitespace_is_trimmed() {
        assert_eq!(render("x{{name}}   \r\n\r\n"), "xworld");
    }

    #[test]
    fn interior_whitespace_is_preserved() {
        // Whitespace around a section tag stays; only the final tail is trimmed.
        let src = "a\n    {{#on}}\n    body\n    {{/on}}\n    z";
        assert_eq!(render(src), "a\n    \n    body\n    \n    z");
    }

    #[test]
    fn unclosed_tag_errors() {
        assert_eq!(
            Template::parse("a {{name").unwrap_err(),
            ParseError::UnclosedTag
        );
    }

    #[test]
    fn unclosed_section_errors() {
        assert!(matches!(
            Template::parse("{{#on}}body").unwrap_err(),
            ParseError::UnclosedSection { .. }
        ));
    }

    #[test]
    fn mismatched_close_errors() {
        assert!(matches!(
            Template::parse("{{#on}}body{{/off}}").unwrap_err(),
            ParseError::MismatchedClose { .. }
        ));
    }
}
