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

/// Prune `obj` against `compiled`.
///
/// A `None` mask is a passthrough and returns `obj` unchanged. An array input is
/// masked element by element with the structure preserved. This is the
/// `mask.filter` entry point and does not coerce a dropped result to null.
///
/// The engine can drop a value entirely, which has no JSON analogue. When that
/// happens at the top level, this returns [`Value::Null`]. The [`crate::mask`]
/// entry point relies on the richer [`filter_inner`] form to apply its own
/// coercion.
///
/// ```
/// use json_fieldmask::{compile, filter};
/// use serde_json::json;
/// let m = compile("a");
/// assert_eq!(filter(&json!({"a": 1, "b": 2}), &m), json!({"a": 1}));
/// ```
pub fn filter(obj: &Value, compiled: &Option<CompiledMask>) -> Value {
    filter_inner(obj, compiled).unwrap_or(Value::Null)
}

/// Prune `obj` and report a dropped result.
///
/// Returns `None` when the engine drops the value, which the caller maps as it
/// sees fit. Used by [`crate::mask`] so it can coerce a dropped or falsy result
/// to null the way the top-level entry point does.
pub(crate) fn filter_inner(obj: &Value, compiled: &Option<CompiledMask>) -> Option<Value> {
    if obj.is_array() {
        array_properties(obj, compiled)
    } else {
        properties(obj, compiled)
    }
}

/// Mask a top-level array.
///
/// Wraps the array under a synthetic key, masks it as an array-typed property,
/// then unwraps the result. Mirrors the object path so the element logic is
/// shared.
fn array_properties(arr: &Value, mask: &Option<CompiledMask>) -> Option<Value> {
    let mut wrapper = Map::new();
    wrapper.insert("_".to_string(), arr.clone());
    let wrapped = Value::Object(wrapper);

    let mut synthetic: CompiledMask = CompiledMask::new();
    synthetic.insert(
        "_".to_string(),
        Node {
            is_array: true,
            is_wildcard: false,
            properties: mask.clone(),
        },
    );

    let result = properties(&wrapped, &Some(synthetic))?;
    match result {
        Value::Object(mut map) => map.remove("_"),
        _ => None,
    }
}

/// Prune an object or scalar against a mask.
///
/// A null obj or a `None` mask returns the value unchanged. The result mirrors
/// the input container: an object yields an object, an array yields an array. A
/// scalar input with a real mask falls through and returns the scalar.
///
/// Returns `None` only when the caller should drop the key, which happens for a
/// missing input value. An object that matches nothing returns `Some({})`.
fn properties(obj: &Value, mask: &Option<CompiledMask>) -> Option<Value> {
    let mask = match mask {
        Some(m) => m,
        None => return Some(obj.clone()),
    };

    // A falsy value short-circuits and returns unchanged. This keeps null,
    // false, zero, and the empty string while a real mask is in play.
    if is_falsy(obj) {
        return Some(obj.clone());
    }

    // An array input yields an array. Only wildcard expansion lands as
    // elements. A named key looks up a string property on the array, which is
    // always absent, so named keys drop.
    if let Value::Array(items) = obj {
        let mut out = Vec::new();
        for (_, node) in mask {
            if node.is_wildcard {
                for (_, value) in for_all_array(items, &node.properties, node.is_array) {
                    out.push(value);
                }
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
            for (ret_key, ret_val) in for_all(obj, &node.properties, node.is_array) {
                masked.insert(ret_key, ret_val);
            }
        } else if let Some(ret) = apply(obj, key, node) {
            masked.insert(key.clone(), ret);
        }
    }

    Some(Value::Object(masked))
}

/// Apply a wildcard over every key of an object.
///
/// Runs the object or array logic per data key and keeps results that are not
/// dropped. Iterates in data order, which sets the order of the kept keys.
fn for_all(obj: &Value, mask: &Option<CompiledMask>, is_array: bool) -> Vec<(String, Value)> {
    let mut ret = Vec::new();
    if let Value::Object(map) = obj {
        for key in map.keys() {
            let value = if is_array {
                array(obj, key, mask)
            } else {
                object(obj, key, mask)
            };
            if let Some(value) = value {
                ret.push((key.clone(), value));
            }
        }
    }
    ret
}

/// Apply a wildcard over every index of an array.
///
/// Mirrors [`for_all`] for the array case, where keys are the index strings.
/// Wraps each element under its index so the object and array handlers can look
/// it up.
fn for_all_array(
    items: &[Value],
    mask: &Option<CompiledMask>,
    is_array: bool,
) -> Vec<(String, Value)> {
    let mut ret = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let key = i.to_string();
        let mut holder = Map::new();
        holder.insert(key.clone(), item.clone());
        let holder = Value::Object(holder);
        let value = if is_array {
            array(&holder, &key, mask)
        } else {
            object(&holder, &key, mask)
        };
        if let Some(value) = value {
            ret.push((key, value));
        }
    }
    ret
}

/// Dispatch a single non-wildcard key to the object or array handler.
fn apply(obj: &Value, key: &str, node: &Node) -> Option<Value> {
    if node.is_array {
        array(obj, key, &node.properties)
    } else {
        object(obj, key, &node.properties)
    }
}

/// Handle an object-typed key.
///
/// An array value is delegated to the array handler so a path through an array
/// masks each element. A present sub-mask recurses. A leaf selection returns the
/// value whole. A missing key returns `None`.
fn object(obj: &Value, key: &str, mask: &Option<CompiledMask>) -> Option<Value> {
    match get(obj, key) {
        Some(value) if value.is_array() => array(obj, key, mask),
        Some(value) => match mask {
            Some(_) => properties(value, mask),
            None => Some(value.clone()),
        },
        // A missing key has no value to recurse into. The key is dropped.
        None => None,
    }
}

/// Handle an array-typed key.
///
/// A non-array value falls back to object filtering. An empty array is returned
/// as-is. Otherwise each element is masked and kept when not dropped. When every
/// element drops, the whole key drops by returning `None`.
fn array(obj: &Value, key: &str, mask: &Option<CompiledMask>) -> Option<Value> {
    let arr = match get(obj, key) {
        Some(Value::Array(items)) => items,
        Some(value) => return properties(value, mask),
        // A missing key is dropped, the same as a missing object key.
        None => return None,
    };

    if arr.is_empty() {
        return Some(Value::Array(arr.clone()));
    }

    let mut ret = Vec::new();
    for item in arr {
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
