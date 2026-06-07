use std::time::{Duration, Instant};

use dashmap::DashMap;

const MAX_FAILURES: u32 = 5;
const WINDOW_DURATION: Duration = Duration::from_secs(15 * 60);
const LOCKOUT_DURATION: Duration = Duration::from_secs(15 * 60);

/// Information about an active lockout returned to callers.
pub struct LockoutInfo {
    pub retry_after_seconds: u64,
}

struct LockoutEntry {
    count: u32,
    first_failure: Instant,
    locked_until: Option<Instant>,
}

/// In-memory per-email login lockout tracker.
///
/// After 5 failed login attempts within a 15-minute window the account is
/// locked for 15 minutes. A successful login resets the counter.
pub struct LoginLockout {
    entries: DashMap<String, LockoutEntry>,
}

impl LoginLockout {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    /// Returns `Ok(())` if login is allowed, or `Err(LockoutInfo)` if the
    /// account is currently locked.
    pub fn check(&self, email: &str) -> Result<(), LockoutInfo> {
        let now = Instant::now();

        if let Some(entry) = self.entries.get(email) {
            if let Some(locked_until) = entry.locked_until {
                if now < locked_until {
                    let remaining = locked_until.duration_since(now).as_secs();
                    return Err(LockoutInfo {
                        retry_after_seconds: remaining.max(1),
                    });
                }
            }
        }

        Ok(())
    }

    /// Record a failed login attempt. Returns `Some(LockoutInfo)` if this
    /// failure triggered a lockout.
    #[allow(clippy::significant_drop_tightening)]
    pub fn record_failure(&self, email: &str) -> Option<LockoutInfo> {
        let now = Instant::now();

        let mut entry = self
            .entries
            .entry(email.to_owned())
            .or_insert(LockoutEntry {
                count: 0,
                first_failure: now,
                locked_until: None,
            });

        // If the previous window has expired, reset the counter.
        if now.duration_since(entry.first_failure) > WINDOW_DURATION {
            entry.count = 0;
            entry.first_failure = now;
            entry.locked_until = None;
        }

        entry.count += 1;

        if entry.count >= MAX_FAILURES {
            let locked_until = now + LOCKOUT_DURATION;
            entry.locked_until = Some(locked_until);
            let remaining = locked_until.duration_since(now).as_secs();
            return Some(LockoutInfo {
                retry_after_seconds: remaining.max(1),
            });
        }

        None
    }

    /// Reset the failure counter after a successful login.
    pub fn record_success(&self, email: &str) {
        self.entries.remove(email);
    }

    /// Remove entries whose window and lockout have both expired.
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.entries.retain(|_email, entry| {
            // Keep entries that are still within their failure window or lockout.
            if let Some(locked_until) = entry.locked_until {
                return now < locked_until;
            }
            now.duration_since(entry.first_failure) <= WINDOW_DURATION
        });
    }
}

impl Default for LoginLockout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_increments_on_failure() {
        let lockout = LoginLockout::new();
        let email = "test@example.com";

        // First 4 failures should not trigger lockout.
        for _ in 0..4 {
            assert!(lockout.record_failure(email).is_none());
        }

        // Still allowed.
        assert!(lockout.check(email).is_ok());
    }

    #[test]
    fn lockout_triggers_at_threshold() {
        let lockout = LoginLockout::new();
        let email = "test@example.com";

        for _ in 0..4 {
            lockout.record_failure(email);
        }

        // The 5th failure should trigger lockout.
        let info = lockout.record_failure(email);
        assert!(info.is_some());
        let info = info.unwrap();
        assert!(info.retry_after_seconds > 0);

        // check() should now return an error.
        let result = lockout.check(email);
        assert!(result.is_err());
        let lock_info = result.unwrap_err();
        assert!(lock_info.retry_after_seconds > 0);
    }

    #[test]
    fn reset_on_success() {
        let lockout = LoginLockout::new();
        let email = "test@example.com";

        // Record some failures.
        for _ in 0..3 {
            lockout.record_failure(email);
        }

        // Successful login resets.
        lockout.record_success(email);

        // Can fail 4 more times without lockout.
        for _ in 0..4 {
            assert!(lockout.record_failure(email).is_none());
        }

        // 5th triggers lockout again.
        assert!(lockout.record_failure(email).is_some());
    }

    #[test]
    fn window_expiry_resets_counter() {
        let lockout = LoginLockout::new();
        let email = "test@example.com";

        // Manually insert an entry whose window has already expired.
        lockout.entries.insert(
            email.to_owned(),
            LockoutEntry {
                count: 4,
                first_failure: Instant::now() - Duration::from_secs(16 * 60),
                locked_until: None,
            },
        );

        // Next failure should reset the counter (window expired) so no lockout.
        assert!(lockout.record_failure(email).is_none());

        // Verify counter was reset to 1.
        let entry = lockout.entries.get(email).unwrap();
        assert_eq!(entry.count, 1);
    }

    #[test]
    fn lockout_expires_after_duration() {
        let lockout = LoginLockout::new();
        let email = "test@example.com";

        // Insert an entry whose lockout has already expired.
        lockout.entries.insert(
            email.to_owned(),
            LockoutEntry {
                count: 5,
                first_failure: Instant::now() - Duration::from_secs(20 * 60),
                locked_until: Some(Instant::now() - Duration::from_secs(1)),
            },
        );

        // Should be allowed again.
        assert!(lockout.check(email).is_ok());
    }

    #[test]
    fn cleanup_removes_expired_entries() {
        let lockout = LoginLockout::new();

        // Insert a stale entry (window expired, no lockout).
        lockout.entries.insert(
            "stale@example.com".to_owned(),
            LockoutEntry {
                count: 2,
                first_failure: Instant::now() - Duration::from_secs(20 * 60),
                locked_until: None,
            },
        );

        // Insert an active entry.
        lockout.entries.insert(
            "active@example.com".to_owned(),
            LockoutEntry {
                count: 2,
                first_failure: Instant::now(),
                locked_until: None,
            },
        );

        // Insert a locked-but-expired entry.
        lockout.entries.insert(
            "expired_lock@example.com".to_owned(),
            LockoutEntry {
                count: 5,
                first_failure: Instant::now() - Duration::from_secs(20 * 60),
                locked_until: Some(Instant::now() - Duration::from_secs(1)),
            },
        );

        lockout.cleanup();

        assert!(!lockout.entries.contains_key("stale@example.com"));
        assert!(lockout.entries.contains_key("active@example.com"));
        assert!(!lockout.entries.contains_key("expired_lock@example.com"));
    }

    #[test]
    fn different_emails_are_independent() {
        let lockout = LoginLockout::new();

        // Lock one email.
        for _ in 0..5 {
            lockout.record_failure("alice@example.com");
        }

        // The other email should be unaffected.
        assert!(lockout.check("bob@example.com").is_ok());
        assert!(lockout.check("alice@example.com").is_err());
    }
}
