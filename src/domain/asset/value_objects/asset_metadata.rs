/// Metadata extracted from loaded scene data
#[derive(Debug, Clone, Default)]
pub struct AssetMetadata {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub animation_count: usize,
    pub file_size_bytes: Option<u64>,
}
