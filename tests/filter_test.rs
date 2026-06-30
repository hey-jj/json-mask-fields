//! Tests for `filter` with a hand-built mask tree.
//!
//! These cases bypass `compile` and feed the engine a tree directly. They keep
//! an engine regression separate from a compiler regression and pin the
//! drop-versus-keep distinction that `filter` reports through its `Option`.

use json_mask_fields::{compile, filter, CompiledMask, Node};
use serde_json::json;

fn mask<I>(entries: I) -> CompiledMask
where
    I: IntoIterator<Item = (&'static str, Node)>,
{
    entries
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

fn obj(properties: Option<CompiledMask>) -> Node {
    Node {
        is_array: false,
        is_wildcard: false,
        properties,
    }
}

fn arr(properties: Option<CompiledMask>) -> Node {
    Node {
        is_array: true,
        is_wildcard: false,
        properties,
    }
}

fn wild(properties: Option<CompiledMask>) -> Node {
    Node {
        is_array: false,
        is_wildcard: true,
        properties,
    }
}

#[test]
fn filter_with_hand_built_mask() {
    // The compiled form of a,b(d/*/z,b(g)),c,d/e,*
    let compiled = mask([
        ("a", obj(None)),
        (
            "b",
            arr(Some(mask([
                (
                    "d",
                    obj(Some(mask([("*", wild(Some(mask([("z", obj(None))]))))]))),
                ),
                ("b", arr(Some(mask([("g", obj(None))])))),
            ]))),
        ),
        ("c", obj(None)),
        ("d/e", obj(None)),
        ("*", obj(None)),
    ]);

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

    assert_eq!(filter(&object, Some(&compiled)), Some(expected));
}

#[test]
fn none_mask_is_passthrough_without_coercion() {
    // A None mask returns the value unchanged. filter does not coerce a falsy
    // scalar to null the way mask does. compile("") yields None, so wiring the
    // two together exercises the empty-string to None to passthrough chain.
    assert_eq!(filter(&json!(0), compile("").as_ref()), Some(json!(0)));
    assert_eq!(
        filter(&json!(false), compile("").as_ref()),
        Some(json!(false))
    );
    assert_eq!(filter(&json!(""), compile("").as_ref()), Some(json!("")));

    let o = json!({"a": 1, "b": {"c": 2}});
    assert_eq!(filter(&o, compile("").as_ref()), Some(o));
}

#[test]
fn filter_reports_drop_distinct_from_kept_null() {
    // A scalar under a real mask is dropped and reported as None. An explicit
    // null is kept and reported as Some(null). The two are not the same.
    assert_eq!(filter(&json!("foo"), compile("a").as_ref()), None);
    assert_eq!(filter(&json!(5), compile("a").as_ref()), None);
    assert_eq!(
        filter(&json!(null), compile("a").as_ref()),
        Some(json!(null))
    );
}
