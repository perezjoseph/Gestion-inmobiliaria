use actix_governor::{KeyExtractor, SimpleKeyExtractionError};
use actix_web::dev::ServiceRequest;
use std::net::IpAddr;

/// A [`KeyExtractor`] that uses the real client IP for rate limiting.
///
/// When behind a reverse proxy (Traefik), `peer_addr()` returns the proxy's IP.
/// This extractor checks `X-Forwarded-For` first, falling back to `peer_addr()`.
/// For IPv6, the last 9 bytes are zeroed to group /56 subnets together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallbackPeerIpKeyExtractor;

impl KeyExtractor for FallbackPeerIpKeyExtractor {
    type Key = IpAddr;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        // Try X-Forwarded-For first (set by Traefik)
        let mut ip = req
            .headers()
            .get("X-Forwarded-For")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.trim().parse::<IpAddr>().ok())
            .unwrap_or_else(|| {
                req.peer_addr()
                    .map_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), |socket| {
                        socket.ip()
                    })
            });

        if let IpAddr::V6(ipv6) = ip {
            let mut octets = ipv6.octets();
            octets[7..16].fill(0);
            ip = IpAddr::V6(octets.into());
        }
        Ok(ip)
    }
}
