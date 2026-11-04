//! Direct resource-policy Deployment TextFormat loading.
use crate::{Error, Goal, Result, proto};
use prost_reflect::{DescriptorPool, DynamicMessage};
/// Parses a Deployment and rejects duplicate IDs and incomplete policies.
pub fn load(path: &std::path::Path) -> Result<proto::Deployment> {
    let text =
        std::fs::read_to_string(path).map_err(|e| crate::io(path.display().to_string(), e))?;
    parse(&text)
}
/// Decodes standard TextFormat without includes, overrides or injected settings.
pub fn parse(text: &str) -> Result<proto::Deployment> {
    let decode = || -> std::result::Result<proto::Deployment, Box<dyn std::error::Error>> {
        let descriptor = DescriptorPool::decode(crate::DESCRIPTOR)?
            .get_message_by_name("resource_policy.v1.Deployment")
            .expect("compiled configuration");
        Ok(DynamicMessage::parse_text_format(descriptor, text)?.transcode_to()?)
    };
    let deployment = decode().map_err(|e| Error::Invalid(e.to_string()))?;
    let mut ids = std::collections::BTreeSet::new();
    for app in &deployment.app {
        if app.id.is_empty() || !ids.insert(&app.id) {
            return Err(Error::Invalid("empty or duplicate app ID".into()));
        }
        Goal::from_proto(
            app.policy
                .as_ref()
                .ok_or_else(|| Error::Invalid(format!("{}: policy required", app.id)))?,
        )?;
    }
    Ok(deployment)
}
