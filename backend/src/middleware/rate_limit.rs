use actix_governor::{KeyExtractor, SimpleKeyExtractionError};
use actix_web::dev::ServiceRequest;
use std::net::{IpAddr, Ipv4Addr};

/// A [`KeyExtractor`] that uses the real client IP for rate limiting.
///
/// Priority chain:
/// 1. `CF-Connecting-IP` — set by Cloudflare, cannot be spoofed by end users.
/// 2. `X-Real-Ip` — set by Traefik for local ingress.
/// 3. `peer_addr()` — socket-level fallback.
///
/// `X-Forwarded-For` is explicitly NOT trusted as it can be spoofed by clients.
///
/// For IPv6, the last 9 bytes are zeroed to group /56 subnets together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallbackPeerIpKeyExtractor;

impl KeyExtractor for FallbackPeerIpKeyExtractor {
    type Key = IpAddr;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        let ip = extract_client_ip(req);
        Ok(normalize_ipv6(ip))
    }
}

/// Extract the client IP from the request using the trusted header priority chain:
/// CF-Connecting-IP → X-Real-Ip → peer_addr.
///
/// This function is public so it can be reused for audit logging (Requirement 1.5).
pub fn extract_client_ip(req: &ServiceRequest) -> IpAddr {
    req.headers()
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<IpAddr>().ok())
        .or_else(|| {
            req.headers()
                .get("X-Real-Ip")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<IpAddr>().ok())
        })
        .unwrap_or_else(|| {
            req.peer_addr()
                .map_or(IpAddr::V4(Ipv4Addr::LOCALHOST), |s| s.ip())
        })
}

/// Normalize IPv6 addresses by zeroing the last 9 bytes to group /56 subnets.
fn normalize_ipv6(ip: IpAddr) -> IpAddr {
    if let IpAddr::V6(ipv6) = ip {
        let mut octets = ipv6.octets();
        octets[7..16].fill(0);
        IpAddr::V6(octets.into())
    } else {
        ip
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    // ── Header priority tests ──

    #[test]
    fn cf_connecting_ip_takes_highest_priority() {
        let sreq = TestRequest::default()
            .insert_header(("CF-Connecting-IP", "203.0.113.50"))
            .insert_header(("X-Real-Ip", "10.0.0.1"))
            .insert_header(("X-Forwarded-For", "192.168.1.1"))
            .peer_addr("172.16.0.1:1234".parse().unwrap())
            .to_srv_request();

        let ip = extract_client_ip(&sreq);
        assert_eq!(ip, "203.0.113.50".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn x_real_ip_used_when_cf_connecting_ip_absent() {
        let sreq = TestRequest::default()
            .insert_header(("X-Real-Ip", "10.0.0.1"))
            .insert_header(("X-Forwarded-For", "192.168.1.1"))
            .peer_addr("172.16.0.1:1234".parse().unwrap())
            .to_srv_request();

        let ip = extract_client_ip(&sreq);
        assert_eq!(ip, "10.0.0.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn peer_addr_used_when_no_trusted_headers() {
        let sreq = TestRequest::default()
            .insert_header(("X-Forwarded-For", "192.168.1.1"))
            .peer_addr("172.16.0.1:1234".parse().unwrap())
            .to_srv_request();

        let ip = extract_client_ip(&sreq);
        assert_eq!(ip, "172.16.0.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn x_forwarded_for_is_never_trusted() {
        // Even when X-Forwarded-For is present and no other headers exist,
        // the extractor should fall back to peer_addr, NOT use X-Forwarded-For.
        let sreq = TestRequest::default()
            .insert_header(("X-Forwarded-For", "1.2.3.4"))
            .peer_addr("10.10.10.10:5000".parse().unwrap())
            .to_srv_request();

        let ip = extract_client_ip(&sreq);
        assert_eq!(ip, "10.10.10.10".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn falls_back_to_localhost_when_nothing_available() {
        let sreq = TestRequest::default().to_srv_request();

        let ip = extract_client_ip(&sreq);
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::LOCALHOST));
    }

    #[test]
    fn invalid_cf_connecting_ip_falls_through_to_x_real_ip() {
        let sreq = TestRequest::default()
            .insert_header(("CF-Connecting-IP", "not-an-ip"))
            .insert_header(("X-Real-Ip", "10.0.0.5"))
            .to_srv_request();

        let ip = extract_client_ip(&sreq);
        assert_eq!(ip, "10.0.0.5".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn invalid_x_real_ip_falls_through_to_peer_addr() {
        let sreq = TestRequest::default()
            .insert_header(("X-Real-Ip", "garbage"))
            .peer_addr("192.168.0.1:8080".parse().unwrap())
            .to_srv_request();

        let ip = extract_client_ip(&sreq);
        assert_eq!(ip, "192.168.0.1".parse::<IpAddr>().unwrap());
    }

    // ── IPv6 normalization tests ──

    #[test]
    fn ipv6_is_normalized_to_56_subnet() {
        let sreq = TestRequest::default()
            .insert_header(("CF-Connecting-IP", "2001:db8:85a3:1234:5678:8a2e:0370:7334"))
            .to_srv_request();

        let extractor = FallbackPeerIpKeyExtractor;
        let key = extractor.extract(&sreq).unwrap();

        // octets: [20 01 0d b8 85 a3 12 34 56 78 8a 2e 03 70 73 34]
        // After zeroing [7..16]: [20 01 0d b8 85 a3 12 00 00 00 00 00 00 00 00 00]
        // = 2001:0db8:85a3:1200::
        let expected: IpAddr = "2001:db8:85a3:1200::".parse().unwrap();
        assert_eq!(key, expected);
    }

    #[test]
    fn ipv4_is_not_modified() {
        let sreq = TestRequest::default()
            .insert_header(("CF-Connecting-IP", "192.168.1.100"))
            .to_srv_request();

        let extractor = FallbackPeerIpKeyExtractor;
        let key = extractor.extract(&sreq).unwrap();
        assert_eq!(key, "192.168.1.100".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn extractor_uses_cf_connecting_ip_with_ipv6_normalization() {
        let sreq = TestRequest::default()
            .insert_header(("CF-Connecting-IP", "2001:db8::1"))
            .to_srv_request();

        let extractor = FallbackPeerIpKeyExtractor;
        let key = extractor.extract(&sreq).unwrap();

        // octets: [20 01 0d b8 00 00 00 00 00 00 00 00 00 00 00 01]
        // After zeroing [7..16]: [20 01 0d b8 00 00 00 00 00 00 00 00 00 00 00 00]
        // = 2001:db8::
        let expected: IpAddr = "2001:db8::".parse().unwrap();
        assert_eq!(key, expected);
    }
}
