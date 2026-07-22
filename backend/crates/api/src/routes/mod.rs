//! Route modules.

pub mod admin;
pub mod admin_ops;
pub mod auth;
pub mod chat;
pub mod compliance;
pub mod feed;
pub mod pay;
pub mod reports;
pub mod rooms;
pub mod social;
pub mod system;
pub mod wallet;
pub mod webhooks;

pub use admin::*;
pub use admin_ops::*;
pub use auth::*;
pub use chat::*;
pub use compliance::{delete_me, export_me, legal_privacy, legal_terms};
pub use feed::*;
pub use pay::{
    create_pay_order, get_pay_order, list_pay_channels, list_pay_products,
    pay_webhook_epay, pay_webhook_jeepay, pay_webhook_mock, pay_webhook_tokenpay,
    sandbox_complete_pay_order,
};
pub use reports::create_report;
pub use rooms::*;
pub use social::*;
pub use system::*;
pub use wallet::*;
pub use webhooks::*;
