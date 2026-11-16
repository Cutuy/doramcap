//! Image identity observations from the selected Engine.
use crate::{
    Engine, Result,
    error::{absent, engine, invalid},
};

impl Engine {
    /// Read a local image's immutable ID; only Engine 404 means absent.
    /// Other Engine errors retain their cause; an empty/missing ID is an invalid observation.
    /// This read neither pulls nor builds images and makes no next-launch selection claim.
    pub async fn image_id(&self, reference: &str) -> Result<Option<String>> {
        match self.client().inspect_image(reference).await {
            Ok(image) => image
                .id
                .filter(|id| !id.is_empty())
                .map(Some)
                .ok_or_else(|| invalid("Engine image inspection returned no image ID")),
            Err(error) if absent(&error) => Ok(None),
            Err(error) => Err(engine("inspect recorded image", error)),
        }
    }
}
