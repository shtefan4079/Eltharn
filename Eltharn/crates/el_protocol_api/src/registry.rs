use crate::{ProtocolFactory, ProtocolId, ProtocolPreflightRequest, ProtocolPreflightResult};
use std::{collections::BTreeMap, fmt};
#[derive(Debug)]
pub struct ProtocolRegistry<F> {
    factories: BTreeMap<ProtocolId, F>,
}
impl<F> Default for ProtocolRegistry<F> {
    fn default() -> Self {
        Self {
            factories: BTreeMap::new(),
        }
    }
}
impl<F> ProtocolRegistry<F>
where
    F: ProtocolFactory,
{
    pub fn register(&mut self, factory: F) -> Result<(), ProtocolRegistryError> {
        let protocol_id = factory.descriptor().protocol_id;
        if self.factories.contains_key(&protocol_id) {
            return Err(ProtocolRegistryError::DuplicateProtocol(protocol_id));
        }
        self.factories.insert(protocol_id, factory);
        Ok(())
    }
    pub fn select(&self, protocol_id: &ProtocolId) -> Result<&F, ProtocolRegistryError> {
        self.factories
            .get(protocol_id)
            .ok_or_else(|| ProtocolRegistryError::UnknownProtocol(protocol_id.clone()))
    }
    pub fn preflight(
        &self,
        request: ProtocolPreflightRequest,
    ) -> Result<ProtocolPreflightResult, ProtocolRegistryError> {
        let factory = self.select(&request.protocol_id)?;
        Ok(factory.preflight(request))
    }
    pub fn protocol_ids(&self) -> impl Iterator<Item = &ProtocolId> {
        self.factories.keys()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolRegistryError {
    DuplicateProtocol(ProtocolId),
    UnknownProtocol(ProtocolId),
}
impl fmt::Display for ProtocolRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateProtocol(protocol_id) => {
                write!(
                    formatter,
                    "a factory is already registered for protocol {protocol_id}"
                )
            }
            Self::UnknownProtocol(protocol_id) => {
                write!(
                    formatter,
                    "no factory is registered for protocol {protocol_id}"
                )
            }
        }
    }
}
impl std::error::Error for ProtocolRegistryError {}
