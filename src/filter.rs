//! Prune a JSON value against a compiled mask.
//!
//! The engine keeps the shape of the input. It walks the mask, copies the
//! selected branches, and drops the rest. It never flattens or reshapes.
//!
//! JavaScript draws a line between `undefined` (drop the key) and `null` (keep
//! it). This module models that line with `Option<Value>`. `None` means drop.
//! `Some(Value::Null)` means keep an explicit null.

use serde_json::{Map, Value};

use crate::compiler::{CompiledMask, Node};

/// Prune `obj` against `compiled` and report a dropped result.
///
/// A `None` mask is a passthrough and returns `obj` unchanged. An array input is
/// masked element by element with the structure preserved. This is the
/// `mask.filter` entry point. It does not coerce a dropped or falsy result to
/// null. That coercion belongs to [`crate::mask`] alone.
///
/// The return type tells a dropped key apart from a kept null. `None` means the
/// engine dropped the value, which has no JSON analogue. `Some(Value::Null)`
/// means an explicit null was kept. A scalar under a real mask is dropped and
/// returns `None`, so `filter(&json!("x"), compile("a").as_ref())` is `None`,
/// while `filter(&json!(null), compile("a").as_ref())` is `Some(Value::Null)`.
///
/// ```
/// use json_mask_fields::{compile, filter};
/// use serde_json::json;
/// let m = compile("a");
/// assert_eq!(filter(&json!({"a": 1, "b": 2}), m.as_ref()), Some(json!({"a": 1})));
/// ```
#[must_use]
pub fn filter(obj: &Value, compiled: Option<&CompiledMask>) -> Option<Value> {
    if let Value::Array(items) = obj {
        mask_array(items, compiled)
    } else {
        properties(obj, compiled)
    }
}

/// Prune an object, array, or scalar against a mask.
///
/// A null obj or a `None` mask returns the value unchanged. The result mirrors
/// the input container: an object yields an object, an array yields an array. A
/// truthy scalar with a real mask has no keys to walk and drops.
///
/// Returns `None` only when the caller should drop the value, which happens for
/// a truthy scalar under a real mask. An object that matches nothing returns
/// `Some({})`.
fn properties(obj: &Value, mask: Option<&CompiledMask>) -> Option<Value> {
    let mask = match mask {
        Some(m) => m,
        None => return Some(obj.clone()),
    };

    // A falsy value short-circuits and returns unchanged. This keeps null,
    // false, zero, and the empty string while a real mask is in play.
    if is_falsy(obj) {
        return Some(obj.clone());
    }

    // An array input yields an array. Named nodes are skipped here. Only
    // wildcard nodes expand into array elements.
    if let Value::Array(items) = obj {
        let mut out = Vec::new();
        for node in mask.values() {
            if node.is_wildcard {
                out.extend(for_all_array(items, node));
            }
        }
        return Some(Value::Array(out));
    }

    // A truthy scalar has no keys to walk. The engine builds no container and
    // drops the value, which the caller reads as a missing key.
    if !obj.is_object() {
        return None;
    }

    let mut masked = Map::new();

    for (key, node) in mask {
        if node.is_wildcard {
            for (ret_key, ret_val) in for_all(obj, node) {
                masked.insert(ret_key, ret_val);
            }
        } else if let Some(value) = get(obj, key) {
            if let Some(ret) = select(value, node) {
                masked.insert(key.clone(), ret);
            }
        }
    }

    Some(Value::Object(masked))
}

/// Apply a wildcard over every key of an object.
///
/// Runs the per-value handler for each data key and keeps results that are not
/// dropped. Iterates in data order, which sets the order of the kept keys.
fn for_all(obj: &Value, node: &Node) -> Vec<(String, Value)> {
    let mut ret = Vec::new();
    if let Value::Object(map) = obj {
        for (key, value) in map {
            if let Some(masked) = select(value, node) {
                ret.push((key.clone(), masked));
            }
        }
    }
    ret
}

/// Apply a wildcard over every element of an array.
///
/// Runs the per-value handler for each element and keeps results that are not
/// dropped. Iterates in array order.
fn for_all_array(items: &[Value], node: &Node) -> Vec<Value> {
    let mut ret = Vec::new();
    for item in items {
        if let Some(masked) = select(item, node) {
            ret.push(masked);
        }
    }
    ret
}

/// Mask a single value against one node.
///
/// An array value is masked element by element. A non-array value under an
/// array node is masked as an object. A present sub-mask recurses. A leaf
/// selection returns the value whole.
fn select(value: &Value, node: &Node) -> Option<Value> {
    let mask = node.properties.as_ref();
    if let Value::Array(items) = value {
        return mask_array(items, mask);
    }
    if node.is_array {
        // An array node over a non-array value falls back to object masking.
        return properties(value, mask);
    }
    match mask {
        Some(_) => properties(value, mask),
        None => Some(value.clone()),
    }
}

/// Mask each element of an array and rebuild it.
///
/// An empty array is returned as-is. Otherwise each element is masked and kept
/// when not dropped. When every element drops, the whole array drops by
/// returning `None`.
fn mask_array(items: &[Value], mask: Option<&CompiledMask>) -> Option<Value> {
    if items.is_empty() {
        return Some(Value::Array(items.to_vec()));
    }

    let mut ret = Vec::new();
    for item in items {
        if let Some(masked) = properties(item, mask) {
            ret.push(masked);
        }
    }

    if ret.is_empty() {
        None
    } else {
        Some(Value::Array(ret))
    }
}

/// Look up an own key on an object value.
///
/// Returns `None` for a missing key or any non-object input.
fn get<'a>(obj: &'a Value, key: &str) -> Option<&'a Value> {
    match obj {
        Value::Object(map) => map.get(key),
        _ => None,
    }
}

/// Test JavaScript falsiness for a JSON value.
///
/// Null, false, numeric zero, and the empty string are falsy. Arrays and
/// objects are always truthy, even when empty.
pub(crate) fn is_falsy(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::Number(n) => n.as_f64() == Some(0.0),
        Value::String(s) => s.is_empty(),
        _ => false,
    }
}
