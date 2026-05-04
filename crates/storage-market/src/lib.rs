use agent_wire_foundation::{CreditAmount, HandlePath};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageOffer {
    pub content_ref: HandlePath,
    pub price: CreditAmount,
}
