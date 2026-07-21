//! Shared API state.

use std::sync::Arc;

use anylive_auth::MemoryAuthService;
use anylive_media::SrsMediaProvider;
use anylive_moderation::MemoryModeration;
use anylive_realtime::{CentrifugoConfig, MemoryChatBus};
use anylive_wallet::MemoryWallet;

use crate::rooms::MemoryRoomStore;

#[derive(Clone)]
pub struct AppState {
    pub auth: MemoryAuthService,
    pub rooms: MemoryRoomStore,
    pub media: SrsMediaProvider,
    pub wallet: MemoryWallet,
    pub chat: MemoryChatBus,
    pub centrifugo: CentrifugoConfig,
    pub moderation: MemoryModeration,
}

impl AppState {
    pub fn new(
        auth: MemoryAuthService,
        rooms: MemoryRoomStore,
        media: SrsMediaProvider,
        wallet: MemoryWallet,
        chat: MemoryChatBus,
        centrifugo: CentrifugoConfig,
        moderation: MemoryModeration,
    ) -> Self {
        Self {
            auth,
            rooms,
            media,
            wallet,
            chat,
            centrifugo,
            moderation,
        }
    }

    pub fn dev() -> Arc<Self> {
        Arc::new(Self::new(
            MemoryAuthService::memory_dev(),
            MemoryRoomStore::new(),
            SrsMediaProvider::from_env(),
            MemoryWallet::new(),
            MemoryChatBus::new(),
            CentrifugoConfig::default(),
            MemoryModeration::new(),
        ))
    }

    /// Async init for tests/dev that need gift catalog.
    pub async fn dev_ready() -> Arc<Self> {
        let state = Self::dev();
        state.wallet.seed_default_gifts().await;
        state
    }
}
