use serde::{Deserialize, Serialize};
use url::Url;

use crate::FoundationError;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EndpointUrl(Url);

impl EndpointUrl {
    pub fn parse(value: &str) -> Result<Self, FoundationError> {
        let url = parse_http_url("endpoint_url", value)?;
        Ok(Self(url))
    }

    pub fn as_url(&self) -> &Url {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallbackUrl(EndpointUrl);

impl CallbackUrl {
    pub fn parse(value: &str) -> Result<Self, FoundationError> {
        Ok(Self(EndpointUrl::parse(value)?))
    }

    pub fn as_url(&self) -> &Url {
        self.0.as_url()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TunnelUrl(String);

impl TunnelUrl {
    pub fn parse(value: &str) -> Result<Self, FoundationError> {
        let url = parse_http_url("tunnel_url", value)?;
        if url.query().is_some() || url.fragment().is_some() {
            return Err(FoundationError::InvalidFormat {
                field: "tunnel_url",
            });
        }
        let normalized = value.trim_end_matches('/').to_owned();
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicEndpoint {
    pub url: EndpointUrl,
    pub advertised_by: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelRequest {
    pub local_endpoint: EndpointUrl,
    pub requested_public_url: Option<TunnelUrl>,
    pub callbacks: Vec<CallbackUrl>,
}

impl TunnelRequest {
    pub fn new(local_endpoint: EndpointUrl) -> Self {
        Self {
            local_endpoint,
            requested_public_url: None,
            callbacks: Vec::new(),
        }
    }

    pub fn with_requested_public_url(mut self, public_url: TunnelUrl) -> Self {
        self.requested_public_url = Some(public_url);
        self
    }

    pub fn with_callback(mut self, callback: CallbackUrl) -> Self {
        self.callbacks.push(callback);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelSession {
    pub driver_name: String,
    pub public_url: TunnelUrl,
    pub local_endpoint: EndpointUrl,
    pub callbacks: Vec<CallbackUrl>,
}

pub trait TransportDriver {
    type Error;

    fn driver_name(&self) -> &'static str;
    fn tunnel_url(&self) -> Option<TunnelUrl>;
    fn open_tunnel(&self, request: TunnelRequest) -> Result<TunnelSession, Self::Error>;
}

fn parse_http_url(field: &'static str, value: &str) -> Result<Url, FoundationError> {
    let url = Url::parse(value).map_err(|_| FoundationError::InvalidFormat { field })?;
    match url.scheme() {
        "http" | "https" => Ok(url),
        _ => Err(FoundationError::UnsupportedScheme { field }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tunnel_url_normalizes_trailing_slash() {
        let parsed = TunnelUrl::parse("https://example.com/").unwrap();

        assert_eq!(parsed.as_str(), "https://example.com");
    }

    #[test]
    fn tunnel_url_rejects_non_http_scheme() {
        assert_eq!(
            TunnelUrl::parse("file:///tmp/socket"),
            Err(FoundationError::UnsupportedScheme {
                field: "tunnel_url"
            })
        );
    }

    #[test]
    fn tunnel_url_rejects_malformed_url() {
        assert_eq!(
            TunnelUrl::parse("https://"),
            Err(FoundationError::InvalidFormat {
                field: "tunnel_url"
            })
        );
    }

    #[test]
    fn tunnel_request_accumulates_callbacks() {
        let request = TunnelRequest::new(EndpointUrl::parse("http://127.0.0.1:8787").unwrap())
            .with_requested_public_url(TunnelUrl::parse("https://tunnel.example").unwrap())
            .with_callback(CallbackUrl::parse("https://example.com/callback").unwrap());

        assert_eq!(request.callbacks.len(), 1);
        assert_eq!(
            request.requested_public_url.unwrap().as_str(),
            "https://tunnel.example"
        );
    }
}
