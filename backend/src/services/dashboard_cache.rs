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

    pub async fn get_stats(
        &self,
        db: &DatabaseConnection,
        org_id: Uuid,
    ) -> Result<DashboardStats, AppError> {
        if let Some(entry) = self.cache.get(&org_id) {
            if entry.cached_at.elapsed().as_secs() < CACHE_TTL_SECS {
                return Ok(entry.stats.clone());
            }
        }

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

    pub fn invalidate(&self, org_id: Uuid) {
        self.cache.remove(&org_id);
    }

    pub fn cleanup_expired(&self) {
        self.cache
            .retain(|_, entry| entry.cached_at.elapsed().as_secs() < CACHE_TTL_SECS * 2);
    }
}
