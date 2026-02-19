use agent_client_protocol::{
    AuthMethod, AuthenticateRequest, AuthenticateResponse, CancelNotification, Error,
    ListSessionsRequest, ListSessionsResponse, LoadSessionRequest, LoadSessionResponse,
    NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse, SessionConfigOption,
    SessionConfigOptionCategory, SessionConfigSelectOption, SessionId, SessionInfo,
    SetSessionConfigOptionRequest, SetSessionConfigOptionResponse, SetSessionModeRequest,
    SetSessionModeResponse, SetSessionModelRequest, SetSessionModelResponse, StopReason,
};
use std::sync::{Arc, Mutex};
use std::{cell::RefCell, collections::HashMap, path::PathBuf, process::Command, rc::Rc};
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    backend::{BackendDriver, BackendKind},
    cli_common::{prompt_blocks_to_text, send_agent_text},
    session_store::{GlobalSessionIndex, SessionStore},
};

struct ClaudeSession {
    cwd: PathBuf,
    model: Option<String>,
    history: Vec<(String, String)>,
    session_store: Option<SessionStore>,
}

/// Claude Code backend driver (shells out to the `claude` CLI).
///
/// This is intentionally minimal:
/// - `new_session` creates an in-memory session ID
/// - `prompt` runs `claude --print` and streams the response as a single ACP message chunk
pub struct ClaudeCodeDriver {
    sessions: Rc<RefCell<HashMap<SessionId, ClaudeSession>>>,
    global_session_index: Option<Arc<Mutex<GlobalSessionIndex>>>,
}

impl ClaudeCodeDriver {
    pub fn new() -> Self {
        Self {
            sessions: Rc::default(),
            global_session_index: GlobalSessionIndex::load().map(|idx| Arc::new(Mutex::new(idx))),
        }
    }

    fn bin() -> String {
        std::env::var("XSFIRE_CLAUDE_BIN").unwrap_or_else(|_| "claude".to_string())
    }

    fn extra_args() -> Vec<String> {
        std::env::var("XSFIRE_CLAUDE_ARGS")
            .ok()
            .and_then(|s| shlex::split(&s))
            .unwrap_or_default()
    }

    fn default_model() -> Option<String> {
        std::env::var("XSFIRE_CLAUDE_MODEL").ok()
    }

    fn config_options(current_model: Option<String>) -> Vec<SessionConfigOption> {
        let current_value = current_model.unwrap_or_else(|| "default".to_string());
        vec![
            SessionConfigOption::select(
                "model",
                "Model",
                current_value,
                vec![
                    SessionConfigSelectOption::new("default", "Default"),
                    SessionConfigSelectOption::new("opus", "Opus"),
                    SessionConfigSelectOption::new("sonnet", "Sonnet"),
                    SessionConfigSelectOption::new("haiku", "Haiku"),
                ],
            )
            .category(SessionConfigOptionCategory::Model)
            .description("Model used by Claude CLI for this session"),
        ]
    }

    async fn run_claude(
        &self,
        cwd: PathBuf,
        model: Option<String>,
        prompt: String,
    ) -> Result<String, Error> {
        let bin = Self::bin();
        let bin_display = bin.clone();
        let extra_args = Self::extra_args();

        let output = tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new(&bin);
            cmd.arg("--print");
            cmd.arg("--cwd");
            cmd.arg(&cwd);
            if let Some(model) = model {
                cmd.arg("--model");
                cmd.arg(model);
            }
            cmd.args(extra_args);
            cmd.arg(prompt);
            cmd.output()
        })
        .await
        .map_err(|e| Error::internal_error().data(e.to_string()))?
        .map_err(|e| {
            Error::invalid_params().data(format!(
                "failed to execute Claude CLI ({bin_display}). Install it or set XSFIRE_CLAUDE_BIN. Error: {e}"
            ))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(Error::internal_error().data(format!(
                "Claude CLI failed (exit {:?}). stderr:\n{stderr}",
                output.status.code()
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

enum ClaudeCommand {
    Help,
    Status,
    Reset,
    SetModel(String),
}

fn parse_claude_command(input: &str) -> Option<ClaudeCommand> {
    let trimmed = input.trim();
    if trimmed == "/help" {
        return Some(ClaudeCommand::Help);
    }
    if trimmed == "/status" {
        return Some(ClaudeCommand::Status);
    }
    if trimmed == "/reset" {
        return Some(ClaudeCommand::Reset);
    }
    if let Some(value) = trimmed.strip_prefix("/model ") {
        let model = value.trim();
        if !model.is_empty() {
            return Some(ClaudeCommand::SetModel(model.to_string()));
        }
    }
    None
}

#[async_trait::async_trait(?Send)]
impl BackendDriver for ClaudeCodeDriver {
    fn backend_kind(&self) -> BackendKind {
        BackendKind::ClaudeCode
    }

    fn auth_methods(&self) -> Vec<AuthMethod> {
        vec![AuthMethod::new(
            "claude-cli",
            "Claude CLI (pre-authenticated)",
        )
        .description("Authenticate using the `claude` CLI before starting. This adapter shells out to the CLI in non-interactive mode.")]
    }

    async fn authenticate(
        &self,
        _request: AuthenticateRequest,
    ) -> Result<AuthenticateResponse, Error> {
        // Best-effort: we cannot reliably start an interactive login flow from ACP.
        // Users should authenticate via the CLI itself.
        Ok(AuthenticateResponse::new())
    }

    async fn new_session(&self, request: NewSessionRequest) -> Result<NewSessionResponse, Error> {
        let session_id = SessionId::new(format!("claude:{}", Uuid::new_v4()));
        let cwd = request.cwd;
        let session_store = self.init_session_store(&session_id, &cwd);

        self.sessions.borrow_mut().insert(
            session_id.clone(),
            ClaudeSession {
                cwd,
                model: Self::default_model(),
                history: Vec::new(),
                session_store,
            },
        );

        info!("Created Claude session: {session_id:?}");
        let model = self
            .sessions
            .borrow()
            .get(&session_id)
            .and_then(|s| s.model.clone());
        Ok(NewSessionResponse::new(session_id).config_options(Self::config_options(model)))
    }

    async fn load_session(
        &self,
        _request: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, Error> {
        Err(Error::invalid_params().data(
            "load_session is not supported for --backend=claude-code yet (sessions are in-memory).",
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
            .map(|(id, s)| {
                SessionInfo::new(id.clone(), s.cwd.clone()).title("Claude Code (in-memory)")
            })
            .collect::<Vec<_>>();
        Ok(ListSessionsResponse::new(sessions))
    }

    async fn prompt(&self, request: PromptRequest) -> Result<PromptResponse, Error> {
        let session_id = request.session_id.clone();
        let user_text = prompt_blocks_to_text(&request.prompt);
        debug!(
            "Claude prompt (session={session_id:?}) chars={}",
            user_text.len()
        );

        {
            let sessions = self.sessions.borrow();
            if let Some(session) = sessions.get(&session_id)
                && let Some(store) = &session.session_store
            {
                store.log("acp.prompt", serde_json::json!({ "text": user_text }));
            }
        }

        if let Some(command) = parse_claude_command(&user_text) {
            let message = {
                let mut sessions = self.sessions.borrow_mut();
                let Some(session) = sessions.get_mut(&session_id) else {
                    return Err(Error::resource_not_found(None));
                };
                match command {
                    ClaudeCommand::Help => {
                        "Claude commands:\n- /status\n- /model <name>\n- /reset".to_string()
                    }
                    ClaudeCommand::Status => {
                        let model = session.model.as_deref().unwrap_or("default");
                        format!(
                            "Claude session status:\n- model: {model}\n- history_turns: {}",
                            session.history.len()
                        )
                    }
                    ClaudeCommand::Reset => {
                        session.history.clear();
                        "Claude session history has been reset.".to_string()
                    }
                    ClaudeCommand::SetModel(model) => {
                        let normalized = if model == "default" {
                            None
                        } else {
                            Some(model)
                        };
                        session.model = normalized.clone();
                        let model_text = normalized.as_deref().unwrap_or("default");
                        format!("Claude model set to `{model_text}`.")
                    }
                }
            };
            {
                let sessions = self.sessions.borrow();
                if let Some(session) = sessions.get(&session_id)
                    && let Some(store) = &session.session_store
                {
                    store.log(
                        "acp.agent_message_chunk",
                        serde_json::json!({ "text": message }),
                    );
                }
            }
            send_agent_text(&session_id, message).await;
            return Ok(PromptResponse::new(StopReason::EndTurn));
        }

        let (cwd, model, full_prompt) = {
            let sessions = self.sessions.borrow();
            let Some(session) = sessions.get(&session_id) else {
                return Err(Error::resource_not_found(None));
            };

            // Keep a short running transcript to preserve basic continuity.
            // If needed, users can override this behavior by embedding their own context in the prompt.
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

            (session.cwd.clone(), session.model.clone(), full_prompt)
        };

        let output = self.run_claude(cwd, model, full_prompt).await?;
        let output_text = output.trim_end_matches('\n').to_string();
        {
            let sessions = self.sessions.borrow();
            if let Some(session) = sessions.get(&session_id)
                && let Some(store) = &session.session_store
            {
                store.log(
                    "acp.agent_message_chunk",
                    serde_json::json!({ "text": output_text }),
                );
            }
        }
        send_agent_text(&session_id, &output_text).await;

        {
            let mut sessions = self.sessions.borrow_mut();
            let Some(session) = sessions.get_mut(&session_id) else {
                return Err(Error::resource_not_found(None));
            };
            session.history.push((user_text, output_text));
        }

        Ok(PromptResponse::new(StopReason::EndTurn))
    }

    async fn cancel(&self, _args: CancelNotification) -> Result<(), Error> {
        // Not implemented for the minimal driver. (No persistent process to kill.)
        Ok(())
    }

    async fn set_session_mode(
        &self,
        _args: SetSessionModeRequest,
    ) -> Result<SetSessionModeResponse, Error> {
        Err(Error::invalid_params()
            .data("set_session_mode is not supported for --backend=claude-code yet."))
    }

    async fn set_session_model(
        &self,
        args: SetSessionModelRequest,
    ) -> Result<SetSessionModelResponse, Error> {
        let mut sessions = self.sessions.borrow_mut();
        let Some(session) = sessions.get_mut(&args.session_id) else {
            return Err(Error::resource_not_found(None));
        };
        session.model = if args.model_id.0.as_ref() == "default" {
            None
        } else {
            Some(args.model_id.0.to_string())
        };
        Ok(SetSessionModelResponse::new())
    }

    async fn set_session_config_option(
        &self,
        args: SetSessionConfigOptionRequest,
    ) -> Result<SetSessionConfigOptionResponse, Error> {
        if args.config_id.0.as_ref() != "model" {
            return Err(Error::invalid_params().data(format!(
                "unsupported config option for claude backend: {}",
                args.config_id
            )));
        }
        let model = if args.value.0.as_ref() == "default" {
            None
        } else {
            Some(args.value.0.to_string())
        };
        let mut sessions = self.sessions.borrow_mut();
        let Some(session) = sessions.get_mut(&args.session_id) else {
            return Err(Error::resource_not_found(None));
        };
        session.model = model;
        Ok(SetSessionConfigOptionResponse::new(Self::config_options(
            session.model.clone(),
        )))
    }
}

impl ClaudeCodeDriver {
    fn init_session_store(&self, session_id: &SessionId, cwd: &PathBuf) -> Option<SessionStore> {
        let idx = self.global_session_index.as_ref()?;
        let global_id = idx
            .lock()
            .ok()
            .and_then(|mut i| i.get_or_create(&format!("claude:{}", session_id.0)))?;
        SessionStore::init(
            global_id,
            "claude-code",
            session_id.0.to_string(),
            session_id.0.to_string(),
            Some(cwd.as_path()),
        )
    }
}
