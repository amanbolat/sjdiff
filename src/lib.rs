//! # Structural JSON Diff Library for Rust
//!
//! `sjdiff` – is a library for Rust that compares two JSON values and produces a structural difference between them.
//!
//! ## Examples
//!
//! **Compare two objects**
//!
//! ```rust
#![doc = include_str!("../examples/simple_object_diff.rs")]
//! ```
//!
//! Output:
//! ```json
#![doc = include_str!("../examples/simple_object_diff.json")]
//! ```
mod element_path_parser;

use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::time::Duration;
use approx::relative_eq;
use chrono::{DateTime};
use derive_builder::Builder;
use serde::{ser::SerializeMap, Serialize};
use crate::element_path_parser::parse_element_path;

#[derive(Debug, Serialize)]
#[serde(tag = "entry_difference", rename_all = "snake_case")]
pub enum EntryDifference {
    /// An entry from `target` that `source` is missing
    Missing { value: serde_json::Value },
    /// An entry that `source` has, and `target` doesn't
    Extra { value: serde_json::Value },
    /// The entry exists in both JSONs, but the values are different
    Value { value_diff: Difference },
}

#[derive(Debug)]
pub struct Map<K: Serialize, V: Serialize>(pub Vec<(K, V)>);

impl<K: Serialize, V: Serialize> Serialize for Map<K, V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "array_difference", rename_all = "snake_case")]
pub enum ArrayDifference {
    /// `source` and `target` are the same length, but some values of the same indices are different
    PairsOnly {
        /// differing pairs that appear in the overlapping indices of `source` and `target`
        different_pairs: Map<usize, Difference>,
    },
    /// `source` is shorter than `target`
    Shorter {
        /// differing pairs that appear in the overlapping indices of `source` and `target`
        different_pairs: Option<Map<usize, Difference>>,
        /// elements missing in `source` that appear in `target`
        missing_elements: Vec<serde_json::Value>,
    },
    /// `source` is longer than `target`
    Longer {
        /// differing pairs that appear in the overlapping indices of `source` and `target`
        different_pairs: Option<Map<usize, Difference>>,
        /// The amount of extra elements `source` has that `target` does not
        extra_length: usize,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    Null,
    Array,
    Bool,
    Object,
    String,
    Number,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ScalarDifference {
    Bool {
        source: bool,
        target: bool,
    },
    String {
        source: String,
        target: String,
    },
    Number {
        source: serde_json::Number,
        target: serde_json::Number,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "difference_of", rename_all = "snake_case")]
pub enum Difference {
    Scalar(ScalarDifference),
    Type {
        source_type: Type,
        source_value: serde_json::Value,
        target_type: Type,
        target_value: serde_json::Value,
    },
    Array(ArrayDifference),
    Object {
        different_entries: Map<String, EntryDifference>,
    },
}


/// Use [`DiffBuilder`] to build [`Diff`] first and run [`Diff::compare`] to get the
/// difference between two JSON values.
#[derive(Default, Builder, Debug)]
pub struct Diff {
    #[builder(setter(skip))]
    #[builder(default = vec![].into())]
    curr_path: Path,

    /// An array of paths to ignore.
    /// Use [`DiffBuilder::ignore_path`] to add them in a more convenient way.
    #[builder(default = vec![])]
    ignore_paths: Vec<IgnorePath>,

    /// If true arrays with a length of zero will be equal, regardless of whether they are nil.
    #[builder(default = false)]
    equate_empty_arrays: bool,

    /// If not zero a float comparison will be done using [`approx::relative_eq`].
    /// It's useful when you want to ignore small differences, e.g. `0.19999999999999 ~ 0.2`.
    #[builder(default = 0.0)]
    approx_float_eq_epsilon: f64,

    /// An acceptable duration difference for the JSON string values that
    /// are valid timestamps. Date approximation will only be executed
    /// when this value is not zero and a string value is a valid `rfc3339` date.
    #[builder(default = Duration::from_millis(0))]
    approx_date_time_eq_duration: Duration,

    /// Source JSON value that will be compared with [`Diff::target`].
    source: serde_json::Value,

    /// Target JSON value that will be compared with [`Diff::source`].
    target: serde_json::Value,
}

impl DiffBuilder {
    /// Set a JSON path using a string format that you want to ignore during the comparison.
    /// A string path will be parsed to [`IgnorePath`] and appended to [`Diff::ignore_paths`].
    ///
    /// ## Examples
    ///
    /// `a.[_].c` will ignore `c` key in any element of array `a`:
    ///
    /// ```json
    /// {
    ///     "a": [{"b": "3", "c": "4"}]
    /// }
    /// ```
    /// `[_]` means any index.
    ///
    /// and `a.[1].c` will ignore `c` key in the element with index 1.
    ///
    /// `address.zip` will ignore `zip` key in the `address`:
    ///
    /// ```json
    /// {
    ///     "address": {
    //         "city": "Astana",
    //         "zip": 123,
    //      }
    /// }
    /// ```
    ///
    /// <div class="warning">
    ///
    /// **NOTE**: if the element is missing in the source or target and you added
    ///  it to ignore paths, the result diff will still show it as a missing one.
    ///  Use [`DiffBuilder::ignore_path_with_missing`] with `ignore_missing` set to true instead.
    ///
    /// </div>
    pub fn ignore_path(&mut self, path: &str) -> &mut Self {
        self.ignore_path_with_missing(path, false)
    }

    /// Adds a path to the ignored ones. `ignore_missing` indicates whether the element should
    /// be ignored if it's missing in the source or target.
    /// See documentation for [`DiffBuilder::ignore_path`] for usage examples.
    pub fn ignore_path_with_missing(&mut self, path: &str, ignore_missing: bool) -> &mut Self {
        if let Ok(elements) = Path::from_str(path) {
            self.ignore_paths.get_or_insert_with(Vec::new).push(IgnorePath(elements, ignore_missing));
        }
        self
    }
}

impl Diff {
    fn arrays(
        &mut self,
        source: Vec<serde_json::Value>,
        target: Vec<serde_json::Value>,
    ) -> Option<ArrayDifference> {
        let different_pairs = self.compare_array_elements(&source, &target);
        let different_pairs = if different_pairs.is_empty() {
            None
        } else {
            Some(Map(different_pairs))
        };

        match (source.len(), target.len()) {
            (s, t) if s > t => Some(ArrayDifference::Longer {
                different_pairs,
                extra_length: s - t,
            }),
            (s, t) if s < t => Some(ArrayDifference::Shorter {
                different_pairs,
                missing_elements: target.into_iter().skip(s).collect(),
            }),
            _ => different_pairs.map(|pairs| ArrayDifference::PairsOnly { different_pairs: pairs }),
        }
    }

    fn compare_array_elements(
        &mut self,
        source: &[serde_json::Value],
        target: &[serde_json::Value],
    ) -> Vec<(usize, Difference)> {
        let mut iterations = 0;
        let res: Vec<_> = source
            .iter()
            .zip(target.iter())
            .enumerate()
            .filter_map(|(i, (s, t))| {
                iterations += 1;
                let elem_path = PathElement::ArrayIndex(ArrayIndex::Index(i));
                if i > 0 { self.curr_path.pop(); }
                self.curr_path.push(elem_path);
                self.values(s.clone(), t.clone()).map(|diff| (i, diff))
            })
            .collect();
        if iterations != 0 {
            self.curr_path.pop();
        };

        res
    }

    #[must_use]
    fn objects(
        &mut self,
        source: serde_json::Map<String, serde_json::Value>,
        mut target: serde_json::Map<String, serde_json::Value>,
    ) -> Option<Map<String, EntryDifference>> {
        let mut is_first = true;
        let mut value_differences = source
            .into_iter()
            .filter_map(|(key, source)| {
                let elem_path = PathElement::Key(key.clone());
                match is_first {
                    true => is_first = false,
                    false => { self.curr_path.pop(); }
                }
                self.curr_path.push(elem_path);

                if self.ignore_path(target.contains_key(&key)) {
                    target.remove(&key);
                    return None;
                }

                let Some(target) = target.remove(&key) else {
                    return Some((key, EntryDifference::Extra {
                        value: source
                    }));
                };

                self.values(source, target).map(|diff| (key, EntryDifference::Value { value_diff: diff }))
            })
            .collect::<Vec<_>>();
        
        if !is_first { self.curr_path.pop(); }

        value_differences.extend(target.into_iter().filter_map(|(missing_key, missing_value)| {
            let elem_path = PathElement::Key(missing_key.clone());
            self.curr_path.push(elem_path);
            let ignore = self.ignore_path(false);
            
            let res = match ignore {
                true => None,
                false => Some((missing_key, EntryDifference::Missing {
                    value: missing_value,
                })),
            };
            
            self.curr_path.pop();
            res
        }));

        match value_differences.is_empty() {
            true => None,
            false => Some(Map(value_differences))
        }
    }

    pub fn compare(mut self) -> Option<Difference> {
        self.values(self.source.clone(), self.target.clone())
    }

    fn values(&mut self, source: serde_json::Value, target: serde_json::Value) -> Option<Difference> {
        use serde_json::Value::{Array, Bool, Null, Number, Object, String};

        match (source, target) {
            (Null, Null) => None,
            (Bool(source), Bool(target)) => {
                if source == target {
                    None
                } else {
                    Some(Difference::Scalar(ScalarDifference::Bool {
                        source,
                        target,
                    }))
                }
            }
            (Number(source), Number(target)) => {
                self.compare_numbers(source, target)
            }
            (String(source), String(target)) => {
                self.compare_strings(source, target)
            }
            (Array(source), Array(target)) => self.arrays(source, target).map(Difference::Array),
            (Object(source), Object(target)) => {
                self.objects(source, target)
                    .map(|different_entries| Difference::Object { different_entries })
            }
            (Array(source), Null) if self.equate_empty_arrays && source.len().eq(&0) => None,
            (Null, Array(target)) if self.equate_empty_arrays && target.len().eq(&0) => None,
            (source, target) => {
                Some(Difference::Type {
                    source_type: source.clone().into(),
                    source_value: source,
                    target_type: target.clone().into(),
                    target_value: target,
                })
            }
        }
    }


    fn compare_strings(&self, source:String, target: String) -> Option<Difference> {
        if !self.approx_date_time_eq_duration.is_zero() {
            let source_datetime = DateTime::parse_from_rfc3339(source.as_str());
            let target_datetime = DateTime::parse_from_rfc3339(target.as_str());

            match (source_datetime, target_datetime) {
                (Ok(source_date_time), Ok(target_date_time)) => {
                    let delta = source_date_time - target_date_time;
                    let delta = delta.abs().to_std().unwrap();
                    if delta.gt(&self.approx_date_time_eq_duration) {
                        return Some(Difference::Scalar(ScalarDifference::String {
                            source,
                            target,
                        }))
                    } else {
                        return None
                    }
                },
                (_, _) => {},
            }
        }
        if source == target {
            None
        } else {
            Some(Difference::Scalar(ScalarDifference::String {
                source,
                target,
            }))
        }
    }

    fn compare_numbers(&self, source: serde_json::Number, target: serde_json::Number) -> Option<Difference> {
        if source.is_u64() && target.is_u64() || source.is_i64() && target.is_i64() {
            if source == target {
                None
            } else {
                Some(Difference::Scalar(ScalarDifference::Number {
                    source,
                    target,
                }))
            }
        } else if source.is_f64() || target.is_f64() {
            if relative_eq!(source.as_f64().unwrap(), target.as_f64().unwrap(), epsilon = self.approx_float_eq_epsilon) {
                None
            } else {
                Some(Difference::Scalar(ScalarDifference::Number {
                    source,
                    target,
                }))
            }
        } else {
            None
        }
    }

    /// Returns true if the current path should be ignored.
    /// `has_key` indicates if the opposite object has the key.
    /// So, if the function is called when the keys of source are iterated
    /// target should be checked for key existence.
    /// After it can only be called on vector of target keys, which
    /// means that all those keys are missing on the source. 
    fn ignore_path(&self, has_key: bool) -> bool {
        let path = self.ignore_paths.iter().find(|p| p.0.eq(&self.curr_path));

        match path {
            Some(IgnorePath(path, _))
            if path.eq(&self.curr_path) && has_key => true,
            Some(IgnorePath(path, ignore_missing))
            if path.eq(&self.curr_path) && !has_key && *ignore_missing => true,
            Some(IgnorePath(path, ignore_missing))
            if path.eq(&self.curr_path) && !has_key && !ignore_missing => false,
            _ => false,
        }
    }
}

impl From<serde_json::Value> for Type {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Type::Null,
            serde_json::Value::Bool(_) => Type::Bool,
            serde_json::Value::Number(_) => Type::Number,
            serde_json::Value::String(_) => Type::String,
            serde_json::Value::Array(_) => Type::Array,
            serde_json::Value::Object(_) => Type::Object,
        }
    }
}

impl PartialEq for ArrayIndex {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ArrayIndex::Index(a), ArrayIndex::Index(b)) => a == b,
            (ArrayIndex::All, ArrayIndex::Index(_)) => true,
            (ArrayIndex::Index(_), ArrayIndex::All) => true,
            (ArrayIndex::All, ArrayIndex::All) => true,
        }
    }
}

#[derive(Eq, Clone, Debug)]
pub enum ArrayIndex {
    Index(usize),
    All,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum PathElement {
    Key(String),
    ArrayIndex(ArrayIndex),
}

#[derive(PartialEq, Clone, Debug)]
pub struct IgnorePath(pub Path, pub bool);

#[derive(PartialEq, Clone, Debug, Default)]
pub struct Path(Vec<PathElement>);

impl Deref for Path {
    type Target = Vec<PathElement>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Path {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<PathElement>> for Path {
    fn from(value: Vec<PathElement>) -> Self {
        Self(value)
    }
}

impl FromStr for Path {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Path(parse_element_path(s)?))
    }
}

impl TryFrom<&str> for Path {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}


#[cfg(test)]
mod tests {
    use std::time::Duration;
    use serde_json::json;
    use crate::DiffBuilder;

    #[test]
    fn ignore_source_missing() {
        let obj1 = json!({
            "name": "John Doe",
        });

        let obj2 = json!({
            "name": "John Doe",
            "age": 30
        });

        let diff = DiffBuilder::default()
            .source(obj1)
            .target(obj2)
            .ignore_path_with_missing("age", true)
            .build();
        let diff = diff.unwrap().compare();

        assert!(diff.is_none(), "{:?}", diff);
    }

    #[test]
    fn equal_objects() {
        let obj1 = json!({
            "string": "b",
            "int": 1,
            "float": 1.0,
            "bool": true,
            "int_array": [1, 2, 3],
            "float_array": [1.0, 2.0, 3.0],
            "bool_array": [true, false, false],
            "string_array": ["foo", "bar"],
            "empty_array": [],
            "null": null,
            "object": {
                "string": "c",
                "int": 1,
                "float": 1.0,
                "bool": true,
                "array": [1, 2, 3],
                "null": null,
                "object": {
                    "string": "d",
                }
            },
        });

        let obj2 = json!({
            "string": "b",
            "int": 1,
            "float": 1.0,
            "bool": true,
            "int_array": [1, 2, 3],
            "float_array": [1.0, 2.0, 3.0],
            "bool_array": [true, false, false],
            "string_array": ["foo", "bar"],
            "empty_array": [],
            "null": null,
            "object": {
                "string": "c",
                "int": 1,
                "float": 1.0,
                "bool": true,
                "array": [1, 2, 3],
                "null": null,
                "object": {
                    "string": "d",
                }
            },
        });

        let diff = DiffBuilder::default().source(obj1).target(obj2).build().unwrap();
        let diff = diff.compare();
        assert_eq!(true, diff.is_none(), "diff should be None, but got: {:?}", diff);
    }


    #[test]
    fn ignore_fields() {
        let user_1 = json!({
            "user": "John",
            "address": {
                "city": "Astana",
                "zip": 123,
            },
            "animals": ["dog", "cat"],
            "object_array": [{"a": "b", "c": "d"}],
            "optional_array": [],
            "target_missing_value": 1,
        });

        let user_2 = json!({
            "user": "Joe",
            "address": {
                "city": "Boston",
                "zip": 312,
            },
            "animals": ["dog", "cat"],
            "object_array": [{"a": "3", "c": "d"}],
            "optional_array": null,
        });

        let diff = DiffBuilder::default()
            .ignore_path("user")
            .ignore_path("address.city")
            .ignore_path("address.zip")
            .ignore_path("object_array.[_].a")
            .ignore_path_with_missing("target_missing_value", true)
            .equate_empty_arrays(true)
            .source(user_1)
            .target(user_2)
            .build()
            .unwrap();

        let diff = diff.compare();

        assert_eq!(true, diff.is_none(), "diff should be None, but got: {:?}", diff);
    }

    #[test]
    fn approx_float_eq() {
        let obj1 = json!({
            "float": 1.34
        });

        let obj2 = json!({
            "float": 1.341
        });

        let diff = DiffBuilder::default()
            .approx_float_eq_epsilon(0.001)
            .source(obj1).target(obj2).build().unwrap();

        let diff = diff.compare();

        assert_eq!(true, diff.is_none(), "diff should be None, but got: {:?}", diff);
    }

    #[test]
    fn approx_date_time_eq() {
        let obj1 = json!({
            "ts": "2023-07-25T15:30:01Z"
        });

        let obj2 = json!({
            "ts": "2023-07-25T15:30:00Z"
        });

        let diff = DiffBuilder::default()
            .approx_date_time_eq_duration(Duration::from_secs(1))
            .source(obj1).target(obj2).build().unwrap();

        let diff = diff.compare();

        assert_eq!(true, diff.is_none(), "diff should be None, but got: {:?}", diff);
    }
}