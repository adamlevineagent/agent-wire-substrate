use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{CrossGraphRef, FoundationError};

pub const MAX_ECONOMIC_KEY_BYTES: usize = 128;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct CreditAmount(u128);

impl CreditAmount {
    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn from_sats(value: u128) -> Self {
        Self(value)
    }

    pub const fn as_sats(self) -> u128 {
        self.0
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }
}

impl TryFrom<i128> for CreditAmount {
    type Error = FoundationError;

    fn try_from(value: i128) -> Result<Self, Self::Error> {
        if value < 0 {
            return Err(FoundationError::OutOfRange {
                field: "credit_amount",
            });
        }
        Ok(Self(value as u128))
    }
}

impl fmt::Display for CreditAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} sats", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceCurve {
    pub base: CreditAmount,
    pub per_unit: CreditAmount,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementIntent {
    pub max_price: CreditAmount,
    pub escrow_required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        validate_economic_key("idempotency_key", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = FoundationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<IdempotencyKey> for String {
    fn from(value: IdempotencyKey) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FillKey(String);

impl FillKey {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        validate_economic_key("fill_key", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for FillKey {
    type Error = FoundationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<FillKey> for String {
    fn from(value: FillKey) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "QuoteReceiptParts", into = "QuoteReceiptParts")]
pub struct QuoteReceipt {
    quote_ref: CrossGraphRef,
    idempotency_key: IdempotencyKey,
    quoted_price: CreditAmount,
    expires_at_ms: u64,
}

impl QuoteReceipt {
    pub fn new(
        quote_ref: CrossGraphRef,
        idempotency_key: IdempotencyKey,
        quoted_price: CreditAmount,
        expires_at_ms: u64,
    ) -> Result<Self, FoundationError> {
        if quoted_price == CreditAmount::zero() {
            return Err(FoundationError::OutOfRange {
                field: "quoted_price",
            });
        }
        if expires_at_ms == 0 {
            return Err(FoundationError::OutOfRange {
                field: "expires_at_ms",
            });
        }
        Ok(Self {
            quote_ref,
            idempotency_key,
            quoted_price,
            expires_at_ms,
        })
    }

    pub fn quote_ref(&self) -> &CrossGraphRef {
        &self.quote_ref
    }

    pub fn idempotency_key(&self) -> &IdempotencyKey {
        &self.idempotency_key
    }

    pub fn quoted_price(&self) -> CreditAmount {
        self.quoted_price
    }

    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }
}

#[derive(Serialize, Deserialize)]
struct QuoteReceiptParts {
    quote_ref: CrossGraphRef,
    idempotency_key: IdempotencyKey,
    quoted_price: CreditAmount,
    expires_at_ms: u64,
}

impl TryFrom<QuoteReceiptParts> for QuoteReceipt {
    type Error = FoundationError;

    fn try_from(value: QuoteReceiptParts) -> Result<Self, Self::Error> {
        Self::new(
            value.quote_ref,
            value.idempotency_key,
            value.quoted_price,
            value.expires_at_ms,
        )
    }
}

impl From<QuoteReceipt> for QuoteReceiptParts {
    fn from(value: QuoteReceipt) -> Self {
        Self {
            quote_ref: value.quote_ref,
            idempotency_key: value.idempotency_key,
            quoted_price: value.quoted_price,
            expires_at_ms: value.expires_at_ms,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SettlementCommitParts", into = "SettlementCommitParts")]
pub struct SettlementCommit {
    quote_receipt: QuoteReceipt,
    fill_key: FillKey,
    committed_price: CreditAmount,
}

impl SettlementCommit {
    pub fn new(
        quote_receipt: QuoteReceipt,
        fill_key: FillKey,
        committed_price: CreditAmount,
    ) -> Result<Self, FoundationError> {
        if committed_price == CreditAmount::zero() || committed_price > quote_receipt.quoted_price()
        {
            return Err(FoundationError::OutOfRange {
                field: "committed_price",
            });
        }
        Ok(Self {
            quote_receipt,
            fill_key,
            committed_price,
        })
    }

    pub fn quote_receipt(&self) -> &QuoteReceipt {
        &self.quote_receipt
    }

    pub fn fill_key(&self) -> &FillKey {
        &self.fill_key
    }

    pub fn committed_price(&self) -> CreditAmount {
        self.committed_price
    }
}

#[derive(Serialize, Deserialize)]
struct SettlementCommitParts {
    quote_receipt: QuoteReceipt,
    fill_key: FillKey,
    committed_price: CreditAmount,
}

impl TryFrom<SettlementCommitParts> for SettlementCommit {
    type Error = FoundationError;

    fn try_from(value: SettlementCommitParts) -> Result<Self, Self::Error> {
        Self::new(value.quote_receipt, value.fill_key, value.committed_price)
    }
}

impl From<SettlementCommit> for SettlementCommitParts {
    fn from(value: SettlementCommit) -> Self {
        Self {
            quote_receipt: value.quote_receipt,
            fill_key: value.fill_key,
            committed_price: value.committed_price,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SettlementSettledParts", into = "SettlementSettledParts")]
pub struct SettlementSettled {
    commit: SettlementCommit,
    settlement_ref: CrossGraphRef,
    settled_price: CreditAmount,
}

impl SettlementSettled {
    pub fn new(
        commit: SettlementCommit,
        settlement_ref: CrossGraphRef,
        settled_price: CreditAmount,
    ) -> Result<Self, FoundationError> {
        if settled_price == CreditAmount::zero() || settled_price > commit.committed_price() {
            return Err(FoundationError::OutOfRange {
                field: "settled_price",
            });
        }
        Ok(Self {
            commit,
            settlement_ref,
            settled_price,
        })
    }

    pub fn commit(&self) -> &SettlementCommit {
        &self.commit
    }

    pub fn settlement_ref(&self) -> &CrossGraphRef {
        &self.settlement_ref
    }

    pub fn settled_price(&self) -> CreditAmount {
        self.settled_price
    }
}

#[derive(Serialize, Deserialize)]
struct SettlementSettledParts {
    commit: SettlementCommit,
    settlement_ref: CrossGraphRef,
    settled_price: CreditAmount,
}

impl TryFrom<SettlementSettledParts> for SettlementSettled {
    type Error = FoundationError;

    fn try_from(value: SettlementSettledParts) -> Result<Self, Self::Error> {
        Self::new(value.commit, value.settlement_ref, value.settled_price)
    }
}

impl From<SettlementSettled> for SettlementSettledParts {
    fn from(value: SettlementSettled) -> Self {
        Self {
            commit: value.commit,
            settlement_ref: value.settlement_ref,
            settled_price: value.settled_price,
        }
    }
}

fn validate_economic_key(field: &'static str, value: &str) -> Result<(), FoundationError> {
    if value.is_empty() {
        return Err(FoundationError::EmptyField { field });
    }
    if value.len() > MAX_ECONOMIC_KEY_BYTES {
        return Err(FoundationError::OutOfRange { field });
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
    }) {
        return Err(FoundationError::InvalidCharacter { field });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credit_amount_rejects_negative_values() {
        assert_eq!(
            CreditAmount::try_from(-1_i128),
            Err(FoundationError::OutOfRange {
                field: "credit_amount"
            })
        );
    }

    #[test]
    fn credit_amount_checked_adds() {
        let left = CreditAmount::from_sats(10);
        let right = CreditAmount::from_sats(5);

        assert_eq!(left.checked_add(right).unwrap().as_sats(), 15);
    }

    #[test]
    fn idempotency_and_fill_keys_are_bounded() {
        assert!(IdempotencyKey::new("quote:playful/124/15").is_ok());
        assert_eq!(
            IdempotencyKey::new("bad key"),
            Err(FoundationError::InvalidCharacter {
                field: "idempotency_key"
            })
        );
        assert_eq!(
            FillKey::new("x".repeat(MAX_ECONOMIC_KEY_BYTES + 1)),
            Err(FoundationError::OutOfRange { field: "fill_key" })
        );
    }

    #[test]
    fn settlement_commit_and_settled_prices_are_quote_bounded() {
        let quote_receipt = QuoteReceipt::new(
            "playful/124/quote/1".parse().unwrap(),
            IdempotencyKey::new("quote-1").unwrap(),
            CreditAmount::from_sats(50),
            1_000,
        )
        .unwrap();

        assert_eq!(
            SettlementCommit::new(
                quote_receipt.clone(),
                FillKey::new("fill-1").unwrap(),
                CreditAmount::from_sats(51)
            ),
            Err(FoundationError::OutOfRange {
                field: "committed_price"
            })
        );

        let commit = SettlementCommit::new(
            quote_receipt,
            FillKey::new("fill-1").unwrap(),
            CreditAmount::from_sats(45),
        )
        .unwrap();

        assert_eq!(
            SettlementSettled::new(
                commit.clone(),
                "playful/124/settlement/1".parse().unwrap(),
                CreditAmount::from_sats(46)
            ),
            Err(FoundationError::OutOfRange {
                field: "settled_price"
            })
        );
        assert_eq!(
            SettlementSettled::new(
                commit,
                "playful/124/settlement/1".parse().unwrap(),
                CreditAmount::from_sats(44)
            )
            .unwrap()
            .settled_price()
            .as_sats(),
            44
        );
    }

    #[test]
    fn serde_rejects_invalid_economic_boundaries() {
        let key = serde_json::from_str::<IdempotencyKey>("\"bad key\"");
        assert!(key.is_err());

        let receipt = serde_json::json!({
            "quote_ref": "playful/124/quote/1",
            "idempotency_key": "quote-1",
            "quoted_price": 0,
            "expires_at_ms": 1_000
        });
        assert!(serde_json::from_value::<QuoteReceipt>(receipt).is_err());

        let commit = serde_json::json!({
            "quote_receipt": {
                "quote_ref": "playful/124/quote/1",
                "idempotency_key": "quote-1",
                "quoted_price": 50,
                "expires_at_ms": 1_000
            },
            "fill_key": "fill-1",
            "committed_price": 51
        });
        assert!(serde_json::from_value::<SettlementCommit>(commit).is_err());
    }
}
