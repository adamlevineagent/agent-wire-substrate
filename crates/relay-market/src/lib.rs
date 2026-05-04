use agent_wire_foundation::{
    CreditAmount, CrossGraphRef, EndpointUrl, HandlePath, SettlementIntent, TunnelUrl,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelayOfferId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathLeaseId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HopCapability {
    HttpTunnel,
    EventStream,
    StoreAndForward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyTier {
    Direct,
    Shielded,
    Onion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RotationPolicy {
    pub rotate_after_seconds: u64,
    pub max_reuses: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayOffer {
    pub offer_id: RelayOfferId,
    pub operator: HandlePath,
    pub ingress: EndpointUrl,
    pub egress: TunnelUrl,
    pub capabilities: Vec<HopCapability>,
    pub privacy_tiers: Vec<PrivacyTier>,
    pub price_per_hop: CreditAmount,
    pub settlement: SettlementIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathLeaseRequest {
    pub requester: HandlePath,
    pub desired_hops: u8,
    pub required_capabilities: Vec<HopCapability>,
    pub privacy_tier: PrivacyTier,
    pub rotation: RotationPolicy,
    pub max_price: CreditAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayHop {
    pub operator: HandlePath,
    pub ingress: EndpointUrl,
    pub egress: TunnelUrl,
    pub capabilities: Vec<HopCapability>,
    pub price: CreditAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayPathLease {
    pub lease_id: PathLeaseId,
    pub requester: HandlePath,
    pub hops: Vec<RelayHop>,
    pub privacy_tier: PrivacyTier,
    pub rotation: RotationPolicy,
    pub settlement: SettlementIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerHopSettlement {
    pub lease_id: PathLeaseId,
    pub hop_index: u16,
    pub payee: HandlePath,
    pub amount: CreditAmount,
    pub receipt_ref: Option<CrossGraphRef>,
}

pub trait RelayMarket {
    type Error;

    fn publish_offer(&self, offer: RelayOffer) -> Result<RelayOfferId, Self::Error>;
    fn lease_path(&self, request: PathLeaseRequest) -> Result<RelayPathLease, Self::Error>;
    fn rotate_path(
        &self,
        lease_id: PathLeaseId,
        policy: RotationPolicy,
    ) -> Result<RelayPathLease, Self::Error>;
    fn settle_hop(&self, settlement: PerHopSettlement) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_offer_declares_capabilities_privacy_and_settlement() {
        let offer = RelayOffer {
            offer_id: RelayOfferId("relay-offer-1".to_owned()),
            operator: HandlePath::new(["agent", "playful", "relay"]).unwrap(),
            ingress: EndpointUrl::parse("https://relay.example/ingress").unwrap(),
            egress: TunnelUrl::parse("https://relay.example/tunnel").unwrap(),
            capabilities: vec![HopCapability::HttpTunnel, HopCapability::EventStream],
            privacy_tiers: vec![PrivacyTier::Shielded, PrivacyTier::Onion],
            price_per_hop: CreditAmount::from_sats(25),
            settlement: SettlementIntent {
                max_price: CreditAmount::from_sats(100),
                escrow_required: true,
            },
        };

        assert!(offer.capabilities.contains(&HopCapability::EventStream));
        assert!(offer.privacy_tiers.contains(&PrivacyTier::Onion));
        assert_eq!(offer.price_per_hop.as_sats(), 25);
    }

    #[test]
    fn per_hop_settlement_can_cite_receipt_ref() {
        let settlement = PerHopSettlement {
            lease_id: PathLeaseId("lease-1".to_owned()),
            hop_index: 1,
            payee: HandlePath::new(["agent", "playful", "relay"]).unwrap(),
            amount: CreditAmount::from_sats(12),
            receipt_ref: Some("playful/122/relay/1".parse().unwrap()),
        };

        assert_eq!(settlement.hop_index, 1);
        assert_eq!(
            settlement.receipt_ref.unwrap().to_string(),
            "playful/122/relay/1"
        );
    }
}
