use std::time::Instant;

use dashmap::DashMap;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::errors::AppError;
use crate::services::dashboard::{self, DashboardStats};

const CACHE_TTL_SECS: u64 = 30;

struct CachedStats {
    stats: DashboardStats,
    cached_at: Instant,
}

/// In-memory per-organization dashboard stats cache.
///
/// Avoids re-running the 5+ concurrent queries on every dashboard load.
/// Entries expire after 30 seconds and are lazily evicted on next access.
pub struct DashboardCache {
    cache: DashMap<Uuid, CachedStats>,
}

impl Default for DashboardCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardCache {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    /// Returns cached stats if fresh, otherwise queries the database and caches the result.
    pub async fn get_stats(
        &self,
        db: &DatabaseConnection,
        org_id: Uuid,
    ) -> Result<DashboardStats, AppError> {
        // Check cache
        if let Some(entry) = self.cache.get(&org_id) {
            if entry.cached_at.elapsed().as_secs() < CACHE_TTL_SECS {
                return Ok(entry.stats.clone());
            }
        }

        // Cache miss or expired — query fresh stats
        let stats = dashboard::get_stats(db, org_id).await?;

        self.cache.insert(
            org_id,
            CachedStats {
                stats: stats.clone(),
                cached_at: Instant::now(),
            },
        );

        Ok(stats)
    }

    /// Invalidate a specific organization's cached stats.
    /// Call after write operations that affect dashboard metrics.
    pub fn invalidate(&self, org_id: Uuid) {
        self.cache.remove(&org_id);
    }

    /// Remove all expired entries. Called periodically by a background task.
    pub fn cleanup_expired(&self) {
        self.cache
            .retain(|_, entry| entry.cached_at.elapsed().as_secs() < CACHE_TTL_SECS * 2);
    }
}
