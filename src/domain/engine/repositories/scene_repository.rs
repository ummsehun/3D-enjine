use crate::domain::engine::{entities::scene::Scene, errors::engine_error::EngineError};
use crate::domain::shared::ids::SceneId;

pub trait SceneRepository: Send + Sync {
    fn load(&self, id: SceneId) -> Result<Scene, EngineError>;
    fn save(&self, scene: &Scene) -> Result<(), EngineError>;
}
