#[path = "local_auth_manager.rs"]
pub mod auth_manager;
mod auth_manager_types;
mod auth_override_warning_body;
pub mod auth_override_warning_modal;
mod auth_view_body;
pub mod auth_view_modal;
mod auth_view_shared_helpers;
mod login_error_modal;
mod login_failure_notification;
pub mod login_slide;
pub mod needs_sso_link_view;
pub mod paste_auth_token_modal;
pub use warp_server_auth::{auth_state, credentials, user, user_uid};
#[cfg(target_family = "wasm")]
pub mod web_handoff;

pub use auth_manager::AuthManager;
pub use auth_state::AuthStateProvider;
pub use login_failure_notification::LoginFailureReason;
pub use user_uid::UserUid;
