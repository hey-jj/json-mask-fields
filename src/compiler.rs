//! Turn a fields query string into a compiled mask tree.
//!
//! The grammar:
//!
//! ```text
//!     Props ::= Prop | Prop "," Props
//!      Prop ::= Object | Array
//!    Object ::= NAME | NAME "/" Prop
//!     Array ::= NAME "(" Props ")"
//!      NAME ::= ? all visible characters except "\" ? | EscapeSeq | Wildcard
//!  Wildcard ::= "*"
//! EscapeSeq ::= "\" ? all visible characters ?
//! ```
//!
//! Compilation runs in two passes. [`scan`] splits the text into name and
//! terminal tokens. [`parse`] folds those tokens into a nested map. The map
//! preserves the order keys appear in the query, which decides output key order
//! at filter time.

use std::collections::VecDeque;

use indexmap::IndexMap;

/// A compiled mask: an ordered map from key to node.
///
/// Key order follows the order keys appear in the query string. The filter
/// walks this map in order, so output keys come out in the same order.
pub type CompiledMask = IndexMap<String, Node>;

/// One node in a compiled mask tree.
///
/// A node selects a key and describes how to treat its value. `is_array` marks
/// a key that was followed by `(...)`. `is_wildcard` marks the bare `*`
/// selector, which expands over every key of the data object. `properties`
/// holds the nested sub-mask and is absent when empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// True when the key was followed by `(...)`, marking an array selector.
    pub is_array: bool,

    /// True only for the bare `*` selector. An escaped `\*` is a literal key.
    pub is_wildcard: bool,

    /// Nested sub-mask. Absent when the node has no children.
    pub properties: Option<CompiledMask>,
}

const ESCAPE_CHAR: char = '\\';
const WILDCARD_CHAR: char = '*';

/// Maximum nesting depth the parser descends into.
///
/// Without a cap a deeply nested query overflows the stack and aborts the
/// process. At this depth the parser stops descending and ignores the deeper
/// nesting. The value matches the recursion limit serde_json uses when parsing.
const MAX_DEPTH: usize = 128;

/// A scanned token: either a name or a structural terminal.
///
/// A name run holds the key text and whether the whole token is a bare wildcard.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Name { value: String, is_wildcard: bool },
    Comma,
    Slash,
    Open,
    Close,
}

struct BuildResult {
    props: CompiledMask,
    dropped_at_cap: bool,
}

impl BuildResult {
    fn new(props: CompiledMask) -> Self {
        Self {
            props,
            dropped_at_cap: false,
        }
    }

    fn from_parts(props: CompiledMask, dropped_at_cap: bool) -> Self {
        Self {
            dropped_at_cap: dropped_at_cap && props.is_empty(),
            props,
        }
    }
}

fn is_terminal(ch: char) -> bool {
    matches!(ch, ',' | '/' | '(' | ')')
}

/// Compile a fields query into a mask tree.
///
/// An empty string returns `None`, which the filter treats as a passthrough.
/// Malformed input does not error. The parser is lenient and returns whatever
/// the recursive walk yields. Nesting deeper than [`MAX_DEPTH`] is ignored so a
/// crafted query cannot overflow the stack.
///
/// ```
/// use json_mask_fields::compile;
/// let tree = compile("a,b/c").unwrap();
/// assert!(tree.contains_key("a"));
/// assert!(tree.contains_key("b"));
/// ```
#[must_use]
pub fn compile(text: &str) -> Option<CompiledMask> {
    if text.is_empty() {
        return None;
    }
    Some(parse(scan(text)))
}

/// Parse a token vector into a mask tree.
fn parse(tokens: Vec<Token>) -> CompiledMask {
    let mut queue: VecDeque<Token> = tokens.into_iter().collect();
    let mut root_is_array = false;
    let mut root_has_child = false;
    build_tree(&mut queue, &mut root_is_array, &mut root_has_child, 0).props
}

/// Split the query into tokens.
///
/// Backslash escapes the next character. A trailing backslash becomes a literal
/// backslash. Any escaped character drops the backslash. The four terminals
/// `, / ( )` flush the current name and emit a terminal token.
fn scan(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut tokens = Vec::new();
    let mut name = String::new();
    let mut name_is_wildcard = false;
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        if ch == ESCAPE_CHAR {
            name_is_wildcard = false;
            i += 1;
            if i >= len {
                name.push(ESCAPE_CHAR);
                break;
            }
            let next = chars[i];
            name.push(next);
        } else if is_terminal(ch) {
            push_name(&mut tokens, &mut name, &mut name_is_wildcard);
            tokens.push(match ch {
                ',' => Token::Comma,
                '/' => Token::Slash,
                '(' => Token::Open,
                _ => Token::Close,
            });
        } else {
            name_is_wildcard = ch == WILDCARD_CHAR && name.is_empty();
            name.push(ch);
        }
        i += 1;
    }
    push_name(&mut tokens, &mut name, &mut name_is_wildcard);

    tokens
}

fn push_name(tokens: &mut Vec<Token>, name: &mut String, name_is_wildcard: &mut bool) {
    if name.is_empty() {
        *name_is_wildcard = false;
        return;
    }
    tokens.push(Token::Name {
        value: std::mem::take(name),
        is_wildcard: *name_is_wildcard,
    });
    *name_is_wildcard = false;
}

/// Fold tokens into a mask tree.
///
/// The token list is a shared queue consumed from the front. `parent_is_array`
/// and `parent_has_child` are the parent node's mutable flags. A `/` sets
/// `has_child` so the next name becomes the parent's single child. A `(` flips
/// the parent to array type. A `,` or `)` ends the current group.
fn build_tree(
    tokens: &mut VecDeque<Token>,
    parent_is_array: &mut bool,
    parent_has_child: &mut bool,
    depth: usize,
) -> BuildResult {
    let mut props: CompiledMask = IndexMap::new();
    let mut dropped_at_cap = false;

    while let Some(token) = tokens.pop_front() {
        match token {
            Token::Name { value, is_wildcard } => {
                if depth >= MAX_DEPTH {
                    let boundary = consume_selector_tail(tokens);
                    dropped_at_cap = true;
                    if *parent_has_child || matches!(boundary, Some(Token::Close)) {
                        return BuildResult {
                            props,
                            dropped_at_cap: true,
                        };
                    }
                    continue;
                }

                let mut child_is_array = false;
                let mut child_has_child = false;
                let child =
                    build_tree(tokens, &mut child_is_array, &mut child_has_child, depth + 1);
                if child.dropped_at_cap {
                    if *parent_has_child {
                        return BuildResult {
                            props,
                            dropped_at_cap: true,
                        };
                    }
                    dropped_at_cap = true;
                    continue;
                }

                let properties = if child.props.is_empty() {
                    None
                } else {
                    Some(child.props)
                };
                add_token(value, is_wildcard, child_is_array, properties, &mut props);
                if *parent_has_child {
                    return BuildResult::new(props);
                }
            }
            Token::Comma => return BuildResult::from_parts(props, dropped_at_cap),
            Token::Open => *parent_is_array = true,
            Token::Close => return BuildResult::from_parts(props, dropped_at_cap),
            Token::Slash => *parent_has_child = true,
        }
    }

    BuildResult::from_parts(props, dropped_at_cap)
}

fn consume_selector_tail(tokens: &mut VecDeque<Token>) -> Option<Token> {
    let mut group_depth = 0;

    while let Some(token) = tokens.pop_front() {
        match token {
            Token::Open => group_depth += 1,
            Token::Close if group_depth == 0 => return Some(Token::Close),
            Token::Close => group_depth -= 1,
            Token::Comma if group_depth == 0 => return Some(Token::Comma),
            _ => {}
        }
    }

    None
}

/// Record a name into the props map.
///
/// A bare `*` becomes a wildcard node. Escaped stars are literal key text.
/// Empty properties are dropped.
fn add_token(
    value: String,
    is_wildcard: bool,
    is_array: bool,
    properties: Option<CompiledMask>,
    props: &mut CompiledMask,
) {
    props.insert(
        value,
        Node {
            is_array,
            is_wildcard,
            properties,
        },
    );
}
