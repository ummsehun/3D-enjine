/// Metadata derived from SceneCpu
#[derive(Debug, Clone, Default)]
pub struct SceneMetadata {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub animation_count: usize,
    pub mesh_count: usize,
    pub material_count: usize,
}
