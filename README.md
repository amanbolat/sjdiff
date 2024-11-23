# Structural JSON Diff Library for Rust

`sjdiff` – is a library for Rust that compares two JSON values and produces a structural difference between them.

## Features

- Compare any JSON value.
- Ignore JSON paths so they are not included in the result diff.
- Customize the equation logic, e.g. `null == []`, `0.111 == 0.11`, `2023-07-25T15:30:01Z == 2023-07-25T15:30:00Z`.

## Example

Compare two objects:

```rust
fn main() {
    let obj1 = serde_json::json!({
        "user": "John",
        "age": 31
    });

    let obj2 = serde_json::json!({
        "user": "John",
        "age": 33
    });

    let diff = sjdiff::DiffBuilder::default()
        .source(obj1)
        .target(obj2)
        .build()
        .unwrap();
    let diff = diff.compare();

    serde_json::to_writer_pretty(std::io::stdout(), &diff).unwrap();
}
```

See more in the [examples](./examples).

## Credits

Thanks to [teajey](https://github.com/teajey) – author of [serde_json_diff](https://github.com/teajey/serde_json_diff).
I forked that project and implemented additional features that were essential for my workflow.

## License

MIT