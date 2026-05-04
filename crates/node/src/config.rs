use agent_wire_foundation::{
    Capability, CapabilityGrant, CreditAmount, EndpointUrl, FoundationError, HandlePath,
    MasterKeyId, MasterPublicKey, NamespaceId, ResourceBudget, SandboxPolicy, SignatureAlgorithm,
    TunnelUrl,
};
use serde::{Deserialize, Serialize};

use crate::server::OperatorApiSurface;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeConfig {
    pub operator: HandlePath,
    pub namespace: NamespaceId,
    pub node_id: String,
    pub mainnet_endpoint: EndpointUrl,
    pub local_api_endpoint: EndpointUrl,
    pub requested_tunnel: TunnelUrl,
    pub keys: NodeKeys,
    pub opt_in: OptInPolicy,
    pub persistence: LocalPersistence,
    pub surfaces: OperatorApiSurface,
    pub sandbox: SandboxPolicy,
}

impl NodeConfig {
    pub fn demo() -> Result<Self, FoundationError> {
        let operator = HandlePath::new(["agent", "playful", "kramer"])?;
        Ok(Self {
            operator,
            namespace: NamespaceId::new("playful")?,
            node_id: "node2-demo".to_owned(),
            mainnet_endpoint: EndpointUrl::parse("https://newsbleach.com/api/v1")?,
            local_api_endpoint: EndpointUrl::parse("http://127.0.0.1:8787")?,
            requested_tunnel: TunnelUrl::parse("https://node2-demo.example")?,
            keys: NodeKeys {
                master_public_key: MasterPublicKey::new(
                    MasterKeyId::new("demo-master")?,
                    SignatureAlgorithm::Ed25519,
                    vec![7; 32],
                )?,
            },
            opt_in: OptInPolicy {
                compute_provider: true,
                compute_requester: true,
                storage_provider: true,
                relay_operator: true,
                wake_triggers: true,
            },
            persistence: LocalPersistence {
                config_dir: "~/.wire-node/config".to_owned(),
                state_dir: "~/.wire-node/state".to_owned(),
                contribution_cache_dir: "~/.wire-node/contributions".to_owned(),
                key_store_label: "agent-wire-substrate-node-v2".to_owned(),
            },
            surfaces: OperatorApiSurface::all_enabled(EndpointUrl::parse("http://127.0.0.1:8787")?),
            sandbox: SandboxPolicy {
                grants: vec![
                    CapabilityGrant {
                        capability: Capability::ReadContribution,
                        reason: "sync mainnet contributions".to_owned(),
                    },
                    CapabilityGrant {
                        capability: Capability::WriteContribution,
                        reason: "publish local node receipts".to_owned(),
                    },
                    CapabilityGrant {
                        capability: Capability::OpenTunnel,
                        reason: "accept tunnel-based peer delivery".to_owned(),
                    },
                    CapabilityGrant {
                        capability: Capability::ExecuteModel,
                        reason: "serve opted-in compute-market jobs".to_owned(),
                    },
                    CapabilityGrant {
                        capability: Capability::EmitEvent,
                        reason: "wake on Wire coordination events".to_owned(),
                    },
                ],
                budget: ResourceBudget {
                    max_credits: CreditAmount::from_sats(10_000),
                    max_events: 10_000,
                    max_wall_time_ms: 86_400_000,
                },
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeKeys {
    pub master_public_key: MasterPublicKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptInPolicy {
    pub compute_provider: bool,
    pub compute_requester: bool,
    pub storage_provider: bool,
    pub relay_operator: bool,
    pub wake_triggers: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalPersistence {
    pub config_dir: String,
    pub state_dir: String,
    pub contribution_cache_dir: String,
    pub key_store_label: String,
}
