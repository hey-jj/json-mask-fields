//! End-to-end tests for the public `mask` entry point.
//!
//! Each case is a mask string, an input value, and the expected output. The
//! table mirrors the canonical conformance suite for the fields language.

use json_fieldmask::mask;
use serde_json::{json, Value};

struct Case {
    m: &'static str,
    o: Value,
    e: Value,
}

fn activities() -> Value {
    let raw = include_str!("fixtures/activities.json");
    serde_json::from_str(raw).expect("fixture parses")
}

#[test]
fn mask_table() {
    let cases = vec![
        // 0: filter(null) yields null, then the top level keeps it null
        Case {
            m: "a",
            o: Value::Null,
            e: Value::Null,
        },
        // 1: missing key is dropped, not nulled
        Case {
            m: "a",
            o: json!({"b": 1}),
            e: json!({}),
        },
        // 2: an explicit null value is kept
        Case {
            m: "a",
            o: json!({"a": null, "b": 1}),
            e: json!({"a": null}),
        },
        // 3: a top-level array is masked element by element
        Case {
            m: "a",
            o: json!([{"b": 1}]),
            e: json!([{}]),
        },
        // 4 and 5: an empty mask is a passthrough. A null or undefined mask
        // collapses to the same empty-string-to-None path in this crate, since
        // there is no separate null mask value. The compile-to-None passthrough
        // chain is asserted directly in filter_test.rs.
        Case {
            m: "",
            o: json!({"a": 1}),
            e: json!({"a": 1}),
        },
        Case {
            m: "",
            o: json!({"a": 1}),
            e: json!({"a": 1}),
        },
        // 6: basic single key select
        Case {
            m: "a",
            o: json!({"a": 1, "b": 1}),
            e: json!({"a": 1}),
        },
        // 7: an empty string value is kept
        Case {
            m: "notEmptyStr",
            o: json!({"notEmptyStr": ""}),
            e: json!({"notEmptyStr": ""}),
        },
        // 8: a zero value is kept
        Case {
            m: "notEmptyNum",
            o: json!({"notEmptyNum": 0}),
            e: json!({"notEmptyNum": 0}),
        },
        // 9: comma-separated list
        Case {
            m: "a,b",
            o: json!({"a": 1, "b": 1, "c": 1}),
            e: json!({"a": 1, "b": 1}),
        },
        // 10: nested path
        Case {
            m: "obj/s",
            o: json!({"obj": {"s": 1, "t": 2}, "b": 1}),
            e: json!({"obj": {"s": 1}}),
        },
        // 11: a path masks each element of an array
        Case {
            m: "arr/s",
            o: json!({"arr": [{"s": 1, "t": 2}, {"s": 2, "t": 3}], "b": 1}),
            e: json!({"arr": [{"s": 1}, {"s": 2}]}),
        },
        // 12: deep path plus a sibling
        Case {
            m: "a/s/g,b",
            o: json!({"a": {"s": {"g": 1, "z": 1}}, "t": 2, "b": 1}),
            e: json!({"a": {"s": {"g": 1}}, "b": 1}),
        },
        // 13: the wildcard keeps every key, including null and zero
        Case {
            m: "*",
            o: json!({"a": 2, "b": null, "c": 0, "d": 3}),
            e: json!({"a": 2, "b": null, "c": 0, "d": 3}),
        },
        // 14: path then wildcard then path
        Case {
            m: "a/*/g",
            o: json!({"a": {"s": {"g": 3}, "t": {"g": 4}, "u": {"z": 1}}, "b": 1}),
            e: json!({"a": {"s": {"g": 3}, "t": {"g": 4}, "u": {}}}),
        },
        // 15: a wildcard leaf keeps each subtree whole
        Case {
            m: "a/*",
            o: json!({"a": {"s": {"g": 3}, "t": {"g": 4}, "u": {"z": 1}}, "b": 3}),
            e: json!({"a": {"s": {"g": 3}, "t": {"g": 4}, "u": {"z": 1}}}),
        },
        // 16: array sub-selection
        Case {
            m: "a(g)",
            o: json!({"a": [{"g": 1, "d": 2}, {"g": 2, "d": 3}]}),
            e: json!({"a": [{"g": 1}, {"g": 2}]}),
        },
        // 17: an empty array and an empty object are preserved
        Case {
            m: "a,c",
            o: json!({"a": [], "c": {}}),
            e: json!({"a": [], "c": {}}),
        },
        // 18: a wildcard nested inside an array sub-selection
        Case {
            m: "b(d/*/z)",
            o: json!({"b": [{"d": {"g": {"z": 22}, "b": 34}}]}),
            e: json!({"b": [{"d": {"g": {"z": 22}}}]}),
        },
        // 19: mixed select with a nested array path
        Case {
            m: "url,obj(url,a/url)",
            o: json!({"url": 1, "id": "1", "obj": {"url": "h", "a": [{"url": 1, "z": 2}], "c": 3}}),
            e: json!({"url": 1, "obj": {"url": "h", "a": [{"url": 1}]}}),
        },
        // 20: wildcard with a sub-selection
        Case {
            m: "*(a,b)",
            o: json!({"p1": {"a": 1, "b": 1, "c": 1}, "p2": {"a": 2, "b": 2, "c": 2}}),
            e: json!({"p1": {"a": 1, "b": 1}, "p2": {"a": 2, "b": 2}}),
        },
        // 21: fixture, single top key
        Case {
            m: "kind",
            o: activities(),
            e: json!({"kind": "plus#activity"}),
        },
        // 22: fixture, nested object sub-selection
        Case {
            m: "object(objectType)",
            o: activities(),
            e: json!({"object": {"objectType": "note"}}),
        },
        // 23: fixture, path into an array
        Case {
            m: "url,object(content,attachments/url)",
            o: activities(),
            e: json!({
                "url": "https://plus.google.com/102817283354809142195/posts/F97fqZwJESL",
                "object": {
                    "content": "Congratulations! You have successfully fetched an explicit public activity. The attached video is your reward. :)",
                    "attachments": [{"url": "http://www.youtube.com/watch?v=dQw4w9WgXcQ"}]
                }
            }),
        },
        // 24: top-level array key select
        Case {
            m: "i",
            o: json!([{"i": 1, "o": 2}, {"i": 2, "o": 2}]),
            e: json!([{"i": 1}, {"i": 2}]),
        },
        // 25: a sub-key that matches nothing leaves an empty object
        Case {
            m: "foo(bar)",
            o: json!({"foo": {"biz": "bar"}}),
            e: json!({"foo": {}}),
        },
        // 26: same, different value
        Case {
            m: "foo(bar)",
            o: json!({"foo": {"biz": "baz"}}),
            e: json!({"foo": {}}),
        },
        // 27: a JS undefined value has no JSON form, so the key is simply absent
        Case {
            m: "foobar,foobiz",
            o: json!({"foobar": {"foo": "bar"}}),
            e: json!({"foobar": {"foo": "bar"}}),
        },
        // 28: a missing key on a plain object yields the empty object
        Case {
            m: "foobar",
            o: json!({"foo": "bar"}),
            e: json!({}),
        },
        // 29: a missing key on a top-level array yields an array of empties
        Case {
            m: "foobar",
            o: json!([{"biz": "baz"}]),
            e: json!([{}]),
        },
        // 30: an array leaf is kept whole, including falsy elements
        Case {
            m: "a",
            o: json!({"a": [0, 0]}),
            e: json!({"a": [0, 0]}),
        },
        // 31: same with a longer array
        Case {
            m: "a",
            o: json!({"a": [1, 0, 1]}),
            e: json!({"a": [1, 0, 1]}),
        },
        // 32: only own keys are considered (a class instance with own a,b)
        Case {
            m: "a/b",
            o: json!({"a": {"a": 3, "b": 4}}),
            e: json!({"a": {"b": 4}}),
        },
        // 33: partial array elements, one matches and one becomes empty
        Case {
            m: "a(b/c),e",
            o: json!({"a": [{"b": {"c": 1}}, {"d": 2}], "e": 3, "f": 4, "g": 5}),
            e: json!({"a": [{"b": {"c": 1}}, {}], "e": 3}),
        },
        // 34: deeper version of the same
        Case {
            m: "a(b/c/d),e",
            o: json!({"a": [{"b": {"c": {"d": 1}}}, {"d": 2}], "e": 3, "f": 4, "g": 5}),
            e: json!({"a": [{"b": {"c": {"d": 1}}}, {}], "e": 3}),
        },
        // 35: two array sub-selections side by side
        Case {
            m: "beta(first,second/third),cappa(first,second/third)",
            o: json!({
                "alpha": 3,
                "beta": {"first": "fv", "second": {"third": "tv", "fourth": "fv"}},
                "cappa": {"first": "fv", "second": {"third": "tv", "fourth": "fv"}}
            }),
            e: json!({
                "beta": {"first": "fv", "second": {"third": "tv"}},
                "cappa": {"first": "fv", "second": {"third": "tv"}}
            }),
        },
        // 36: an escaped slash makes a literal key
        Case {
            m: "a\\/b",
            o: json!({"a/b": 1, "c": 2}),
            e: json!({"a/b": 1}),
        },
        // 37: an escaped slash inside a sub-selection
        Case {
            m: "beta(first,second\\/third),cappa(first,second\\/third)",
            o: json!({
                "alpha": 3,
                "beta": {"first": "fv", "second/third": "tv", "third": {"fourth": "fv"}},
                "cappa": {"first": "fv", "second/third": "tv", "third": {"fourth": "fv"}}
            }),
            e: json!({
                "beta": {"first": "fv", "second/third": "tv"},
                "cappa": {"first": "fv", "second/third": "tv"}
            }),
        },
        // 38: an escaped star is a literal key, not a wildcard
        Case {
            m: "\\*",
            o: json!({"*": 101, "beta": "hidden"}),
            e: json!({"*": 101}),
        },
        // 39: escaped star nested in a sub-selection
        Case {
            m: "first(\\*)",
            o: json!({"first": {"*": 101, "beta": "hidden"}}),
            e: json!({"first": {"*": 101}}),
        },
        // 40: escaped star alongside a normal key
        Case {
            m: "some,\\*",
            o: json!({"*": 101, "beta": "hidden", "some": "visible"}),
            e: json!({"*": 101, "some": "visible"}),
        },
        // 41: an escaped backslash selects a literal backslash key
        Case {
            m: "some,\\\\",
            o: json!({"\\": 120, "beta": "hidden", "some": "visible"}),
            e: json!({"\\": 120, "some": "visible"}),
        },
        // 42: a real newline byte is an ordinary name character
        Case {
            m: "multi\nline(a)",
            o: json!({"multi": 130, "line": 131, "multi\nline": {"a": 135, "b": 134}}),
            e: json!({"multi\nline": {"a": 135}}),
        },
        // 43: a star at the end of a name is literal
        Case {
            m: "a*",
            o: json!({"a*": 1, "b": 2}),
            e: json!({"a*": 1}),
        },
        // 44: a star at the start of a name is literal
        Case {
            m: "*a",
            o: json!({"*a": 1, "b": 2}),
            e: json!({"*a": 1}),
        },
    ];

    for (i, c) in cases.iter().enumerate() {
        let got = mask(&c.o, c.m);
        assert_eq!(got, c.e, "case #{i} mask={:?}", c.m);
    }
}
