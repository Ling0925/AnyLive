//! Route modules.

pub mod admin;
pub mod admin_ops;
pub mod admin_users;
pub mod auth;
pub mod avatar;
pub mod chat;
pub mod compliance;
pub mod creator;
pub mod feed;
pub mod interactive;
pub mod events;
pub mod pay;
pub mod presence;
pub mod push;
pub mod recording;
pub mod reports;
pub mod rooms;
pub mod search;
pub mod social;
pub mod system;
pub mod wallet;
pub mod webhooks;

pub use admin::*;
pub use admin_ops::*;
pub use admin_users::*;
pub use auth::*;
pub use avatar::*;
pub use chat::*;
pub use compliance::{delete_me, export_me, legal_privacy, legal_terms};
pub use creator::*;
pub use events::*;
pub use feed::*;
pub use interactive::*;
pub use pay::{
    create_pay_order, get_pay_order, list_pay_channels, list_pay_products,
    pay_webhook_epay, pay_webhook_iap, pay_webhook_jeepay, pay_webhook_mock,
    pay_webhook_stripe, pay_webhook_tokenpay, sandbox_complete_pay_order,
};
pub use presence::*;
pub use push::*;
pub use recording::*;
pub use reports::create_report;
pub use rooms::*;
pub use search::*;
pub use social::*;
pub use system::*;
pub use wallet::*;
pub use webhooks::*;
