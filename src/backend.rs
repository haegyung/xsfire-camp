use agent_client_protocol::{AuthMethod, SessionId};
use agent_client_protocol::{
    AuthenticateRequest, AuthenticateResponse, CancelNotification, Error, ListSessionsRequest,
    ListSessionsResponse, NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse,
    SetSessionConfigOptionRequest, SetSessionConfigOptionResponse, SetSessionModeRequest,
    SetSessionModeResponse, SetSessionModelRequest, SetSessionModelResponse,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    Codex,
    ClaudeCode,
    Gemini,
    Multi,
}

impl BackendKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "codex" => Some(Self::Codex),
            "claude-code" | "claude" => Some(Self::ClaudeCode),
            "gemini" | "gemini-cli" => Some(Self::Gemini),
            "multi" | "all" => Some(Self::Multi),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
            Self::Gemini => "gemini",
            Self::Multi => "multi",
        }
    }
}

#[async_trait::async_trait(?Send)]
pub trait BackendDriver {
    fn backend_kind(&self) -> BackendKind;

    fn supports_load_session(&self) -> bool {
        self.backend_kind() == BackendKind::Codex
    }

    fn auth_methods(&self) -> Vec<AuthMethod>;

    async fn authenticate(
        &self,
        request: AuthenticateRequest,
    ) -> Result<AuthenticateResponse, Error>;

    async fn new_session(&self, request: NewSessionRequest) -> Result<NewSessionResponse, Error>;

    async fn load_session(
        &self,
        request: agent_client_protocol::LoadSessionRequest,
    ) -> Result<agent_client_protocol::LoadSessionResponse, Error>;

    async fn list_sessions(
        &self,
        request: ListSessionsRequest,
    ) -> Result<ListSessionsResponse, Error>;

    async fn prompt(&self, request: PromptRequest) -> Result<PromptResponse, Error>;

    async fn cancel(&self, args: CancelNotification) -> Result<(), Error>;

    async fn set_session_mode(
        &self,
        args: SetSessionModeRequest,
    ) -> Result<SetSessionModeResponse, Error>;

    async fn set_session_model(
        &self,
        args: SetSessionModelRequest,
    ) -> Result<SetSessionModelResponse, Error>;

    async fn set_session_config_option(
        &self,
        args: SetSessionConfigOptionRequest,
    ) -> Result<SetSessionConfigOptionResponse, Error>;

    fn session_id_display_hint(&self, session_id: &SessionId) -> String {
        // Helpful for unsupported drivers to reuse stable messaging.
        session_id.0.to_string()
    }
}

pub struct UnsupportedBackendDriver {
    backend: BackendKind,
}

impl UnsupportedBackendDriver {
    pub fn new(backend: BackendKind) -> Self {
        Self { backend }
    }

    fn err(&self) -> Error {
        Error::invalid_params().data(format!(
            "backend not supported yet: {} (current: codex only).",
            self.backend.as_str()
        ))
    }
}

#[async_trait::async_trait(?Send)]
impl BackendDriver for UnsupportedBackendDriver {
    fn backend_kind(&self) -> BackendKind {
        self.backend
    }

    fn auth_methods(&self) -> Vec<AuthMethod> {
        Vec::new()
    }

    async fn authenticate(
        &self,
        _request: AuthenticateRequest,
    ) -> Result<AuthenticateResponse, Error> {
        Err(self.err())
    }

    async fn new_session(&self, _request: NewSessionRequest) -> Result<NewSessionResponse, Error> {
        Err(self.err())
    }

    async fn load_session(
        &self,
        _request: agent_client_protocol::LoadSessionRequest,
    ) -> Result<agent_client_protocol::LoadSessionResponse, Error> {
        Err(self.err())
    }

    async fn list_sessions(
        &self,
        _request: ListSessionsRequest,
    ) -> Result<ListSessionsResponse, Error> {
        Err(self.err())
    }

    async fn prompt(&self, _request: PromptRequest) -> Result<PromptResponse, Error> {
        Err(self.err())
    }

    async fn cancel(&self, _args: CancelNotification) -> Result<(), Error> {
        Err(self.err())
    }

    async fn set_session_mode(
        &self,
        _args: SetSessionModeRequest,
    ) -> Result<SetSessionModeResponse, Error> {
        Err(self.err())
    }

    async fn set_session_model(
        &self,
        _args: SetSessionModelRequest,
    ) -> Result<SetSessionModelResponse, Error> {
        Err(self.err())
    }

    async fn set_session_config_option(
        &self,
        _args: SetSessionConfigOptionRequest,
    ) -> Result<SetSessionConfigOptionResponse, Error> {
        Err(self.err())
    }
}
