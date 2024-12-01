fn main() {
    let obj1 = serde_json::json!({
            "users": [
                {
                    "name": "Joe",
                    "age": 43,
                },
                {
                    "name": "Ana",
                    "age": 33,
                    "animals": {
                        "type": "dog"
                    }
                },
            ]
        });

    let obj2 = serde_json::json!({
            "users": [
                {
                    "name": "Joe",
                    "age": 43,
                },
                {
                    "name": "Ana",
                    "age": 33,
                    "animals": {
                        "type": "cat"
                    }
                },
            ]
        });

    let script = r#"
        let res = target.value_by_path("users.[_].age", curr_path);
        res == 33
        "#;

    let diff = sjdiff::DiffBuilder::default()
        .source(obj1)
        .target(obj2)
        .ignore_path_with_condition("users.[_].animals.type", sjdiff::IgnorePathCondition::Rhai(script.to_string()))
        .build();
    let diff = diff.unwrap().compare();
    print!("{:?}", diff);
}