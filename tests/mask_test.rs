//! Targeted `mask` tests that the main table leaves implicit.
//!
//! These pin scalar top-level inputs, the all-elements-drop array path, the
//! null coercion split between `mask` and `filter`, serialized key order, and
//! the embedded escaped star.

use json_mask_fields::mask;
use serde_json::{json, Value};

#[test]
fn scalar_top_level_masks_to_null() {
    // A scalar has no keys. A truthy scalar drops, a falsy scalar passes the
    // filter guard, and mask coerces both to null at the top level.
    let cases: Vec<(Value, &str)> = vec![
        (json!(5), "a"),
        (json!("foo"), "a"),
        (json!(true), "a"),
        (json!(0), "a"),
        (json!(""), "a"),
        (json!(false), "a"),
    ];
    for (input, query) in cases {
        assert_eq!(mask(&input, query), Value::Null, "mask {input:?} {query:?}");
    }
}

#[test]
fn array_key_drops_when_every_element_vanishes() {
    // Truthy scalar elements under a sub-mask all drop, so the array key drops.
    assert_eq!(mask(&json!({"a": [1, 2, 3]}), "a(b)"), json!({}));
    // Falsy scalar elements pass the filter guard and are kept.
    assert_eq!(mask(&json!({"a": [0, 0]}), "a(b)"), json!({"a": [0, 0]}));
    // An object element survives as {} while the scalar drops.
    assert_eq!(
        mask(&json!({"a": [{"b": 1}, 5]}), "a(b)"),
        json!({"a": [{"b": 1}]})
    );
}

#[test]
fn mask_coerces_falsy_filter_result_while_filter_keeps_it() {
    // Same input, same empty mask, different result. Only mask runs the falsy
    // coercion. The filter side of this split lives in filter_test.rs.
    assert_eq!(mask(&json!(0), ""), Value::Null);
    assert_eq!(mask(&json!(false), ""), Value::Null);
    assert_eq!(mask(&json!(""), ""), Value::Null);
}

#[test]
fn serialized_output_follows_mask_order_then_data_order() {
    // Named keys come out in mask order. A value compare ignores order, so
    // assert the exact serialized string instead.
    let cases: Vec<(Value, &str, &str)> = vec![
        (json!({"a": 1, "b": 2}), "b,a", r#"{"b":2,"a":1}"#),
        // A wildcard follows data order, not sorted order.
        (
            json!({"z": 1, "a": 2, "m": 3}),
            "*",
            r#"{"z":1,"a":2,"m":3}"#,
        ),
        (json!({"a": 1, "b": 2, "c": 3}), "c,a", r#"{"c":3,"a":1}"#),
    ];
    for (input, query, expected) in cases {
        let out = serde_json::to_string(&mask(&input, query)).unwrap();
        assert_eq!(out, expected, "serialized mask {input:?} {query:?}");
    }
}

#[test]
fn embedded_escaped_star_keeps_its_backslash() {
    // Lone \* normalizes to a literal *. An escaped star inside a longer name
    // keeps its backslash, so the key is the four-character string a\*b.
    assert_eq!(
        mask(&json!({"a\\*b": 1, "a*b": 2}), "a\\*b"),
        json!({"a\\*b": 1})
    );
    assert_eq!(mask(&json!({"a*b": 2}), "a\\*b"), json!({}));
}
