//! Route modules.

pub mod admin;
pub mod auth;
pub mod chat;
pub mod feed;
pub mod reports;
pub mod rooms;
pub mod social;
pub mod system;
pub mod wallet;

pub use admin::*;
pub use auth::*;
pub use chat::*;
pub use feed::*;
pub use reports::create_report;
pub use rooms::*;
pub use social::*;
pub use system::*;
pub use wallet::*;
