//! Select parts of a JSON value with a fields query and drop the rest.
//!
//! This crate implements the Google APIs partial-response `fields` language over
//! [`serde_json::Value`]. You write a small query like
//! `url,object(content,attachments/url)` and get back a value of the same shape
//! with only the selected branches kept.
//!
//! The query keeps structure. It prunes branches you did not ask for and leaves
//! everything else in place. It does not flatten matches into a list the way a
//! path query would.
//!
//! # Query language
//!
//! - `a,b` selects keys `a` and `b`.
//! - `a/b` selects key `b` inside `a`.
//! - `a(b,c)` selects `b` and `c` inside each element of array `a`.
//! - `*` is a wildcard over every key of the current object.
//! - `\` escapes the next character, so `a\/b` selects the literal key `a/b`.
//!
//! # Example
//!
//! ```
//! use json_mask_fields::mask;
//! use serde_json::json;
//!
//! let input = json!({
//!     "url": "x",
//!     "id": "1",
//!     "obj": { "url": "h", "a": [{ "url": 1, "z": 2 }], "c": 3 }
//! });
//! let out = mask(&input, "url,obj(url,a/url)");
//! assert_eq!(out, json!({
//!     "url": "x",
//!     "obj": { "url": "h", "a": [{ "url": 1 }] }
//! }));
//! ```
//!
//! # Entry points
//!
//! - [`mask`] compiles the query and filters in one call. A dropped or falsy
//!   result becomes [`serde_json::Value::Null`].
//! - [`compile`] turns a query into a reusable [`CompiledMask`].
//! - [`filter`] applies a compiled mask to a value. It returns `Option<Value>`
//!   so a dropped key stays distinct from a kept null.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod compiler;
mod filter;

pub use compiler::{compile, CompiledMask, Node};
pub use filter::filter;

use serde_json::Value;

/// Compile `mask_str` and filter `obj` in one call.
///
/// An empty mask returns `obj` unchanged. When the result is dropped or falsy,
/// this returns [`Value::Null`]. Falsy means null, false, numeric zero, or the
/// empty string. An empty object or array is truthy and is returned as-is.
///
/// ```
/// use json_mask_fields::mask;
/// use serde_json::{json, Value};
///
/// assert_eq!(mask(&json!({"a": 1, "b": 2}), "a"), json!({"a": 1}));
/// assert_eq!(mask(&json!({"a": 1}), ""), json!({"a": 1}));
/// assert_eq!(mask(&Value::Null, "a"), Value::Null);
/// ```
#[must_use]
pub fn mask(obj: &Value, mask_str: &str) -> Value {
    let compiled = compile(mask_str);
    match filter::filter(obj, compiled.as_ref()) {
        Some(value) if !filter::is_falsy(&value) => value,
        _ => Value::Null,
    }
}
