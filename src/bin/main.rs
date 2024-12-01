use rhai::{Dynamic, EvalAltResult};
use sjdiff::{ArrayIndex, Path, PathElement};

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

    let mut engine = rhai::Engine::new();
    engine.register_fn("value_by_path", |source_obj: Dynamic, path: Path| -> Result<Dynamic, Box<EvalAltResult>> {
        let mut value: Option<Dynamic> = None;
        let mut source_obj = source_obj.clone();

        for elem in path.iter() {
            match elem {
                PathElement::Key(k) => {
                    if !source_obj.is_map() { return Err("expected map, but got something else".into()); };
                    let map: rhai::Map = source_obj.as_map_ref().unwrap().clone();
                    let val = map.get(k.as_str());
                    let val = if let Some(val) = val { val } else { return Err("expected a value".into()) };
                    if val.is_map() || val.is_array() { source_obj = val.clone() } else { value = Some(val.clone()) };
                }
                PathElement::ArrayIndex(idx) => match idx {
                    ArrayIndex::Index(idx) => {
                        if !source_obj.is_array() { return Err("expected an array".into()); };
                        let array: rhai::Array = source_obj.as_array_ref().unwrap().clone();
                        let val = array.get(*idx);
                        let val = if let Some(val) = val { val } else { return Err("expected a value".into()) };
                        if val.is_map() || val.is_array() { source_obj = val.clone() } else { value = Some(val.clone()) };
                    }
                    _ => return Err("expected array index not all".into())
                },
            }
        }

        Ok(value.unwrap())
    });

    let source = engine.parse_json(obj1.to_string(), true).unwrap();
    let target = engine.parse_json(obj2.to_string(), true).unwrap();
    let mut scope = rhai::Scope::new();
    scope.push("source", source);
    scope.push("target", target);

    let path: Path = vec![
        PathElement::Key("users".to_string()),
        PathElement::ArrayIndex(ArrayIndex::Index(1)),
        PathElement::Key("age".to_string()),
    ]
        .into();

    scope.push("path", path);

    // I need a function that will return the path to
    // the value according to the current path and the given one
    // by changing `_` in given path with curr_path indices.

    let script = r#"
    source.value_by_path(path)
    "#;

    let result = engine.eval_with_scope::<Dynamic>(&mut scope, script);
    println!("{:?}", result);
}
