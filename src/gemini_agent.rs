use agent_client_protocol::{
    AuthMethod, AuthenticateRequest, AuthenticateResponse, CancelNotification, Error,
    ListSessionsRequest, ListSessionsResponse, LoadSessionRequest, LoadSessionResponse,
    NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse, SessionId, SessionInfo,
    SetSessionConfigOptionRequest, SetSessionConfigOptionResponse, SetSessionModeRequest,
    SetSessionModeResponse, SetSessionModelRequest, SetSessionModelResponse, StopReason,
};
use std::{cell::RefCell, collections::HashMap, path::PathBuf, process::Command, rc::Rc};
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    backend::{BackendDriver, BackendKind},
    cli_common::{prompt_blocks_to_text, send_agent_text},
};

struct GeminiSession {
    cwd: PathBuf,
    history: Vec<(String, String)>,
}

/// Gemini CLI backend driver (shells out to the `gemini` CLI).
///
/// Minimal implementation:
/// - `new_session` creates an in-memory session ID
/// - `prompt` runs `gemini --output-format text --approval-mode plan -p "<prompt>"`
///   to avoid interactive approvals and streams the response as a single ACP message chunk
pub struct GeminiCliDriver {
    sessions: Rc<RefCell<HashMap<SessionId, GeminiSession>>>,
}

impl GeminiCliDriver {
    pub fn new() -> Self {
        Self {
            sessions: Rc::default(),
        }
    }

    fn bin() -> String {
        std::env::var("XSFIRE_GEMINI_BIN").unwrap_or_else(|_| "gemini".to_string())
    }

    fn extra_args() -> Vec<String> {
        std::env::var("XSFIRE_GEMINI_ARGS")
            .ok()
            .and_then(|s| shlex::split(&s))
            .unwrap_or_default()
    }

    fn approval_mode() -> String {
        std::env::var("XSFIRE_GEMINI_APPROVAL_MODE").unwrap_or_else(|_| "plan".to_string())
    }

    async fn run_gemini(&self, cwd: PathBuf, prompt: String) -> Result<String, Error> {
        let bin = Self::bin();
        let bin_display = bin.clone();
        let extra_args = Self::extra_args();
        let approval_mode = Self::approval_mode();

        let output = tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new(&bin);
            cmd.current_dir(&cwd);
            cmd.arg("--output-format");
            cmd.arg("text");
            cmd.arg("--approval-mode");
            cmd.arg(approval_mode);
            cmd.args(extra_args);
            cmd.arg("--prompt");
            cmd.arg(prompt);
            cmd.output()
        })
        .await
        .map_err(|e| Error::internal_error().data(e.to_string()))?
        .map_err(|e| {
            Error::invalid_params().data(format!(
                "failed to execute Gemini CLI ({bin_display}). Install it or set XSFIRE_GEMINI_BIN. Error: {e}"
            ))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(Error::internal_error().data(format!(
                "Gemini CLI failed (exit {:?}). stderr:\n{stderr}",
                output.status.code()
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[async_trait::async_trait(?Send)]
impl BackendDriver for GeminiCliDriver {
    fn backend_kind(&self) -> BackendKind {
        BackendKind::Gemini
    }

    fn auth_methods(&self) -> Vec<AuthMethod> {
        vec![AuthMethod::new(
            "gemini-cli",
            "Gemini CLI (pre-authenticated)",
        )
        .description("Authenticate using the `gemini` CLI before starting. This adapter shells out to the CLI in non-interactive mode.")]
    }

    async fn authenticate(
        &self,
        _request: AuthenticateRequest,
    ) -> Result<AuthenticateResponse, Error> {
        Ok(AuthenticateResponse::new())
    }

    async fn new_session(&self, request: NewSessionRequest) -> Result<NewSessionResponse, Error> {
        let session_id = SessionId::new(format!("gemini:{}", Uuid::new_v4()));

        self.sessions.borrow_mut().insert(
            session_id.clone(),
            GeminiSession {
                cwd: request.cwd,
                history: Vec::new(),
            },
        );

        info!("Created Gemini session: {session_id:?}");
        Ok(NewSessionResponse::new(session_id))
    }

    async fn load_session(
        &self,
        _request: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, Error> {
        Err(Error::invalid_params().data(
            "load_session is not supported for --backend=gemini yet (sessions are in-memory).",
        ))
    }

    async fn list_sessions(
        &self,
        _request: ListSessionsRequest,
    ) -> Result<ListSessionsResponse, Error> {
        let sessions = self
            .sessions
            .borrow()
            .iter()
            .map(|(id, s)| SessionInfo::new(id.clone(), s.cwd.clone()).title("Gemini (in-memory)"))
            .collect::<Vec<_>>();
        Ok(ListSessionsResponse::new(sessions))
    }

    async fn prompt(&self, request: PromptRequest) -> Result<PromptResponse, Error> {
        let session_id = request.session_id.clone();
        let mut sessions = self.sessions.borrow_mut();
        let Some(session) = sessions.get_mut(&session_id) else {
            return Err(Error::resource_not_found(None));
        };

        let user_text = prompt_blocks_to_text(&request.prompt);
        debug!(
            "Gemini prompt (session={session_id:?}) chars={}",
            user_text.len()
        );

        let mut full_prompt = String::new();
        for (user, assistant) in session.history.iter().rev().take(6).rev() {
            full_prompt.push_str("User:\n");
            full_prompt.push_str(user);
            full_prompt.push_str("\n\nAssistant:\n");
            full_prompt.push_str(assistant);
            full_prompt.push_str("\n\n");
        }
        full_prompt.push_str("User:\n");
        full_prompt.push_str(&user_text);

        let output = self.run_gemini(session.cwd.clone(), full_prompt).await?;
        send_agent_text(&session_id, output.trim_end_matches('\n')).await;

        session
            .history
            .push((user_text, output.trim_end_matches('\n').to_string()));

        Ok(PromptResponse::new(StopReason::EndTurn))
    }

    async fn cancel(&self, _args: CancelNotification) -> Result<(), Error> {
        Ok(())
    }

    async fn set_session_mode(
        &self,
        _args: SetSessionModeRequest,
    ) -> Result<SetSessionModeResponse, Error> {
        Err(Error::invalid_params()
            .data("set_session_mode is not supported for --backend=gemini yet."))
    }

    async fn set_session_model(
        &self,
        _args: SetSessionModelRequest,
    ) -> Result<SetSessionModelResponse, Error> {
        Err(Error::invalid_params()
            .data("set_session_model is not supported for --backend=gemini yet."))
    }

    async fn set_session_config_option(
        &self,
        _args: SetSessionConfigOptionRequest,
    ) -> Result<SetSessionConfigOptionResponse, Error> {
        Err(Error::invalid_params()
            .data("set_session_config_option is not supported for --backend=gemini yet."))
    }
}
