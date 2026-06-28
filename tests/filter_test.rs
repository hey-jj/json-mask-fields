//! Tests for `filter` with a hand-built mask tree.
//!
//! This case bypasses `compile` and feeds the engine a tree directly. It keeps
//! an engine regression separate from a compiler regression.

use json_fieldmask::{filter, CompiledMask};
use serde_json::json;

#[test]
fn filter_with_hand_built_mask() {
    // The compiled form of a,b(d/*/z,b(g)),c,d/e,*
    let mask_value = json!({
        "a": {"type": "object"},
        "b": {
            "type": "array",
            "properties": {
                "d": {
                    "type": "object",
                    "properties": {
                        "*": {
                            "type": "object",
                            "isWildcard": true,
                            "properties": {"z": {"type": "object"}}
                        }
                    }
                },
                "b": {
                    "type": "array",
                    "properties": {"g": {"type": "object"}}
                }
            }
        },
        "c": {"type": "object"},
        "d/e": {"type": "object"},
        "*": {"type": "object"}
    });
    let compiled: CompiledMask = serde_json::from_value(mask_value).expect("tree deserializes");

    let object = json!({
        "a": 11,
        "n": 0,
        "b": [{
            "d": {"g": {"z": 22}, "b": 34, "c": {"a": 32}},
            "b": [{"z": 33}],
            "k": 99
        }],
        "c": 44,
        "g": 99,
        "d/e": 101,
        "*": 110
    });

    let expected = json!({
        "a": 11,
        "b": [{
            "d": {
                "g": {"z": 22},
                "c": {}
            },
            "b": [{}]
        }],
        "c": 44,
        "d/e": 101,
        "*": 110
    });

    assert_eq!(filter(&object, &Some(compiled)), expected);
}
