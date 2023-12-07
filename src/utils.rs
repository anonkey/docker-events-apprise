use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TypeOrVector<T> {
    Type(T),
    Vector(Vec<T>),
}

pub fn match_with_vector_f<
    DataType,
    MatcherType,
    Callback: FnOnce(&MatcherType, &DataType) -> bool,
>(
    matcher: Option<TypeOrVector<MatcherType>>,
    value: Option<DataType>,
    callback: Callback,
) -> bool
where
    Callback: Copy,
{
    match matcher {
        Some(values_vec_or_type) => match value {
            Some(value) => match values_vec_or_type {
                TypeOrVector::Type(match_value) => callback(&match_value, &value),
                TypeOrVector::Vector(vector) => vector
                    .iter()
                    .any(|match_value| callback(match_value, &value)),
            },
            None => false,
        },
        None => true,
    }
}

pub fn match_with_vector<DataType>(
    matcher: Option<TypeOrVector<DataType>>,
    value: Option<DataType>,
) -> bool
where
    DataType: std::cmp::PartialEq,
{
    match matcher {
        Some(values_vec_or_type) => match value {
            Some(value) => match values_vec_or_type {
                TypeOrVector::Type(match_value) => match_value == value,
                TypeOrVector::Vector(vector) => {
                    vector.iter().any(|match_value| *match_value == value)
                }
            },
            None => false,
        },
        None => true,
    }
}
