//! Shared API state.

use std::sync::Arc;

use anylive_auth::MemoryAuthService;

#[derive(Clone)]
pub struct AppState {
    pub auth: MemoryAuthService,
}

impl AppState {
    pub fn new(auth: MemoryAuthService) -> Self {
        Self { auth }
    }

    pub fn dev() -> Arc<Self> {
        Arc::new(Self::new(MemoryAuthService::memory_dev()))
    }
}
