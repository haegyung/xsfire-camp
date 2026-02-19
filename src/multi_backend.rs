use agent_client_protocol::{
    AuthMethod, AuthenticateRequest, AuthenticateResponse, CancelNotification, Error,
    ListSessionsRequest, ListSessionsResponse, LoadSessionRequest, LoadSessionResponse,
    NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse, SessionConfigOption,
    SessionConfigOptionCategory, SessionConfigSelectOption, SessionId, SessionInfo,
    SetSessionConfigOptionRequest, SetSessionConfigOptionResponse, SetSessionModeRequest,
    SetSessionModeResponse, SetSessionModelRequest, SetSessionModelResponse, StopReason,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use uuid::Uuid;

use crate::{
    backend::{BackendDriver, BackendKind},
    cli_common::{prompt_blocks_to_text, send_agent_text},
    register_session_alias,
};

struct RoutedSession {
    active_backend: BackendKind,
    backend_sessions: HashMap<BackendKind, SessionId>,
    backend_config_options: HashMap<BackendKind, Vec<SessionConfigOption>>,
    cwd: std::path::PathBuf,
    mcp_servers: Vec<agent_client_protocol::McpServer>,
    meta: Option<agent_client_protocol::Meta>,
}

pub struct MultiBackendDriver {
    codex: Rc<dyn BackendDriver>,
    claude: Rc<dyn BackendDriver>,
    gemini: Rc<dyn BackendDriver>,
    sessions: RefCell<HashMap<SessionId, RoutedSession>>,
}

impl MultiBackendDriver {
    pub fn new(
        codex: Rc<dyn BackendDriver>,
        claude: Rc<dyn BackendDriver>,
        gemini: Rc<dyn BackendDriver>,
    ) -> Self {
        Self {
            codex,
            claude,
            gemini,
            sessions: RefCell::new(HashMap::new()),
        }
    }

    fn default_backend() -> BackendKind {
        std::env::var("XSFIRE_DEFAULT_BACKEND")
            .ok()
            .and_then(|v| BackendKind::parse(v.trim()))
            .filter(|k| *k != BackendKind::Multi)
            .unwrap_or(BackendKind::Codex)
    }

    fn driver_for(&self, backend: BackendKind) -> Rc<dyn BackendDriver> {
        match backend {
            BackendKind::Codex => self.codex.clone(),
            BackendKind::ClaudeCode => self.claude.clone(),
            BackendKind::Gemini => self.gemini.clone(),
            BackendKind::Multi => self.codex.clone(),
        }
    }

    fn parse_backend_selector(raw: &str) -> Option<BackendKind> {
        let trimmed = raw.trim();
        let mut parts = trimmed.split_whitespace();
        let Some(cmd) = parts.next() else {
            return None;
        };
        if cmd != "/backend" {
            return None;
        }
        parts.next().and_then(BackendKind::parse)
    }

    fn is_switch_backend_command(raw: &str) -> bool {
        raw.trim().starts_with("/backend")
    }

    fn backend_config_option(active_backend: BackendKind) -> SessionConfigOption {
        SessionConfigOption::select(
            "backend",
            "Backend",
            active_backend.as_str(),
            vec![
                SessionConfigSelectOption::new("codex", "Codex"),
                SessionConfigSelectOption::new("claude-code", "Claude Code"),
                SessionConfigSelectOption::new("gemini", "Gemini"),
            ],
        )
        .category(SessionConfigOptionCategory::Other)
        .description("Choose which backend handles this thread")
    }

    fn with_backend_option(
        existing: Option<Vec<SessionConfigOption>>,
        active_backend: BackendKind,
    ) -> Vec<SessionConfigOption> {
        let mut options = existing.unwrap_or_default();
        options.retain(|opt| opt.id.0.as_ref() != "backend");
        options.push(Self::backend_config_option(active_backend));
        options
    }

    fn merge_active_options(
        active_backend: BackendKind,
        mut options: Vec<SessionConfigOption>,
    ) -> Vec<SessionConfigOption> {
        options.retain(|opt| opt.id.0.as_ref() != "backend");
        options.push(Self::backend_config_option(active_backend));
        options
    }

    async fn ensure_backend_session(
        &self,
        session_id: &SessionId,
        backend: BackendKind,
    ) -> Result<SessionId, Error> {
        if let Some(existing) = self
            .sessions
            .borrow()
            .get(session_id)
            .and_then(|s| s.backend_sessions.get(&backend))
            .cloned()
        {
            return Ok(existing);
        }

        let (cwd, mcp_servers, meta) = {
            let sessions = self.sessions.borrow();
            let Some(session) = sessions.get(session_id) else {
                return Err(Error::resource_not_found(None));
            };
            (
                session.cwd.clone(),
                session.mcp_servers.clone(),
                session.meta.clone(),
            )
        };

        let request = NewSessionRequest::new(cwd)
            .mcp_servers(mcp_servers)
            .meta(meta.clone());
        let response = self.driver_for(backend).new_session(request).await?;
        let child = response.session_id;
        let child_options = response.config_options;

        {
            let mut sessions = self.sessions.borrow_mut();
            let Some(session) = sessions.get_mut(session_id) else {
                return Err(Error::resource_not_found(None));
            };
            session.backend_sessions.insert(backend, child.clone());
            session
                .backend_config_options
                .insert(backend, child_options.unwrap_or_default());
        }

        register_session_alias(&child, session_id);
        Ok(child)
    }

    fn resolve_routed(
        &self,
        session_id: &SessionId,
    ) -> Result<(BackendKind, Rc<dyn BackendDriver>, SessionId), Error> {
        let sessions = self.sessions.borrow();
        let Some(route) = sessions.get(session_id) else {
            return Err(Error::resource_not_found(None));
        };
        let backend = route.active_backend;
        let Some(child) = route.backend_sessions.get(&backend).cloned() else {
            return Err(Error::resource_not_found(None));
        };
        Ok((backend, self.driver_for(backend), child))
    }
}

#[async_trait::async_trait(?Send)]
impl BackendDriver for MultiBackendDriver {
    fn backend_kind(&self) -> BackendKind {
        BackendKind::Multi
    }

    fn supports_load_session(&self) -> bool {
        true
    }

    fn auth_methods(&self) -> Vec<AuthMethod> {
        let mut out = Vec::new();
        out.extend(self.codex.auth_methods());
        out.extend(self.claude.auth_methods());
        out.extend(self.gemini.auth_methods());
        out
    }

    async fn authenticate(
        &self,
        request: AuthenticateRequest,
    ) -> Result<AuthenticateResponse, Error> {
        let method = request.method_id.to_string();
        if matches!(
            method.as_str(),
            "chatgpt" | "codex-api-key" | "openai-api-key"
        ) {
            return self.codex.authenticate(request).await;
        }
        if method == "claude-cli" {
            return self.claude.authenticate(request).await;
        }
        if method == "gemini-cli" {
            return self.gemini.authenticate(request).await;
        }
        Err(Error::invalid_params().data(format!("unsupported auth method: {method}")))
    }

    async fn new_session(&self, request: NewSessionRequest) -> Result<NewSessionResponse, Error> {
        let backend = Self::default_backend();
        let child_response = self
            .driver_for(backend)
            .new_session(request.clone())
            .await?;
        let child_session_id = child_response.session_id.clone();

        let routed_session_id = SessionId::new(format!("multi:{}", Uuid::new_v4()));
        let mut backend_sessions = HashMap::new();
        backend_sessions.insert(backend, child_session_id.clone());
        let child_backend_options = child_response.config_options.clone().unwrap_or_default();
        let mut backend_config_options = HashMap::new();
        backend_config_options.insert(backend, child_backend_options.clone());

        self.sessions.borrow_mut().insert(
            routed_session_id.clone(),
            RoutedSession {
                active_backend: backend,
                backend_sessions,
                backend_config_options,
                cwd: request.cwd,
                mcp_servers: request.mcp_servers,
                meta: request.meta,
            },
        );
        register_session_alias(&child_session_id, &routed_session_id);

        let mut response = NewSessionResponse::new(routed_session_id);
        response.modes = child_response.modes;
        response.models = child_response.models;
        response.config_options = Some(Self::merge_active_options(backend, child_backend_options));
        response.meta = child_response.meta;
        Ok(response)
    }

    async fn load_session(
        &self,
        request: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, Error> {
        let mut response = self.codex.load_session(request.clone()).await?;
        let mut backend_sessions = HashMap::new();
        backend_sessions.insert(BackendKind::Codex, request.session_id.clone());
        let mut backend_config_options = HashMap::new();
        backend_config_options.insert(
            BackendKind::Codex,
            response.config_options.clone().unwrap_or_default(),
        );
        self.sessions.borrow_mut().insert(
            request.session_id.clone(),
            RoutedSession {
                active_backend: BackendKind::Codex,
                backend_sessions,
                backend_config_options,
                cwd: request.cwd,
                mcp_servers: request.mcp_servers,
                meta: request.meta,
            },
        );
        response.config_options = Some(Self::with_backend_option(
            response.config_options,
            BackendKind::Codex,
        ));
        Ok(response)
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
                SessionInfo::new(id.clone(), s.cwd.clone())
                    .title(format!("Unified session [{}]", s.active_backend.as_str()))
            })
            .collect::<Vec<_>>();
        Ok(ListSessionsResponse::new(sessions))
    }

    async fn prompt(&self, request: PromptRequest) -> Result<PromptResponse, Error> {
        let session_id = request.session_id.clone();
        let prompt_text = prompt_blocks_to_text(&request.prompt);

        if let Some(target_backend) = Self::parse_backend_selector(&prompt_text) {
            self.ensure_backend_session(&session_id, target_backend)
                .await?;
            {
                let mut sessions = self.sessions.borrow_mut();
                let Some(session) = sessions.get_mut(&session_id) else {
                    return Err(Error::resource_not_found(None));
                };
                session.active_backend = target_backend;
            }

            send_agent_text(
                &session_id,
                format!(
                    "Switched backend to `{}` for this thread.",
                    target_backend.as_str()
                ),
            )
            .await;
            return Ok(PromptResponse::new(StopReason::EndTurn));
        }

        if Self::is_switch_backend_command(&prompt_text) {
            send_agent_text(&session_id, "Usage: /backend <codex|claude-code|gemini>").await;
            return Ok(PromptResponse::new(StopReason::EndTurn));
        }

        let (backend, driver, child_session_id) = match self.resolve_routed(&session_id) {
            Ok(v) => v,
            Err(_) => {
                let target_backend = Self::default_backend();
                self.ensure_backend_session(&session_id, target_backend)
                    .await?;
                {
                    let mut sessions = self.sessions.borrow_mut();
                    let Some(session) = sessions.get_mut(&session_id) else {
                        return Err(Error::resource_not_found(None));
                    };
                    session.active_backend = target_backend;
                }
                self.resolve_routed(&session_id)?
            }
        };

        let mut routed = request;
        routed.session_id = child_session_id;
        let _ = backend; // Reserved for backend-specific behavior.
        driver.prompt(routed).await
    }

    async fn cancel(&self, args: CancelNotification) -> Result<(), Error> {
        let (_, driver, child_session_id) = self.resolve_routed(&args.session_id)?;
        let mut routed = args;
        routed.session_id = child_session_id;
        driver.cancel(routed).await
    }

    async fn set_session_mode(
        &self,
        args: SetSessionModeRequest,
    ) -> Result<SetSessionModeResponse, Error> {
        let (_, driver, child_session_id) = self.resolve_routed(&args.session_id)?;
        let mut routed = args;
        routed.session_id = child_session_id;
        driver.set_session_mode(routed).await
    }

    async fn set_session_model(
        &self,
        args: SetSessionModelRequest,
    ) -> Result<SetSessionModelResponse, Error> {
        let (_, driver, child_session_id) = self.resolve_routed(&args.session_id)?;
        let mut routed = args;
        routed.session_id = child_session_id;
        driver.set_session_model(routed).await
    }

    async fn set_session_config_option(
        &self,
        args: SetSessionConfigOptionRequest,
    ) -> Result<SetSessionConfigOptionResponse, Error> {
        if args.config_id.0.as_ref() == "backend" {
            let target_backend = BackendKind::parse(args.value.0.as_ref()).ok_or_else(|| {
                Error::invalid_params().data("backend must be one of: codex|claude-code|gemini")
            })?;
            if target_backend == BackendKind::Multi {
                return Err(Error::invalid_params()
                    .data("backend must be one of: codex|claude-code|gemini"));
            }

            self.ensure_backend_session(&args.session_id, target_backend)
                .await?;
            let merged_options = {
                let sessions = self.sessions.borrow();
                let Some(session) = sessions.get(&args.session_id) else {
                    return Err(Error::resource_not_found(None));
                };
                Self::merge_active_options(
                    target_backend,
                    session
                        .backend_config_options
                        .get(&target_backend)
                        .cloned()
                        .unwrap_or_default(),
                )
            };
            {
                let mut sessions = self.sessions.borrow_mut();
                let Some(session) = sessions.get_mut(&args.session_id) else {
                    return Err(Error::resource_not_found(None));
                };
                session.active_backend = target_backend;
            }
            send_agent_text(
                &args.session_id,
                format!(
                    "Switched backend to `{}` for this thread.",
                    target_backend.as_str()
                ),
            )
            .await;
            return Ok(SetSessionConfigOptionResponse::new(merged_options));
        }

        let parent_session_id = args.session_id.clone();
        let (backend, driver, child_session_id) = self.resolve_routed(&parent_session_id)?;
        let mut routed = args;
        routed.session_id = child_session_id;
        let response = driver.set_session_config_option(routed).await?;
        let merged_options = Self::merge_active_options(backend, response.config_options.clone());
        {
            let mut sessions = self.sessions.borrow_mut();
            if let Some(session) = sessions.get_mut(&parent_session_id) {
                session
                    .backend_config_options
                    .insert(backend, response.config_options.clone());
            }
        }
        Ok(SetSessionConfigOptionResponse::new(merged_options).meta(response.meta))
    }
}
