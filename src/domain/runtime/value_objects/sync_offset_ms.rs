use crate::domain::shared::value_object::ValueObject;

#[derive(Debug, Clone, PartialEq)]
pub struct SyncOffsetMs(pub i32);

impl ValueObject for SyncOffsetMs {}
