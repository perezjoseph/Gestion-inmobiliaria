use dashmap::DashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::models::ocr::ImportPreview;

const TTL: Duration = Duration::from_secs(30 * 60);

pub struct PreviewStore {
    previews: DashMap<Uuid, (ImportPreview, Instant)>,
}

impl PreviewStore {
    pub fn new() -> Self {
        Self {
            previews: DashMap::new(),
        }
    }

    pub fn insert(&self, preview: ImportPreview) -> Uuid {
        let id = preview.preview_id;
        self.previews.insert(id, (preview, Instant::now()));
        id
    }

    pub fn get(&self, id: &Uuid) -> Option<ImportPreview> {
        self.previews.get(id).map(|entry| entry.0.clone())
    }

    pub fn remove(&self, id: &Uuid) -> Option<ImportPreview> {
        self.previews.remove(id).map(|(_, (preview, _))| preview)
    }

    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.previews
            .retain(|_, (_, inserted_at)| now.duration_since(*inserted_at) < TTL);
    }
}

impl Default for PreviewStore {
    fn default() -> Self {
        Self::new()
    }
}
