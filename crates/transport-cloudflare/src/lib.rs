use agent_wire_foundation::{TransportDriver, TunnelUrl};

#[derive(Debug, Clone)]
pub struct CloudflareTunnelDriver {
    tunnel_url: Option<TunnelUrl>,
}

impl CloudflareTunnelDriver {
    pub fn new(tunnel_url: Option<TunnelUrl>) -> Self {
        Self { tunnel_url }
    }
}

impl TransportDriver for CloudflareTunnelDriver {
    fn driver_name(&self) -> &'static str {
        "cloudflare"
    }

    fn tunnel_url(&self) -> Option<TunnelUrl> {
        self.tunnel_url.clone()
    }
}
