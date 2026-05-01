use crate::domain::shared::value_object::ValueObject;

#[derive(Debug, Clone, PartialEq)]
pub struct AssetFormat(pub String);

impl ValueObject for AssetFormat {}
