use serde::{Deserialize, Serialize};

use agent_wire_contracts::{HandlePathDto, MoneyAmountDto, TunnelEndpointDto};

use crate::economics::CreditAmount;
use crate::namespace::NamespaceId;
use crate::refs::CrossGraphRef;
use crate::transport::TunnelUrl;
use crate::FoundationError;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BoundaryName(String);

impl BoundaryName {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(FoundationError::EmptyField { field: "boundary" });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedContract<T> {
    boundary: BoundaryName,
    inner: T,
}

impl<T> WrappedContract<T> {
    pub fn wrap_contract(boundary: BoundaryName, inner: T) -> Self {
        Self { boundary, inner }
    }

    pub fn boundary(&self) -> &BoundaryName {
        &self.boundary
    }

    pub fn as_inner(&self) -> &T {
        &self.inner
    }

    pub fn unwrap_contract(self) -> T {
        self.inner
    }
}

pub trait FromContractDto<Dto>: Sized {
    type Error;

    fn from_contract_dto(dto: WrappedContract<Dto>) -> Result<Self, Self::Error>;
}

pub trait IntoContractDto<Dto> {
    fn into_contract_dto(self, boundary: BoundaryName) -> WrappedContract<Dto>;
}

impl TryFrom<HandlePathDto> for CrossGraphRef {
    type Error = FoundationError;

    fn try_from(value: HandlePathDto) -> Result<Self, Self::Error> {
        Ok(Self {
            namespace: NamespaceId::new(value.handle)?,
            day: value.wire_day,
            slug: value.graph_slug,
            sequence: value.sequence,
        })
    }
}

impl From<CrossGraphRef> for HandlePathDto {
    fn from(value: CrossGraphRef) -> Self {
        Self {
            handle: value.namespace.as_str().to_owned(),
            wire_day: value.day,
            graph_slug: value.slug,
            sequence: value.sequence,
        }
    }
}

impl From<MoneyAmountDto> for CreditAmount {
    fn from(value: MoneyAmountDto) -> Self {
        Self::from_sats(value.credits as u128)
    }
}

impl TryFrom<CreditAmount> for MoneyAmountDto {
    type Error = FoundationError;

    fn try_from(value: CreditAmount) -> Result<Self, Self::Error> {
        let credits = u64::try_from(value.as_sats()).map_err(|_| FoundationError::OutOfRange {
            field: "money_amount",
        })?;
        Ok(Self { credits })
    }
}

impl TryFrom<TunnelEndpointDto> for TunnelUrl {
    type Error = FoundationError;

    fn try_from(value: TunnelEndpointDto) -> Result<Self, Self::Error> {
        TunnelUrl::parse(&value.url)
    }
}

impl From<TunnelUrl> for TunnelEndpointDto {
    fn from(value: TunnelUrl) -> Self {
        Self {
            url: value.as_str().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_keeps_boundary_visible() {
        let boundary = BoundaryName::new("transport").unwrap();
        let wrapped = WrappedContract::wrap_contract(boundary.clone(), 42_u64);

        assert_eq!(wrapped.boundary(), &boundary);
        assert_eq!(*wrapped.as_inner(), 42);
        assert_eq!(wrapped.unwrap_contract(), 42);
    }

    #[test]
    fn converts_handle_path_dto_to_foundation_ref() {
        let dto = HandlePathDto {
            handle: "playful".to_owned(),
            wire_day: 122,
            graph_slug: Some("substrate".to_owned()),
            sequence: 7,
        };

        let runtime = CrossGraphRef::try_from(dto.clone()).unwrap();
        assert_eq!(runtime.to_string(), "playful/122/substrate/7");
        assert_eq!(HandlePathDto::from(runtime), dto);
    }

    #[test]
    fn rejects_invalid_handle_path_dto_at_boundary() {
        let dto = HandlePathDto {
            handle: "bad handle".to_owned(),
            wire_day: 122,
            graph_slug: None,
            sequence: 7,
        };

        assert_eq!(
            CrossGraphRef::try_from(dto),
            Err(FoundationError::InvalidCharacter { field: "namespace" })
        );
    }

    #[test]
    fn converts_money_dto_without_leaking_contract_shape() {
        let amount = CreditAmount::from(MoneyAmountDto { credits: 42 });

        assert_eq!(amount.as_sats(), 42);
        assert_eq!(
            MoneyAmountDto::try_from(amount).unwrap(),
            MoneyAmountDto { credits: 42 }
        );
    }

    #[test]
    fn converts_tunnel_endpoint_dto_with_runtime_validation() {
        let tunnel = TunnelUrl::try_from(TunnelEndpointDto {
            url: "https://wire.example/tunnel/".to_owned(),
        })
        .unwrap();

        assert_eq!(tunnel.as_str(), "https://wire.example/tunnel");
        assert_eq!(
            TunnelEndpointDto::from(tunnel),
            TunnelEndpointDto {
                url: "https://wire.example/tunnel".to_owned()
            }
        );
    }
}
