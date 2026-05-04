use agent_wire_foundation::{
    FoundationError, TransportDriver, TunnelRequest, TunnelSession, TunnelUrl,
};

#[derive(Debug, Clone, Default)]
pub struct CloudflareTunnelDriver {
    tunnel_url: Option<TunnelUrl>,
}

impl CloudflareTunnelDriver {
    pub fn new(tunnel_url: Option<TunnelUrl>) -> Self {
        Self { tunnel_url }
    }

    pub fn with_static_tunnel(tunnel_url: TunnelUrl) -> Self {
        Self {
            tunnel_url: Some(tunnel_url),
        }
    }
}

impl TransportDriver for CloudflareTunnelDriver {
    type Error = FoundationError;

    fn driver_name(&self) -> &'static str {
        "cloudflare"
    }

    fn tunnel_url(&self) -> Option<TunnelUrl> {
        self.tunnel_url.clone()
    }

    fn open_tunnel(&self, request: TunnelRequest) -> Result<TunnelSession, Self::Error> {
        let public_url = self
            .tunnel_url
            .clone()
            .or_else(|| request.requested_public_url.clone())
            .ok_or(FoundationError::EmptyField {
                field: "public_tunnel_url",
            })?;

        Ok(TunnelSession {
            driver_name: self.driver_name().to_owned(),
            public_url,
            local_endpoint: request.local_endpoint,
            callbacks: request.callbacks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_wire_foundation::{CallbackUrl, EndpointUrl};

    #[test]
    fn driver_opens_session_from_static_url() {
        let driver = CloudflareTunnelDriver::with_static_tunnel(
            TunnelUrl::parse("https://tunnel.example").unwrap(),
        );
        let request = TunnelRequest::new(EndpointUrl::parse("http://127.0.0.1:8787").unwrap())
            .with_callback(CallbackUrl::parse("https://node.example/callback").unwrap());

        let session = driver.open_tunnel(request).unwrap();

        assert_eq!(session.driver_name, "cloudflare");
        assert_eq!(session.public_url.as_str(), "https://tunnel.example");
        assert_eq!(session.callbacks.len(), 1);
    }

    #[test]
    fn driver_uses_requested_public_url_when_static_url_absent() {
        let driver = CloudflareTunnelDriver::default();
        let request = TunnelRequest::new(EndpointUrl::parse("http://127.0.0.1:8787").unwrap())
            .with_requested_public_url(TunnelUrl::parse("https://requested.example").unwrap());

        let session = driver.open_tunnel(request).unwrap();

        assert_eq!(session.public_url.as_str(), "https://requested.example");
    }

    #[test]
    fn driver_errors_without_any_public_url() {
        let driver = CloudflareTunnelDriver::default();
        let request = TunnelRequest::new(EndpointUrl::parse("http://127.0.0.1:8787").unwrap());

        assert_eq!(
            driver.open_tunnel(request),
            Err(FoundationError::EmptyField {
                field: "public_tunnel_url"
            })
        );
    }
}
