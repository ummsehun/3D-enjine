use crate::domain::asset::entities::asset::Asset;
use crate::domain::asset::errors::asset_error::AssetError;
use crate::domain::shared::ids::AssetId;
use std::path::Path;

pub trait AssetRepository: Send + Sync {
    fn load(&self, id: AssetId) -> Result<Asset, AssetError>;
    fn preload(&self, ids: &[AssetId]) -> Result<Vec<Asset>, AssetError>;
    fn evict(&self, id: AssetId) -> Result<(), AssetError>;
}

pub trait AssetPort: Send + Sync {
    fn load_gltf(&self, path: &Path) -> Result<Asset, AssetError>;
    fn load_pmx(&self, path: &Path) -> Result<Asset, AssetError>;
    fn load_obj(&self, path: &Path) -> Result<Asset, AssetError>;
}
