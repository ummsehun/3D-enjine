use crate::domain::shared::value_object::ValueObject;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct AssetPath(pub PathBuf);

impl ValueObject for AssetPath {}
