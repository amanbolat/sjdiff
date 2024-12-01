use crate::{ArrayIndex, Path, PathElement};
use crate::element_path_parser::parse_element_path;

/// Should be used only in rhai scope.
/// A method will be part of object map and receive two arguments:
/// `path` – string typed path which should be ignored. It will be parsed
/// and all the [`ArrayIndex::All`] will element will be replaced by the real index values
/// only if they are in the `curr_path`.
/// `curr_path` – should be passed in the rhai script. It will be injected to the scope.
/// 
/// Method will return a unit `()` if the value cannot be read by the given path. 
pub(crate) fn value_by_path(source_obj: rhai::Dynamic, path: &str, curr_path: Path) -> rhai::Dynamic {
    let path_res = parse_element_path(path);
    let path: Path = if path_res.is_ok() {path_res.unwrap().into()} else {return rhai::Dynamic::from(())};
    let path = path.replace_array_index_all_by_exact_path(curr_path);
    let path = if let Some(path) = path {path} else { return rhai::Dynamic::from(()) };
    let mut value: Option<rhai::Dynamic> = None;
    let mut source_obj = source_obj.clone();

    for elem in path.iter() {
        match elem {
            PathElement::Key(k) => {
                if !source_obj.is_map() {
                    return rhai::Dynamic::from(());
                };
                let map: rhai::Map = source_obj.as_map_ref().unwrap().clone();
                let val = map.get(k.as_str());
                let val = if let Some(val) = val {
                    val
                } else {
                    return rhai::Dynamic::from(());
                };
                if val.is_map() || val.is_array() {
                    source_obj = val.clone()
                } else {
                    value = Some(val.clone())
                };
            }
            PathElement::ArrayIndex(idx) => match idx {
                ArrayIndex::Index(idx) => {
                    if !source_obj.is_array() {
                        return rhai::Dynamic::from(());
                    };
                    let array: rhai::Array = source_obj.as_array_ref().unwrap().clone();
                    let val = array.get(*idx);
                    let val = if let Some(val) = val {
                        val
                    } else {
                        return rhai::Dynamic::from(());
                    };
                    if val.is_map() || val.is_array() {
                        source_obj = val.clone()
                    } else {
                        value = Some(val.clone())
                    };
                }
                _ => return rhai::Dynamic::from(()),
            },
        }
    }

    value.unwrap()
}
