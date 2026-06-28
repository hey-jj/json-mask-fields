//! Tests for `compile`, the query-to-tree pass.
//!
//! Each case compiles a query and compares the serialized tree to an expected
//! JSON literal. The serialized shape uses `type`, `isWildcard`, and
//! `properties` so it can be checked node for node.

use json_fieldmask::compile;
use serde_json::{json, Value};

fn tree(query: &str) -> Value {
    let compiled = compile(query).expect("query compiles");
    serde_json::to_value(&compiled).expect("tree serializes")
}

#[test]
fn compile_table() {
    let cases: Vec<(&str, Value)> = vec![
        ("a", json!({"a": {"type": "object"}})),
        (
            "a,b,c",
            json!({
                "a": {"type": "object"},
                "b": {"type": "object"},
                "c": {"type": "object"}
            }),
        ),
        (
            "a/*/c",
            json!({
                "a": {
                    "type": "object",
                    "properties": {
                        "*": {
                            "type": "object",
                            "isWildcard": true,
                            "properties": {"c": {"type": "object"}}
                        }
                    }
                }
            }),
        ),
        (
            "a,b(d/*/g,b),c",
            json!({
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
                                    "properties": {"g": {"type": "object"}}
                                }
                            }
                        },
                        "b": {"type": "object"}
                    }
                },
                "c": {"type": "object"}
            }),
        ),
        (
            "a(b/c,e)",
            json!({
                "a": {
                    "type": "array",
                    "properties": {
                        "b": {"type": "object", "properties": {"c": {"type": "object"}}},
                        "e": {"type": "object"}
                    }
                }
            }),
        ),
        (
            "a(b/c),e",
            json!({
                "a": {
                    "type": "array",
                    "properties": {
                        "b": {"type": "object", "properties": {"c": {"type": "object"}}}
                    }
                },
                "e": {"type": "object"}
            }),
        ),
        (
            "a(b/c/d),e",
            json!({
                "a": {
                    "type": "array",
                    "properties": {
                        "b": {
                            "type": "object",
                            "properties": {
                                "c": {"type": "object", "properties": {"d": {"type": "object"}}}
                            }
                        }
                    }
                },
                "e": {"type": "object"}
            }),
        ),
        (
            "a(b/g(c)),e",
            json!({
                "a": {
                    "type": "array",
                    "properties": {
                        "b": {
                            "type": "object",
                            "properties": {
                                "g": {
                                    "type": "array",
                                    "properties": {"c": {"type": "object"}}
                                }
                            }
                        }
                    }
                },
                "e": {"type": "object"}
            }),
        ),
        ("a\\/b\\/c", json!({"a/b/c": {"type": "object"}})),
        ("a\\(b\\)c", json!({"a(b)c": {"type": "object"}})),
        // an escaped b resolves to the literal b character
        ("a\\bc", json!({"abc": {"type": "object"}})),
        // an escaped star is a literal key, not a wildcard
        ("\\*", json!({"*": {"type": "object"}})),
        ("*", json!({"*": {"type": "object", "isWildcard": true}})),
        (
            "*(a,b,\\*,\\(,\\),\\,)",
            json!({
                "*": {
                    "type": "array",
                    "isWildcard": true,
                    "properties": {
                        "a": {"type": "object"},
                        "b": {"type": "object"},
                        "*": {"type": "object"},
                        "(": {"type": "object"},
                        ")": {"type": "object"},
                        ",": {"type": "object"}
                    }
                }
            }),
        ),
        ("\\\\", json!({"\\": {"type": "object"}})),
        ("foo*bar", json!({"foo*bar": {"type": "object"}})),
        // a trailing backslash is kept as a literal backslash
        ("foo\\", json!({"foo\\": {"type": "object"}})),
        // \n escapes the n character, which has no special meaning, so it is n
        ("\\n", json!({"n": {"type": "object"}})),
        // a real newline byte is an ordinary name character
        ("multi\nline", json!({"multi\nline": {"type": "object"}})),
    ];

    for (query, expected) in cases {
        assert_eq!(tree(query), expected, "compile {query:?}");
    }
}

#[test]
fn empty_query_compiles_to_none() {
    assert!(compile("").is_none());
}
