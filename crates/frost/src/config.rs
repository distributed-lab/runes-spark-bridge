use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorConfig {
    pub threshold: u16,
    pub total_participants: u16,
    pub verifier_identifiers: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerConfig {
    pub identifier: u16,
    pub threshold: u16,
    pub total_participants: u16,
}
