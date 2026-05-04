use std::fmt;

use serde::{Deserialize, Serialize};

use crate::FoundationError;

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
}
