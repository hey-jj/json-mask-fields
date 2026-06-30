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

/// A scanned token: either a name or a structural terminal.
///
/// A name run holds the accumulated key text. An escaped wildcard is stored as
/// the two-char marker `\*` so the parser can tell it apart from a bare `*`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Name(String),
    Comma,
    Slash,
    Open,
    Close,
}

fn is_terminal(ch: char) -> bool {
    matches!(ch, ',' | '/' | '(' | ')')
}

/// Compile a fields query into a mask tree.
///
/// An empty string returns `None`, which the filter treats as a passthrough.
/// Malformed input does not error. The parser is lenient and returns whatever
/// the recursive walk yields.
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
    build_tree(&mut queue, &mut root_is_array, &mut root_has_child)
}

/// Split the query into tokens.
///
/// Backslash escapes the next character. A trailing backslash becomes a literal
/// backslash. An escaped wildcard keeps the two-char `\*` marker. Any other
/// escaped character drops the backslash. The four terminals `, / ( )` flush the
/// current name and emit a terminal token.
fn scan(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut tokens = Vec::new();
    let mut name = String::new();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        if ch == ESCAPE_CHAR {
            i += 1;
            if i >= len {
                name.push(ESCAPE_CHAR);
                break;
            }
            let next = chars[i];
            if next == WILDCARD_CHAR {
                name.push(ESCAPE_CHAR);
                name.push(WILDCARD_CHAR);
            } else {
                name.push(next);
            }
        } else if is_terminal(ch) {
            push_name(&mut tokens, &mut name);
            tokens.push(match ch {
                ',' => Token::Comma,
                '/' => Token::Slash,
                '(' => Token::Open,
                _ => Token::Close,
            });
        } else {
            name.push(ch);
        }
        i += 1;
    }
    push_name(&mut tokens, &mut name);

    tokens
}

fn push_name(tokens: &mut Vec<Token>, name: &mut String) {
    if name.is_empty() {
        return;
    }
    tokens.push(Token::Name(std::mem::take(name)));
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
) -> CompiledMask {
    let mut props: CompiledMask = IndexMap::new();

    while let Some(token) = tokens.pop_front() {
        match token {
            Token::Name(value) => {
                let mut child_is_array = false;
                let mut child_has_child = false;
                let properties = build_tree(tokens, &mut child_is_array, &mut child_has_child);
                let properties = if properties.is_empty() {
                    None
                } else {
                    Some(properties)
                };
                add_token(value, child_is_array, properties, &mut props);
                if *parent_has_child {
                    return props;
                }
            }
            Token::Comma => return props,
            Token::Open => *parent_is_array = true,
            Token::Close => return props,
            Token::Slash => *parent_has_child = true,
        }
    }

    props
}

/// Record a name into the props map.
///
/// A bare `*` becomes a wildcard node. The escaped marker `\*` rewrites to a
/// literal `*` key with no wildcard flag. Empty properties are dropped.
fn add_token(
    mut value: String,
    is_array: bool,
    properties: Option<CompiledMask>,
    props: &mut CompiledMask,
) {
    let mut is_wildcard = false;
    if value == "*" {
        is_wildcard = true;
    } else if value == "\\*" {
        value = "*".to_string();
    }

    props.insert(
        value,
        Node {
            is_array,
            is_wildcard,
            properties,
        },
    );
}
