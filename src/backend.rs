use agent_client_protocol::{AuthMethod, SessionId};
use agent_client_protocol::{
    AuthenticateRequest, AuthenticateResponse, CancelNotification, Error, ForkSessionRequest,
    ForkSessionResponse, ListSessionsRequest, ListSessionsResponse, NewSessionRequest,
    NewSessionResponse, PromptRequest, PromptResponse, ResumeSessionRequest, ResumeSessionResponse,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkOrchestrationProfile {
    pub display_name: &'static str,
    pub evidence_summary: &'static str,
    pub task_orchestration: &'static str,
    pub task_monitoring: &'static str,
    pub vector_checks: bool,
    pub preempt_on_new_prompt: bool,
    pub supports_live_plan_updates: bool,
    pub supports_live_tool_calls: bool,
    pub operator_hint: &'static str,
}

impl WorkOrchestrationProfile {
    pub const SEQUENCE: &'static str = "R->P->M->W->A";
    pub const GLOSSARY: &'static str = "R=Root, P=Phase, M=Milestone, W=Work package, A=Action";

    pub const fn vector_checks_value(self) -> &'static str {
        if self.vector_checks { "on" } else { "off" }
    }

    pub const fn preempt_value(self) -> &'static str {
        if self.preempt_on_new_prompt {
            "on"
        } else {
            "off"
        }
    }

    pub const fn bridge_summary(self) -> &'static str {
        if self.supports_live_plan_updates && self.supports_live_tool_calls {
            "live ACP plan/tool updates available"
        } else {
            "single ACP message chunk only"
        }
    }

    pub fn render_summary(self) -> String {
        format!(
            "Work orchestration profile: {}\n- Sequence: {} ({})\n- Evidence: {}\n- Defaults: orchestration={}, monitoring={}, vector_checks={}, preempt={}\n- ACP bridge: {}\n- Operator hint: {}",
            self.display_name,
            Self::SEQUENCE,
            Self::GLOSSARY,
            self.evidence_summary,
            self.task_orchestration,
            self.task_monitoring,
            self.vector_checks_value(),
            self.preempt_value(),
            self.bridge_summary(),
            self.operator_hint,
        )
    }
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

    pub const fn work_orchestration_profile(self) -> WorkOrchestrationProfile {
        match self {
            Self::Codex => WorkOrchestrationProfile {
                display_name: "Codex / ChatGPT",
                evidence_summary: "Full ACP parity: sessions, approvals, tool calls, and live plan updates.",
                task_orchestration: "parallel",
                task_monitoring: "auto",
                vector_checks: true,
                preempt_on_new_prompt: true,
                supports_live_plan_updates: true,
                supports_live_tool_calls: true,
                operator_hint: "Use for tool-heavy or concurrent turns; ACP can keep plan/tool state live.",
            },
            Self::ClaudeCode => WorkOrchestrationProfile {
                display_name: "Claude Code",
                evidence_summary: "`claude --print --cwd <cwd> <prompt>` returns one ACP text stream; live tool/approval bridging is not wired yet.",
                task_orchestration: "sequential",
                task_monitoring: "status-only",
                vector_checks: false,
                preempt_on_new_prompt: false,
                supports_live_plan_updates: false,
                supports_live_tool_calls: false,
                operator_hint: "Keep one bounded goal per turn, externalize Goal/Rubric/Next Action in the prompt, and use /status between iterations.",
            },
            Self::Gemini => WorkOrchestrationProfile {
                display_name: "Gemini CLI",
                evidence_summary: "`gemini --output-format text --approval-mode plan --prompt` returns one ACP text stream; live tool/approval bridging is not wired yet.",
                task_orchestration: "sequential",
                task_monitoring: "status-only",
                vector_checks: false,
                preempt_on_new_prompt: false,
                supports_live_plan_updates: false,
                supports_live_tool_calls: false,
                operator_hint: "Keep one bounded goal per turn, ask Gemini to echo Goal/Rubric/Next Action, and use /status between iterations.",
            },
            Self::Multi => WorkOrchestrationProfile {
                display_name: "Unified ACP Router",
                evidence_summary: "Routes one thread across Codex, Claude Code, or Gemini; the active backend decides the actual ACP affordances.",
                task_orchestration: "backend-specific",
                task_monitoring: "backend-specific",
                vector_checks: false,
                preempt_on_new_prompt: false,
                supports_live_plan_updates: false,
                supports_live_tool_calls: false,
                operator_hint: "Switch backends explicitly when the task shape changes so ACP can show the correct work-orchestration profile.",
            },
        }
    }
}

#[async_trait::async_trait(?Send)]
pub trait BackendDriver {
    fn backend_kind(&self) -> BackendKind;

    fn supports_load_session(&self) -> bool {
        self.backend_kind() == BackendKind::Codex
    }

    fn supports_fork_session(&self) -> bool {
        false
    }

    fn supports_resume_session(&self) -> bool {
        false
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

    async fn fork_session(
        &self,
        request: ForkSessionRequest,
    ) -> Result<ForkSessionResponse, Error> {
        drop(request);
        Err(Error::invalid_params().data(format!(
            "backend does not support session/fork: {}",
            self.backend_kind().as_str()
        )))
    }

    async fn resume_session(
        &self,
        request: ResumeSessionRequest,
    ) -> Result<ResumeSessionResponse, Error> {
        drop(request);
        Err(Error::invalid_params().data(format!(
            "backend does not support session/resume: {}",
            self.backend_kind().as_str()
        )))
    }

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

    async fn fork_session(
        &self,
        _request: ForkSessionRequest,
    ) -> Result<ForkSessionResponse, Error> {
        Err(self.err())
    }

    async fn resume_session(
        &self,
        _request: ResumeSessionRequest,
    ) -> Result<ResumeSessionResponse, Error> {
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

#[cfg(test)]
mod tests {
    use super::BackendKind;

    #[test]
    fn codex_profile_uses_live_acp_defaults() {
        let profile = BackendKind::Codex.work_orchestration_profile();

        assert_eq!(profile.task_orchestration, "parallel");
        assert_eq!(profile.task_monitoring, "auto");
        assert_eq!(profile.vector_checks_value(), "on");
        assert_eq!(profile.preempt_value(), "on");
        assert!(profile.supports_live_plan_updates);
        assert!(profile.supports_live_tool_calls);
        assert!(profile.render_summary().contains("R->P->M->W->A"));
    }

    #[test]
    fn claude_and_gemini_profiles_are_sequential_summary_only() {
        for backend in [BackendKind::ClaudeCode, BackendKind::Gemini] {
            let profile = backend.work_orchestration_profile();

            assert_eq!(profile.task_orchestration, "sequential");
            assert_eq!(profile.task_monitoring, "status-only");
            assert_eq!(profile.vector_checks_value(), "off");
            assert_eq!(profile.preempt_value(), "off");
            assert_eq!(profile.bridge_summary(), "single ACP message chunk only");
            assert!(profile.render_summary().contains("Goal/Rubric/Next Action"));
        }
    }
}
