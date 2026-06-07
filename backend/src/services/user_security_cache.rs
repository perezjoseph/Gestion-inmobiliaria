use chrono::{DateTime, Utc};
use dashmap::DashMap;
use sea_orm::{DatabaseConnection, EntityTrait};
use std::time::Instant;
use uuid::Uuid;

use crate::entities::usuario;
use crate::errors::AppError;

const CACHE_TTL_SECS: u64 = 60;

struct UserSecurityState {
    activo: bool,
    password_changed_at: DateTime<Utc>,
    cached_at: Instant,
}

pub struct UserSecurityCache {
    cache: DashMap<Uuid, UserSecurityState>,
}

impl Default for UserSecurityCache {
    fn default() -> Self {
        Self::new()
    }
}

impl UserSecurityCache {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    /// Check whether a token is still valid for the given user.
    ///
    /// Returns `false` if:
    /// - The user does not exist
    /// - The user is inactive (`activo = false`)
    /// - The token was issued before the user's last password change
    ///
    /// Results are cached for 60 seconds.
    pub async fn is_token_valid(
        &self,
        db: &DatabaseConnection,
        user_id: Uuid,
        iat: i64,
    ) -> Result<bool, AppError> {
        // Check cache first
        if let Some(entry) = self.cache.get(&user_id) {
            if entry.cached_at.elapsed().as_secs() < CACHE_TTL_SECS {
                return Ok(is_valid(&entry, iat));
            }
        }

        // Cache miss or expired — query DB
        let user = usuario::Entity::find_by_id(user_id).one(db).await?;

        let Some(user) = user else {
            // User not found — remove stale cache entry if any
            self.cache.remove(&user_id);
            return Ok(false);
        };

        let state = UserSecurityState {
            activo: user.activo,
            password_changed_at: user.password_changed_at.into(),
            cached_at: Instant::now(),
        };

        let valid = is_valid(&state, iat);
        self.cache.insert(user_id, state);

        Ok(valid)
    }

    /// Remove a user's cached state so the next check queries the DB.
    /// Call this on user deactivation or password change.
    pub fn invalidate(&self, user_id: Uuid) {
        self.cache.remove(&user_id);
    }
}

#[allow(clippy::missing_const_for_fn)]
fn is_valid(state: &UserSecurityState, iat: i64) -> bool {
    if !state.activo {
        return false;
    }
    // Token issued before password change → invalid
    if state.password_changed_at.timestamp() > iat {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_returns_false_when_inactive() {
        let state = UserSecurityState {
            activo: false,
            password_changed_at: Utc::now(),
            cached_at: Instant::now(),
        };
        assert!(!is_valid(&state, Utc::now().timestamp()));
    }

    #[test]
    fn is_valid_returns_false_when_password_changed_after_iat() {
        let state = UserSecurityState {
            activo: true,
            password_changed_at: Utc::now(),
            cached_at: Instant::now(),
        };
        // iat is 1 hour ago — password was changed after token issuance
        let iat = Utc::now().timestamp() - 3600;
        assert!(!is_valid(&state, iat));
    }

    #[test]
    fn is_valid_returns_true_when_active_and_token_after_password_change() {
        let state = UserSecurityState {
            activo: true,
            password_changed_at: DateTime::from_timestamp(1000, 0).unwrap_or_default(),
            cached_at: Instant::now(),
        };
        // iat is well after password_changed_at
        let iat = 2000;
        assert!(is_valid(&state, iat));
    }

    #[test]
    fn invalidate_removes_from_cache() {
        let cache = UserSecurityCache::new();
        let id = Uuid::new_v4();
        cache.cache.insert(
            id,
            UserSecurityState {
                activo: true,
                password_changed_at: Utc::now(),
                cached_at: Instant::now(),
            },
        );
        assert!(cache.cache.contains_key(&id));
        cache.invalidate(id);
        assert!(!cache.cache.contains_key(&id));
    }
}
