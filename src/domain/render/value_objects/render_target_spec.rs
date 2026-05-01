use crate::domain::shared::value_object::ValueObject;

#[derive(Debug, Clone, PartialEq)]
pub struct RenderTargetSpec {
    pub width: u32,
    pub height: u32,
}

impl ValueObject for RenderTargetSpec {}
