//! Tests for `compile`, the query-to-tree pass.
//!
//! Each case compiles a query and compares the result to a hand-built tree.
//! Trees are assembled with the `obj`, `arr`, and `wild` helpers so the shape
//! reads close to the query it stands for.

use json_fieldmask::{compile, CompiledMask, Node};

/// Build a `CompiledMask` from key and node pairs in order.
fn mask<I>(entries: I) -> CompiledMask
where
    I: IntoIterator<Item = (&'static str, Node)>,
{
    entries
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

/// An object-typed node with an optional sub-mask.
fn obj(properties: Option<CompiledMask>) -> Node {
    Node {
        is_array: false,
        is_wildcard: false,
        properties,
    }
}

/// An array-typed node with an optional sub-mask.
fn arr(properties: Option<CompiledMask>) -> Node {
    Node {
        is_array: true,
        is_wildcard: false,
        properties,
    }
}

/// A wildcard node carrying the given type and optional sub-mask.
fn wild(is_array: bool, properties: Option<CompiledMask>) -> Node {
    Node {
        is_array,
        is_wildcard: true,
        properties,
    }
}

#[test]
fn compile_table() {
    let cases: Vec<(&str, CompiledMask)> = vec![
        ("a", mask([("a", obj(None))])),
        (
            "a,b,c",
            mask([("a", obj(None)), ("b", obj(None)), ("c", obj(None))]),
        ),
        (
            "a/*/c",
            mask([(
                "a",
                obj(Some(mask([(
                    "*",
                    wild(false, Some(mask([("c", obj(None))]))),
                )]))),
            )]),
        ),
        (
            "a,b(d/*/g,b),c",
            mask([
                ("a", obj(None)),
                (
                    "b",
                    arr(Some(mask([
                        (
                            "d",
                            obj(Some(mask([(
                                "*",
                                wild(false, Some(mask([("g", obj(None))]))),
                            )]))),
                        ),
                        ("b", obj(None)),
                    ]))),
                ),
                ("c", obj(None)),
            ]),
        ),
        (
            "a(b/c,e)",
            mask([(
                "a",
                arr(Some(mask([
                    ("b", obj(Some(mask([("c", obj(None))])))),
                    ("e", obj(None)),
                ]))),
            )]),
        ),
        (
            "a(b/c),e",
            mask([
                (
                    "a",
                    arr(Some(mask([("b", obj(Some(mask([("c", obj(None))]))))]))),
                ),
                ("e", obj(None)),
            ]),
        ),
        (
            "a(b/c/d),e",
            mask([
                (
                    "a",
                    arr(Some(mask([(
                        "b",
                        obj(Some(mask([("c", obj(Some(mask([("d", obj(None))]))))]))),
                    )]))),
                ),
                ("e", obj(None)),
            ]),
        ),
        (
            "a(b/g(c)),e",
            mask([
                (
                    "a",
                    arr(Some(mask([(
                        "b",
                        obj(Some(mask([("g", arr(Some(mask([("c", obj(None))]))))]))),
                    )]))),
                ),
                ("e", obj(None)),
            ]),
        ),
        ("a\\/b\\/c", mask([("a/b/c", obj(None))])),
        ("a\\(b\\)c", mask([("a(b)c", obj(None))])),
        // an escaped b resolves to the literal b character
        ("a\\bc", mask([("abc", obj(None))])),
        // an escaped star is a literal key, not a wildcard
        ("\\*", mask([("*", obj(None))])),
        ("*", mask([("*", wild(false, None))])),
        (
            "*(a,b,\\*,\\(,\\),\\,)",
            mask([(
                "*",
                wild(
                    true,
                    Some(mask([
                        ("a", obj(None)),
                        ("b", obj(None)),
                        ("*", obj(None)),
                        ("(", obj(None)),
                        (")", obj(None)),
                        (",", obj(None)),
                    ])),
                ),
            )]),
        ),
        ("\\\\", mask([("\\", obj(None))])),
        ("foo*bar", mask([("foo*bar", obj(None))])),
        // a trailing backslash is kept as a literal backslash
        ("foo\\", mask([("foo\\", obj(None))])),
        // \n escapes the n character, which has no special meaning, so it is n
        ("\\n", mask([("n", obj(None))])),
        // a real newline byte is an ordinary name character
        ("multi\nline", mask([("multi\nline", obj(None))])),
        // an escaped star inside a longer name keeps its backslash
        ("a\\*b", mask([("a\\*b", obj(None))])),
    ];

    for (query, expected) in cases {
        assert_eq!(compile(query), Some(expected), "compile {query:?}");
    }
}

#[test]
fn compile_lenient_on_malformed_queries() {
    // The parser never errors. An unbalanced or stray terminal yields whatever
    // the recursive walk produces. These pin that lenient behavior so a later
    // validation pass cannot change it unnoticed.
    let cases: Vec<(&str, CompiledMask)> = vec![
        // An unbalanced "(" still flips the parent to array and keeps parsing,
        // so "a(b" compiles the same as "a(b)".
        ("a(b", mask([("a", arr(Some(mask([("b", obj(None))]))))])),
        // A stray ")" ends the current group, so "a)b" splits into siblings.
        ("a)b", mask([("a", obj(None)), ("b", obj(None))])),
        ("(a)", mask([("a", obj(None))])),
        ("a((b))", mask([("a", arr(Some(mask([("b", obj(None))]))))])),
        // A doubled comma closes the group early, so only "a" survives.
        ("a,,b", mask([("a", obj(None))])),
        ("/a", mask([("a", obj(None))])),
    ];

    for (query, expected) in cases {
        assert_eq!(compile(query), Some(expected), "compile {query:?}");
    }
}

#[test]
fn compile_empty_table_cases() {
    // ")(" and ",,," parse to an empty tree but are not the empty string, so
    // they compile to Some(empty), not None.
    assert_eq!(compile(")("), Some(CompiledMask::new()));
    assert_eq!(compile(",,,"), Some(CompiledMask::new()));
}

#[test]
fn empty_query_compiles_to_none() {
    assert!(compile("").is_none());
}
