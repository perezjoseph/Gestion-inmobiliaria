use actix_governor::{KeyExtractor, SimpleKeyExtractionError};
use actix_web::dev::ServiceRequest;
use std::net::IpAddr;

/// A [`KeyExtractor`] that uses peer IP as key, falling back to loopback
/// when `peer_addr()` is unavailable (e.g. in integration tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallbackPeerIpKeyExtractor;

impl KeyExtractor for FallbackPeerIpKeyExtractor {
    type Key = IpAddr;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        let mut ip = req
            .peer_addr()
            .map_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), |socket| socket.ip());
        if let IpAddr::V6(ipv6) = ip {
            let mut octets = ipv6.octets();
            octets[7..16].fill(0);
            ip = IpAddr::V6(octets.into());
        }
        Ok(ip)
    }
}
