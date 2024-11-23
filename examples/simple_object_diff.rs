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
