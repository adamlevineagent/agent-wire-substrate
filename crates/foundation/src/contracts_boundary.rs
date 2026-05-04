use serde::{Deserialize, Serialize};

use agent_wire_contracts::{HandlePathDto, MoneyAmountDto, TunnelEndpointDto, WireDto};

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
pub struct WrappedContract<T: WireDto> {
    boundary: BoundaryName,
    inner: T,
}

impl<T: WireDto> WrappedContract<T> {
    pub fn wrap_contract(boundary: BoundaryName, inner: T) -> Self {
        Self { boundary, inner }
    }

    pub fn boundary(&self) -> &BoundaryName {
        &self.boundary
    }

    #[cfg(test)]
    pub(crate) fn as_inner(&self) -> &T {
        &self.inner
    }

    fn into_inner(self) -> T {
        self.inner
    }
}

pub trait FromContractDto<Dto: WireDto>: Sized {
    type Error;

    fn from_contract_dto(dto: WrappedContract<Dto>) -> Result<Self, Self::Error>;
}

pub trait IntoContractDto<Dto: WireDto> {
    type Error;

    fn into_contract_dto(self, boundary: BoundaryName)
        -> Result<WrappedContract<Dto>, Self::Error>;
}

impl FromContractDto<HandlePathDto> for CrossGraphRef {
    type Error = FoundationError;

    fn from_contract_dto(dto: WrappedContract<HandlePathDto>) -> Result<Self, Self::Error> {
        let value = dto.into_inner();
        Ok(Self {
            namespace: NamespaceId::new(value.handle)?,
            day: value.wire_day,
            slug: value.graph_slug,
            sequence: value.sequence,
        })
    }
}

impl IntoContractDto<HandlePathDto> for CrossGraphRef {
    type Error = FoundationError;

    fn into_contract_dto(
        self,
        boundary: BoundaryName,
    ) -> Result<WrappedContract<HandlePathDto>, Self::Error> {
        Ok(WrappedContract::wrap_contract(
            boundary,
            HandlePathDto {
                handle: self.namespace.as_str().to_owned(),
                wire_day: self.day,
                graph_slug: self.slug,
                sequence: self.sequence,
            },
        ))
    }
}

impl FromContractDto<MoneyAmountDto> for CreditAmount {
    type Error = FoundationError;

    fn from_contract_dto(dto: WrappedContract<MoneyAmountDto>) -> Result<Self, Self::Error> {
        Ok(Self::from_sats(dto.into_inner().credits as u128))
    }
}

impl IntoContractDto<MoneyAmountDto> for CreditAmount {
    type Error = FoundationError;

    fn into_contract_dto(
        self,
        boundary: BoundaryName,
    ) -> Result<WrappedContract<MoneyAmountDto>, Self::Error> {
        let credits = u64::try_from(self.as_sats()).map_err(|_| FoundationError::OutOfRange {
            field: "money_amount",
        })?;
        Ok(WrappedContract::wrap_contract(
            boundary,
            MoneyAmountDto { credits },
        ))
    }
}

impl FromContractDto<TunnelEndpointDto> for TunnelUrl {
    type Error = FoundationError;

    fn from_contract_dto(dto: WrappedContract<TunnelEndpointDto>) -> Result<Self, Self::Error> {
        TunnelUrl::parse(&dto.into_inner().url)
    }
}

impl IntoContractDto<TunnelEndpointDto> for TunnelUrl {
    type Error = FoundationError;

    fn into_contract_dto(
        self,
        boundary: BoundaryName,
    ) -> Result<WrappedContract<TunnelEndpointDto>, Self::Error> {
        Ok(WrappedContract::wrap_contract(
            boundary,
            TunnelEndpointDto {
                url: self.as_str().to_owned(),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_keeps_boundary_visible() {
        let boundary = BoundaryName::new("transport").unwrap();
        let wrapped =
            WrappedContract::wrap_contract(boundary.clone(), MoneyAmountDto { credits: 42 });

        assert_eq!(wrapped.boundary(), &boundary);
        assert_eq!(wrapped.as_inner().credits, 42);
        assert_eq!(wrapped.into_inner(), MoneyAmountDto { credits: 42 });
    }

    #[test]
    fn converts_handle_path_dto_to_foundation_ref() {
        let boundary = BoundaryName::new("identity").unwrap();
        let dto = HandlePathDto {
            handle: "playful".to_owned(),
            wire_day: 122,
            graph_slug: Some("substrate".to_owned()),
            sequence: 7,
        };

        let runtime = CrossGraphRef::from_contract_dto(WrappedContract::wrap_contract(
            boundary.clone(),
            dto.clone(),
        ))
        .unwrap();
        assert_eq!(runtime.to_string(), "playful/122/substrate/7");
        assert_eq!(
            runtime.into_contract_dto(boundary).unwrap().into_inner(),
            dto
        );
    }

    #[test]
    fn rejects_invalid_handle_path_dto_at_boundary() {
        let boundary = BoundaryName::new("identity").unwrap();
        let dto = HandlePathDto {
            handle: "bad handle".to_owned(),
            wire_day: 122,
            graph_slug: None,
            sequence: 7,
        };

        assert_eq!(
            CrossGraphRef::from_contract_dto(WrappedContract::wrap_contract(boundary, dto)),
            Err(FoundationError::InvalidCharacter { field: "namespace" })
        );
    }

    #[test]
    fn converts_money_dto_without_leaking_contract_shape() {
        let boundary = BoundaryName::new("economics").unwrap();
        let amount = CreditAmount::from_contract_dto(WrappedContract::wrap_contract(
            boundary.clone(),
            MoneyAmountDto { credits: 42 },
        ))
        .unwrap();

        assert_eq!(amount.as_sats(), 42);
        assert_eq!(
            amount.into_contract_dto(boundary).unwrap().into_inner(),
            MoneyAmountDto { credits: 42 }
        );
    }

    #[test]
    fn converts_tunnel_endpoint_dto_with_runtime_validation() {
        let boundary = BoundaryName::new("transport").unwrap();
        let tunnel = TunnelUrl::from_contract_dto(WrappedContract::wrap_contract(
            boundary.clone(),
            TunnelEndpointDto {
                url: "https://wire.example/tunnel/".to_owned(),
            },
        ))
        .unwrap();

        assert_eq!(tunnel.as_str(), "https://wire.example/tunnel");
        assert_eq!(
            tunnel.into_contract_dto(boundary).unwrap().into_inner(),
            TunnelEndpointDto {
                url: "https://wire.example/tunnel".to_owned()
            }
        );
    }
}
