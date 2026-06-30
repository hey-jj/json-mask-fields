//! A deeply nested query must compile instead of overflowing the stack.

use json_mask_fields::compile;

#[test]
fn deeply_nested_query_does_not_overflow() {
    let query = "a(".repeat(100_000);
    assert!(compile(&query).is_some());
}
