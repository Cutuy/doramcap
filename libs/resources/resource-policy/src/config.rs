//! Resource-policy adapter for the shared pbtxt resolver.
use crate::{Goal, proto};
pub use pbtxt_config::{Override, Resolved};
use pbtxt_config::{Payload, Result};
impl Payload for proto::Policy {
    const NAME: &'static str = "resource_policy.v1.Policy";
    const DESCRIPTOR: &'static [u8] = crate::DESCRIPTOR;
    fn validate_partial(&self) -> Result<()> {
        crate::validate_partial(self).map_err(|e| pbtxt_config::Error::Invalid(e.to_string()))
    }
    fn overlay(&self, target: &mut Self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.placement.is_some() {
            target.placement = self.placement.clone();
            fields.push("placement");
        }
        if self.rt_priority.is_some() {
            target.rt_priority = self.rt_priority;
            fields.push("rt_priority");
        }
        fields
    }
    fn normalize(&self) -> Result<Self> {
        Goal::from_proto(self)
            .map(|goal| goal.to_proto())
            .map_err(|e| pbtxt_config::Error::Invalid(e.to_string()))
    }
}
pub fn parse_fragment(text: &str) -> Result<pbtxt_config::Fragment<proto::Policy>> {
    pbtxt_config::parse_fragment(text)
}
pub fn resolve(path: &std::path::Path, registry: &[&str]) -> Result<Resolved<proto::Policy>> {
    pbtxt_config::resolve(path, registry)
}
