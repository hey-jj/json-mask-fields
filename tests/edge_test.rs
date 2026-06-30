//! Edge cases the main suite leaves implicit.
//!
//! Each expected value was checked against a reference run. They pin behavior
//! the engine defines but the core table does not exercise directly.

use json_mask_fields::mask;
use serde_json::{json, Value};

struct Case {
    m: &'static str,
    o: Value,
    e: Value,
}

#[test]
fn edge_table() {
    let cases = vec![
        // A child mask on a scalar drops the key, since there is nothing to walk.
        Case {
            m: "a/b",
            o: json!({"a": 5}),
            e: json!({}),
        },
        Case {
            m: "a/b",
            o: json!({"a": "x"}),
            e: json!({}),
        },
        // Nested arrays keep their shape.
        Case {
            m: "a",
            o: json!({"a": [[{"b": 1, "c": 2}]]}),
            e: json!({"a": [[{"b": 1, "c": 2}]]}),
        },
        Case {
            m: "a(b(c))",
            o: json!({"a": [{"b": [{"c": 1, "d": 2}]}]}),
            e: json!({"a": [{"b": [{"c": 1}]}]}),
        },
        // A duplicate mask key takes the last definition.
        Case {
            m: "a(b),a(c)",
            o: json!({"a": {"b": 1, "c": 2}}),
            e: json!({"a": {"c": 2}}),
        },
        Case {
            m: "a,a(b)",
            o: json!({"a": {"b": 1, "c": 2}}),
            e: json!({"a": {"b": 1}}),
        },
        // Whitespace is part of the name, so " b" is a literal key.
        Case {
            m: "a, b",
            o: json!({"a": 1, " b": 2, "b": 3}),
            e: json!({"a": 1, " b": 2}),
        },
        // A whitespace-only mask is a literal key that matches nothing.
        Case {
            m: "   ",
            o: json!({"a": 1}),
            e: json!({}),
        },
        // A wildcard over an array keeps each element.
        Case {
            m: "a/*",
            o: json!({"a": [{"x": 1}, {"y": 2}]}),
            e: json!({"a": [{"x": 1}, {"y": 2}]}),
        },
        Case {
            m: "a(*)",
            o: json!({"a": [{"x": 1}, {"y": 2}]}),
            e: json!({"a": [{"x": 1}, {"y": 2}]}),
        },
        // A wildcard inside an array of arrays keeps the nested shape.
        Case {
            m: "a(*)",
            o: json!({"a": [[{"x": 1}]]}),
            e: json!({"a": [[{"x": 1}]]}),
        },
        Case {
            m: "a/*",
            o: json!({"a": [[{"x": 1}]]}),
            e: json!({"a": [[{"x": 1}]]}),
        },
        // A wildcard with a sub-path inside an array selects the leaf.
        Case {
            m: "a(*/z)",
            o: json!({"a": [{"g": {"z": 1, "q": 2}}]}),
            e: json!({"a": [{"g": {"z": 1}}]}),
        },
        // A top-level wildcard on an array keeps each element whole.
        Case {
            m: "*",
            o: json!([{"a": 1, "b": 2}]),
            e: json!([{"a": 1, "b": 2}]),
        },
        // A multi-byte key name matches as written.
        Case {
            m: "café",
            o: json!({"café": 1, "x": 2}),
            e: json!({"café": 1}),
        },
        // Empty containers selected as leaves are preserved.
        Case {
            m: "a",
            o: json!({"a": {}}),
            e: json!({"a": {}}),
        },
        Case {
            m: "a(b)",
            o: json!({"a": []}),
            e: json!({"a": []}),
        },
    ];

    for (i, c) in cases.iter().enumerate() {
        let got = mask(&c.o, c.m);
        assert_eq!(got, c.e, "edge case #{i} mask={:?}", c.m);
    }
}

#[test]
fn empty_and_null_mask_are_passthrough() {
    let o = json!({"a": 1, "b": {"c": 2}});
    assert_eq!(mask(&o, ""), o);
}

#[test]
fn wildcard_on_flat_object_is_identity() {
    let o = json!({"a": 1, "b": "x", "c": null, "d": 0});
    assert_eq!(mask(&o, "*"), o);
}

#[test]
fn mask_is_idempotent() {
    let o = json!({
        "url": 1,
        "obj": {"url": "h", "a": [{"url": 1, "z": 2}], "c": 3}
    });
    let once = mask(&o, "url,obj(url,a/url)");
    let twice = mask(&once, "url,obj(url,a/url)");
    assert_eq!(once, twice);
}
