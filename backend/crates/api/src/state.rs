//! Shared API state.

use std::sync::Arc;

use anylive_auth::MemoryAuthService;
use anylive_media::SrsMediaProvider;
use anylive_wallet::MemoryWallet;

use crate::rooms::MemoryRoomStore;

#[derive(Clone)]
pub struct AppState {
    pub auth: MemoryAuthService,
    pub rooms: MemoryRoomStore,
    pub media: SrsMediaProvider,
    pub wallet: MemoryWallet,
}

impl AppState {
    pub fn new(
        auth: MemoryAuthService,
        rooms: MemoryRoomStore,
        media: SrsMediaProvider,
        wallet: MemoryWallet,
    ) -> Self {
        Self {
            auth,
            rooms,
            media,
            wallet,
        }
    }

    pub fn dev() -> Arc<Self> {
        let wallet = MemoryWallet::new();
        Arc::new(Self::new(
            MemoryAuthService::memory_dev(),
            MemoryRoomStore::new(),
            SrsMediaProvider::from_env(),
            wallet,
        ))
    }

    /// Async init for tests/dev that need gift catalog.
    pub async fn dev_ready() -> Arc<Self> {
        let state = Self::dev();
        state.wallet.seed_default_gifts().await;
        state
    }
}
