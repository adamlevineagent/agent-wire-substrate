use agent_wire_foundation::{
    CreditAmount, CrossGraphRef, GraphSlug, HandlePath, PriceCurve, SettlementIntent,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageOfferId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PinCommitmentId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RetrievalRequestId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicationFactor(pub u16);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub minimum_seconds: u64,
    pub renew_before_expiry_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityAllocation {
    pub graph: GraphSlug,
    pub reserved_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageOffer {
    pub offer_id: StorageOfferId,
    pub provider: HandlePath,
    pub capacity_bytes: u64,
    pub capacity_allocation: Vec<CapacityAllocation>,
    pub price: PriceCurve,
    pub replication: ReplicationFactor,
    pub retention: RetentionPolicy,
    pub settlement: SettlementIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinCommitmentRequest {
    pub offer_id: StorageOfferId,
    pub requester: HandlePath,
    pub content_ref: CrossGraphRef,
    pub bytes: u64,
    pub replication: ReplicationFactor,
    pub retention: RetentionPolicy,
    pub settlement: SettlementIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinCommitment {
    pub commitment_id: PinCommitmentId,
    pub request: PinCommitmentRequest,
    pub provider: HandlePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalRequest {
    pub request_id: RetrievalRequestId,
    pub commitment_id: PinCommitmentId,
    pub requester: HandlePath,
    pub content_ref: CrossGraphRef,
    pub max_price: CreditAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalReceipt {
    pub request_id: RetrievalRequestId,
    pub content_ref: CrossGraphRef,
    pub served_by: HandlePath,
    pub bytes_served: u64,
    pub charged: CreditAmount,
}

pub trait StorageMarket {
    type Error;

    fn publish_offer(&self, offer: StorageOffer) -> Result<StorageOfferId, Self::Error>;
    fn commit_pin(&self, request: PinCommitmentRequest) -> Result<PinCommitment, Self::Error>;
    fn retrieve(&self, request: RetrievalRequest) -> Result<RetrievalReceipt, Self::Error>;
    fn renew_retention(
        &self,
        commitment_id: PinCommitmentId,
        retention: RetentionPolicy,
    ) -> Result<PinCommitment, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_offer_carries_capacity_retention_and_settlement() {
        let offer = StorageOffer {
            offer_id: StorageOfferId("offer-1".to_owned()),
            provider: HandlePath::new(["agent", "playful", "storage"]).unwrap(),
            capacity_bytes: 1_000_000,
            capacity_allocation: vec![CapacityAllocation {
                graph: GraphSlug::new("kitty").unwrap(),
                reserved_bytes: 250_000,
            }],
            price: PriceCurve {
                base: CreditAmount::from_sats(10),
                per_unit: CreditAmount::from_sats(2),
            },
            replication: ReplicationFactor(3),
            retention: RetentionPolicy {
                minimum_seconds: 86_400,
                renew_before_expiry_seconds: Some(3_600),
            },
            settlement: SettlementIntent {
                max_price: CreditAmount::from_sats(1_000),
                escrow_required: true,
            },
        };

        assert_eq!(offer.capacity_allocation[0].graph.as_str(), "kitty");
        assert_eq!(offer.replication, ReplicationFactor(3));
        assert!(offer.settlement.escrow_required);
    }

    #[test]
    fn pin_commitment_request_names_cross_graph_content() {
        let request = PinCommitmentRequest {
            offer_id: StorageOfferId("offer-1".to_owned()),
            requester: HandlePath::new(["agent", "playful", "kramer"]).unwrap(),
            content_ref: "playful/122/storage/1".parse().unwrap(),
            bytes: 512,
            replication: ReplicationFactor(2),
            retention: RetentionPolicy {
                minimum_seconds: 600,
                renew_before_expiry_seconds: None,
            },
            settlement: SettlementIntent {
                max_price: CreditAmount::from_sats(40),
                escrow_required: false,
            },
        };

        assert_eq!(request.content_ref.to_string(), "playful/122/storage/1");
        assert_eq!(request.bytes, 512);
    }
}
