//! The headless `warp-tui` front-end's app-side entry point.
//!
mod mcp;
mod user_info;

pub use mcp::{
    TuiMcpAction, TuiMcpConfigDiagnostic, TuiMcpFileScope, TuiMcpFileSource, TuiMcpInstallRequest,
    TuiMcpManager, TuiMcpManagerEvent, TuiMcpServerId, TuiMcpServerSnapshot, TuiMcpServerSource,
    TuiMcpServerStatus, TuiMcpSnapshot, TuiMcpSyncedTemplateProvenance, TuiMcpTemplateVariable,
    TuiMcpTransport, TuiMcpVariableValue,
};
pub use user_info::{TuiUserInfoManager, TuiUserInfoManagerEvent, TuiUserInfoSnapshot};
use warpui::AppContext;

use crate::TuiMountFn;
use crate::tui_onboarding_markers::TuiOnboardingMarkers;

pub(crate) fn init(mount: TuiMountFn, ctx: &mut AppContext) {
    ctx.add_singleton_model(TuiMcpManager::new);
    ctx.add_singleton_model(TuiUserInfoManager::new);
    ctx.add_singleton_model(TuiOnboardingMarkers::new);
    mount(ctx);
}
