//! Shared API state.

use std::sync::Arc;

use anylive_auth::MemoryAuthService;
use anylive_media::SrsMediaProvider;

use crate::rooms::MemoryRoomStore;

#[derive(Clone)]
pub struct AppState {
    pub auth: MemoryAuthService,
    pub rooms: MemoryRoomStore,
    pub media: SrsMediaProvider,
}

impl AppState {
    pub fn new(auth: MemoryAuthService, rooms: MemoryRoomStore, media: SrsMediaProvider) -> Self {
        Self {
            auth,
            rooms,
            media,
        }
    }

    pub fn dev() -> Arc<Self> {
        Arc::new(Self::new(
            MemoryAuthService::memory_dev(),
            MemoryRoomStore::new(),
            SrsMediaProvider::from_env(),
        ))
    }
}
