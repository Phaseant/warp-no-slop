mod local_auth_client;
#[path = "local_session.rs"]
mod session;
use std::fmt;
use std::result::Result as StdResult;

use anyhow::Result;
use async_trait::async_trait;
use cloud_objects::ids::ServerId;
use firebase::FirebaseError;
use instant::Duration;
pub use local_auth_client::AuthClientImpl;
#[cfg(any(test, feature = "test-util"))]
use mockall::automock;
pub use session::*;
use thiserror::Error;
pub use user_uid::{TEST_USER_EMAIL, TEST_USER_UID, UserUid};
use warp_errors::{AnyhowErrorExt, ErrorExt, register_error};
use warp_graphql::mutations::create_anonymous_user::{
    AnonymousUserType, CreateAnonymousUserResult,
};
use warp_graphql::mutations::expire_api_key::ExpireApiKeyResult;
use warp_graphql::mutations::generate_api_key::GenerateApiKeyResult;
use warp_graphql::mutations::mint_custom_token::MintCustomTokenResult;
use warp_graphql::mutations::update_user_settings::UpdateUserSettingsInput;
use warp_graphql::queries::api_keys::ApiKeyProperties;
use warp_graphql::queries::get_user::UserOutput as GqlUserOutput;
use warp_server_auth::credentials::{AuthToken, Credentials, FirebaseToken, LoginToken};
pub use warp_server_auth::user_uid;

use crate::ids::ApiKeyUid;

/// Authentication and authenticated-transport conditions observed by shared client code.
#[derive(Clone)]
pub enum AuthEvent {
    /// A staging API call was blocked, which may indicate a firewall misconfiguration.
    StagingAccessBlocked,
    /// The user's access token was invalid, so they need to reauthenticate.
    NeedsReauth,
    /// The user's account has been disabled.
    UserAccountDisabled,
    /// The current bearer token was refreshed.
    AccessTokenRefreshed {
        #[cfg_attr(target_family = "wasm", allow(dead_code))]
        token: String,
    },
    /// An Identity-Aware Proxy challenge was received.
    IapChallengeReceived,
}

impl fmt::Debug for AuthEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StagingAccessBlocked => f.write_str("StagingAccessBlocked"),
            Self::NeedsReauth => f.write_str("NeedsReauth"),
            Self::UserAccountDisabled => f.write_str("UserAccountDisabled"),
            Self::AccessTokenRefreshed { .. } => f
                .debug_struct("AccessTokenRefreshed")
                .field("token", &"<redacted>")
                .finish(),
            Self::IapChallengeReceived => f.write_str("IapChallengeReceived"),
        }
    }
}

/// Header key used to associate unauthenticated requests with an experiment identity.
pub const EXPERIMENT_ID_HEADER: &str = "X-Warp-Experiment-Id";

/// A named agent identity from the public API.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct AgentIdentity {
    pub uid: String,
    pub name: String,
    pub available: bool,
}

/// User settings that are stored server-side on a per-user basis.
#[derive(Copy, Clone, Debug, Default)]
pub struct SyncedUserSettings {
    pub is_cloud_conversation_storage_enabled: bool,
    pub is_crash_reporting_enabled: bool,
    pub is_telemetry_enabled: bool,
}

/// Protocol-level results of fetching the current user.
pub struct FetchUserResult {
    pub user_output: GqlUserOutput,
    /// The credentials used to authenticate this user.
    pub credentials: Credentials,
    /// Whether this attempt to fetch the user was for refreshing an existing logged-in user.
    pub from_refresh: bool,
}

#[cfg_attr(any(test, feature = "test-util"), automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait AuthClient: Send + Sync {
    /// Creates an anonymous user who is allowed to use Warp but may lack the ability
    /// to interact with particular features.
    async fn create_anonymous_user(
        &self,
        referral_code: Option<String>,
        anonymous_user_type: AnonymousUserType,
    ) -> Result<CreateAnonymousUserResult>;

    /// Returns the cached access token if it is still valid.
    ///
    /// If it has expired, this fetches a new access token using the user's refresh
    /// token, caches it, and then returns it. It may return an auth mode that does
    /// not require an Authorization header, such as session cookies or test credentials.
    async fn get_or_refresh_access_token(&self) -> Result<AuthToken>;

    /// Fetches the user's metadata and authentication tokens.
    async fn fetch_user(
        &self,
        token: LoginToken,
        for_refresh: bool,
    ) -> StdResult<FetchUserResult, UserAuthenticationError>;

    /// Creates and fetches a new custom token for the current user from Firebase.
    ///
    /// This only works for anonymous users and surfaces an error if the user is not anonymous.
    async fn fetch_new_custom_token(&self) -> Result<MintCustomTokenResult>;

    /// Handles the response from [`Self::fetch_new_custom_token`] by returning the newly minted custom token.
    fn on_custom_token_fetched(
        &self,
        response: Result<MintCustomTokenResult>,
    ) -> Result<String, MintCustomTokenError>;

    /// Queries warp-server for a set of the currently logged-in user's fields.
    async fn fetch_user_properties<'a>(&self, auth_token: Option<&'a str>)
    -> Result<GqlUserOutput>;

    /// Returns the user's settings retrieved from the server, if any.
    ///
    /// The user may not have server-side settings if they onboarded before telemetry
    /// opt-out launched, have not logged in since the launch, and have never changed
    /// defaults for any setting in [`SyncedUserSettings`]. If the fetched settings
    /// object exists but is missing required fields, or if the request itself fails,
    /// this returns an error.
    async fn get_user_settings(&self) -> Result<Option<SyncedUserSettings>>;

    async fn set_is_telemetry_enabled(&self, value: bool) -> Result<()>;

    async fn set_is_crash_reporting_enabled(&self, value: bool) -> Result<()>;

    async fn set_is_cloud_conversation_storage_enabled(&self, value: bool) -> Result<()>;

    /// Sends a request to update the user's settings on the server with values in the given input.
    async fn update_user_settings(&self, input: UpdateUserSettingsInput) -> Result<()>;

    async fn set_user_is_onboarded(&self) -> Result<bool>;

    /// Requests a device authorization code from the server for headless CLI or SDK authentication.
    async fn request_device_code(
        &self,
    ) -> StdResult<oauth2::StandardDeviceAuthorizationResponse, UserAuthenticationError>;

    /// Waits for the request to be approved or rejected and exchanges it for a short-lived custom access token.
    async fn exchange_device_access_token(
        &self,
        details: &oauth2::StandardDeviceAuthorizationResponse,
        timeout: Duration,
    ) -> StdResult<FirebaseToken, UserAuthenticationError>;

    async fn list_api_keys(&self, team_uid: Option<ServerId>) -> Result<Vec<ApiKeyProperties>>;

    async fn create_api_key(
        &self,
        name: String,
        team_id: Option<cynic::Id>,
        agent_uid: Option<cynic::Id>,
        expires_at: Option<warp_graphql::scalars::Time>,
    ) -> Result<GenerateApiKeyResult>;

    async fn expire_api_key(&self, key_uid: &ApiKeyUid) -> Result<ExpireApiKeyResult>;

    /// Fetches the list of named agent identities for the user's team.
    async fn list_agent_identities(&self, team_uid: Option<ServerId>)
    -> Result<Vec<AgentIdentity>>;
}

/// Error type when retrieving a user and validating it against Firebase.
#[derive(Error, Debug)]
pub enum UserAuthenticationError {
    /// The user's refresh token is invalid, which can occur after the user changes
    /// a password for Google or GitHub authentication.
    #[error("Firebase returned a token error when fetching an ID token")]
    DeniedAccessToken(FirebaseError),
    /// The user's account is invalid, which can occur after the user requests
    /// account deletion under GDPR or CCPA.
    #[error("Firebase returned a user error when fetching an ID token")]
    UserAccountDisabled(FirebaseError),
    #[error("Invalid state parameter in auth redirect")]
    InvalidStateParameter,
    #[error("Missing state parameter in auth redirect")]
    MissingStateParameter,
    #[error("Timed out requesting a sign-in link after {attempts} attempts")]
    DeviceCodeRequestTimedOut { attempts: usize },
    #[error("unexpected error occurred when fetching an ID token: {0:#}")]
    Unexpected(#[from] anyhow::Error),
}

impl ErrorExt for UserAuthenticationError {
    fn is_actionable(&self) -> bool {
        match self {
            UserAuthenticationError::DeniedAccessToken(error) => {
                // If a request to our server failed because the user's refresh token
                // has expired, they should reauthenticate, but there is no value in
                // reporting this back to us.
                log::info!("ignoring denied access token error: {error:#}");
                false
            }
            UserAuthenticationError::UserAccountDisabled(error) => {
                // If the user's account is disabled, they cannot make requests.
                log::info!("ignoring user account disabled error: {error:#}");
                false
            }
            UserAuthenticationError::DeviceCodeRequestTimedOut { .. } => false,
            UserAuthenticationError::Unexpected(error) => error.is_actionable(),
            UserAuthenticationError::InvalidStateParameter
            | UserAuthenticationError::MissingStateParameter => {
                // These errors remain actionable because a surplus could indicate a problem in
                // the login flow, although an attempt to spoof the `state` variable is not actionable.
                true
            }
        }
    }
}
register_error!(UserAuthenticationError);

impl From<FirebaseError> for UserAuthenticationError {
    fn from(error: FirebaseError) -> Self {
        // These Firebase errors indicate that the user's token is in an errored state
        // and that the user likely just needs to log in again.
        const SOFT_ERRORS: &[&str] = &[
            "TOKEN_EXPIRED",
            "INVALID_REFRESH_TOKEN",
            "MISSING_REFRESH_TOKEN",
        ];
        // These Firebase errors indicate that the user's account is in an errored state
        // and that the user likely can no longer sign in with it.
        const HARD_ERRORS: &[&str] = &["USER_DISABLED", "USER_NOT_FOUND"];
        if SOFT_ERRORS.contains(&error.message.as_str()) {
            UserAuthenticationError::DeniedAccessToken(error)
        } else if HARD_ERRORS.contains(&error.message.as_str()) {
            UserAuthenticationError::UserAccountDisabled(error)
        } else {
            UserAuthenticationError::Unexpected(
                anyhow::Error::from(error)
                    .context("Failed to exchange refresh token with access token."),
            )
        }
    }
}

/// Error type when minting a new custom token for an anonymous user.
#[derive(Error, Debug)]
pub enum MintCustomTokenError {
    #[error("Received a user facing error: {0}")]
    UserFacingError(String),
    #[error("Failed to create new custom token with unknown error")]
    Unknown,
}
