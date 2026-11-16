//! TUI implementation of [`InputModePolicy`].

use warp::settings::AISettingsChangedEvent;
use warp::tui_export::{
    BlocklistAIInputModel, ConversationSelectionEvent, InputConfig, InputModePolicy, InputType,
    PolicyConfigUpdate,
};
use warpui_core::AppContext;

/// The TUI's agent config when autodetection is disabled or explicitly overridden.
pub(crate) const AI_LOCKED_CONFIG: InputConfig = InputConfig {
    input_type: InputType::AI,
    is_locked: true,
};
/// The config for `!` shell mode.
pub(crate) const SHELL_LOCKED_CONFIG: InputConfig = InputConfig {
    input_type: InputType::Shell,
    is_locked: true,
};
/// Whether the shared input mode is shell input, detected or explicitly locked.
/// The single definition of "in shell mode" for every TUI read site.
pub(crate) fn is_shell_mode(input_mode: &BlocklistAIInputModel) -> bool {
    input_mode.input_type() == InputType::Shell
}

pub(crate) struct TuiInputModePolicy;

impl InputModePolicy for TuiInputModePolicy {
    fn initial_config(&self, _: &AppContext) -> InputConfig {
        SHELL_LOCKED_CONFIG
    }

    fn allows_locked_ai_input(&self, _app: &AppContext) -> bool {
        false
    }

    fn is_autodetection_enabled(&self, _: &AppContext) -> bool {
        false
    }

    fn config_on_conversation_selection_changed(
        &self,
        _event: &ConversationSelectionEvent,
        _current: InputConfig,
        _app: &AppContext,
    ) -> Option<PolicyConfigUpdate> {
        None
    }

    fn config_on_ai_settings_changed(
        &self,
        _: &AISettingsChangedEvent,
        _: InputConfig,
        _: bool,
        _: &AppContext,
    ) -> Option<PolicyConfigUpdate> {
        None
    }
}
