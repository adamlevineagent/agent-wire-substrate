use agent_wire_contracts::MoneyAmountDto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CreditAmount(u64);

impl CreditAmount {
    pub const ZERO: Self = Self(0);

    pub fn new(credits: u64) -> Self {
        Self(credits)
    }

    pub fn as_credits(self) -> u64 {
        self.0
    }
}

impl From<MoneyAmountDto> for CreditAmount {
    fn from(value: MoneyAmountDto) -> Self {
        Self::new(value.credits)
    }
}

impl From<CreditAmount> for MoneyAmountDto {
    fn from(value: CreditAmount) -> Self {
        Self {
            credits: value.as_credits(),
        }
    }
}
