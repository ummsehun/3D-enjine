use crate::domain::shared::value_object::ValueObject;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneName(pub String);

impl ValueObject for SceneName {}
