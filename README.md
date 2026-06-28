# json-fieldmask

Select parts of a JSON value with a small fields query and drop the rest. The
query keeps the shape of the input. It prunes the branches you did not ask for
and leaves the rest in place.

This crate implements the Google APIs partial-response `fields` language over
[`serde_json::Value`].

## Install

```toml
[dependencies]
json-fieldmask = "0.1"
```

## Use

```rust
use json_fieldmask::mask;
use serde_json::json;

let input = json!({
    "url": "x",
    "id": "1",
    "obj": { "url": "h", "a": [{ "url": 1, "z": 2 }], "c": 3 }
});

let out = mask(&input, "url,obj(url,a/url)");

assert_eq!(out, json!({
    "url": "x",
    "obj": { "url": "h", "a": [{ "url": 1 }] }
}));
```

## Query language

- `a,b` selects keys `a` and `b`.
- `a/b` selects key `b` inside `a`.
- `a(b,c)` selects `b` and `c` inside each element of array `a`.
- `*` is a wildcard over every key of the current object.
- `\` escapes the next character, so `a\/b` selects the literal key `a/b`.

The query keeps structure. An explicit `null` value stays. A missing key is
dropped. An empty object or array is preserved.

## Entry points

- `mask(obj, query)` compiles and filters in one call. A dropped or falsy result
  becomes `null`.
- `compile(query)` turns a query into a reusable mask tree.
- `filter(obj, compiled)` applies a compiled mask to a value.

## CLI

The crate ships a binary.

```sh
json-fieldmask "url,object(content,attachments/url)" input.json
cat input.json | json-fieldmask "url,object(content,attachments/url)"
```

It reads the file given as the second argument, or stdin when no file is given.
It prints compact JSON. Any error prints to stderr and exits with code 1.

## License

Licensed under the [MIT license](LICENSE).

[`serde_json::Value`]: https://docs.rs/serde_json/latest/serde_json/enum.Value.html
