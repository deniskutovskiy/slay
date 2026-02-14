use serde::{Deserialize, Serialize};

use crate::NodeId;

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq)]
pub struct EdgeConfig {
    pub latency_us: u64,       // Base one-way latency
    pub jitter_us: u64,        // Amplitude of random variation
    pub packet_loss_rate: f32, // Probability of packet drop (0.0 - 1.0)
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            latency_us: 10_000,
            jitter_us: 0,
            packet_loss_rate: 0.0,
        }
    }
}

/// A physical link between two nodes.
/// It contains configuration for both directions.
/// Stored by canonical key (min_id, max_id).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Link {
    pub min_to_max: EdgeConfig,
    pub max_to_min: EdgeConfig,
}

impl Default for Link {
    fn default() -> Self {
        Self {
            min_to_max: EdgeConfig::default(),
            max_to_min: EdgeConfig::default(),
        }
    }
}

impl Link {
    pub fn get_config(&self, from: NodeId, to: NodeId) -> &EdgeConfig {
        if from < to {
            &self.min_to_max
        } else {
            &self.max_to_min
        }
    }

    pub fn get_config_mut(&mut self, from: NodeId, to: NodeId) -> &mut EdgeConfig {
        if from < to {
            &mut self.min_to_max
        } else {
            &mut self.max_to_min
        }
    }
}

pub fn canonical_key(a: NodeId, b: NodeId) -> (NodeId, NodeId) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}
