use std::{
    cell::RefCell,
    collections::HashMap,
    ops::DerefMut,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, LazyLock, Mutex},
};

use codex_apply_patch::parse_patch;

use agent_client_protocol::{
    Annotations, AudioContent, AvailableCommand, AvailableCommandInput, AvailableCommandsUpdate,
    BlobResourceContents, Client, ClientCapabilities, ConfigOptionUpdate, Content, ContentBlock,
    ContentChunk, Diff, EmbeddedResource, EmbeddedResourceResource, Error, ImageContent,
    LoadSessionResponse, Meta, ModelId, ModelInfo, PermissionOption, PermissionOptionKind, Plan,
    PlanEntry, PlanEntryPriority, PlanEntryStatus, PromptRequest, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, ResourceLink, SelectedPermissionOutcome,
    SessionConfigId, SessionConfigOption, SessionConfigOptionCategory, SessionConfigSelectOption,
    SessionConfigValueId, SessionId, SessionMode, SessionModeId, SessionModeState,
    SessionModelState, SessionNotification, SessionUpdate, StopReason, Terminal, TextContent,
    TextResourceContents, ToolCall, ToolCallContent, ToolCallId, ToolCallLocation, ToolCallStatus,
    ToolCallUpdate, ToolCallUpdateFields, ToolKind, UnstructuredCommandInput,
};
use codex_common::approval_presets::{ApprovalPreset, builtin_approval_presets};
use codex_core::{
    AuthManager, CodexThread, RolloutRecorder, ThreadSortKey,
    config::{Config, set_project_trust_level},
    error::CodexErr,
    models_manager::manager::{ModelsManager, RefreshStrategy},
    parse_command::parse_command,
    parse_turn_item,
    protocol::{
        AgentMessageContentDeltaEvent, AgentMessageEvent, AgentReasoningEvent,
        AgentReasoningRawContentEvent, AgentReasoningSectionBreakEvent,
        ApplyPatchApprovalRequestEvent, ElicitationAction, ErrorEvent, Event, EventMsg,
        ExecApprovalRequestEvent, ExecCommandBeginEvent, ExecCommandEndEvent,
        ExecCommandOutputDeltaEvent, ExitedReviewModeEvent, FileChange, ItemCompletedEvent,
        ItemStartedEvent, ListCustomPromptsResponseEvent, McpInvocation, McpStartupCompleteEvent,
        McpStartupUpdateEvent, McpToolCallBeginEvent, McpToolCallEndEvent, Op,
        PatchApplyBeginEvent, PatchApplyEndEvent, ReasoningContentDeltaEvent,
        ReasoningRawContentDeltaEvent, ReviewDecision, ReviewOutputEvent, ReviewRequest,
        ReviewTarget, SandboxPolicy, SessionSource, StreamErrorEvent, TerminalInteractionEvent,
        TurnAbortedEvent, TurnCompleteEvent, TurnStartedEvent, UserMessageEvent,
        ViewImageToolCallEvent, WarningEvent, WebSearchBeginEvent, WebSearchEndEvent,
    },
    review_format::format_review_findings_block,
    review_prompts::user_facing_hint,
};
use codex_protocol::{
    approvals::ElicitationRequestEvent,
    config_types::{Personality, TrustLevel},
    custom_prompts::CustomPrompt,
    items::TurnItem,
    models::ResponseItem,
    openai_models::{ModelPreset, ReasoningEffort},
    parse_command::ParsedCommand,
    plan_tool::{PlanItemArg, StepStatus, UpdatePlanArgs},
    protocol::{RolloutItem, SessionMetaLine},
    user_input::UserInput,
};
use heck::ToTitleCase;
use itertools::Itertools;
use mcp_types::{CallToolResult, RequestId};
use serde_json::json;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::{
    ACP_CLIENT,
    prompt_args::{expand_custom_prompt, parse_slash_name},
    session_store::SessionStore,
};

static APPROVAL_PRESETS: LazyLock<Vec<ApprovalPreset>> = LazyLock::new(builtin_approval_presets);
const INIT_COMMAND_PROMPT: &str = include_str!("./prompt_for_init_command.md");
const SESSION_LIST_PAGE_SIZE: usize = 25;
const SESSION_TITLE_MAX_GRAPHEMES: usize = 120;

#[derive(Clone, Debug)]
struct SessionListEntry {
    id: SessionId,
    title: Option<String>,
    updated_at: Option<String>,
}

/// Trait for abstracting over the `CodexThread` to make testing easier.
#[async_trait::async_trait]
pub trait CodexThreadImpl {
    async fn submit(&self, op: Op) -> Result<String, CodexErr>;
    async fn next_event(&self) -> Result<Event, CodexErr>;
}

#[async_trait::async_trait]
impl CodexThreadImpl for CodexThread {
    async fn submit(&self, op: Op) -> Result<String, CodexErr> {
        self.submit(op).await
    }

    async fn next_event(&self) -> Result<Event, CodexErr> {
        self.next_event().await
    }
}

#[async_trait::async_trait]
pub trait ModelsManagerImpl {
    async fn get_model(&self, model_id: &Option<String>, config: &Config) -> String;
    async fn list_models(&self, config: &Config) -> Vec<ModelPreset>;
}

#[async_trait::async_trait]
impl ModelsManagerImpl for ModelsManager {
    async fn get_model(&self, model_id: &Option<String>, config: &Config) -> String {
        self.get_default_model(model_id, config, RefreshStrategy::OnlineIfUncached)
            .await
    }

    async fn list_models(&self, config: &Config) -> Vec<ModelPreset> {
        self.list_models(config, RefreshStrategy::OnlineIfUncached)
            .await
    }
}

pub trait Auth {
    fn logout(&self) -> Result<bool, Error>;
}

impl Auth for Arc<AuthManager> {
    fn logout(&self) -> Result<bool, Error> {
        self.as_ref()
            .logout()
            .map_err(|e| Error::internal_error().data(e.to_string()))
    }
}

enum ThreadMessage {
    Load {
        response_tx: oneshot::Sender<Result<LoadSessionResponse, Error>>,
    },
    GetConfigOptions {
        response_tx: oneshot::Sender<Result<Vec<SessionConfigOption>, Error>>,
    },
    Prompt {
        request: PromptRequest,
        response_tx: oneshot::Sender<Result<oneshot::Receiver<Result<StopReason, Error>>, Error>>,
    },
    SetMode {
        mode: SessionModeId,
        response_tx: oneshot::Sender<Result<(), Error>>,
    },
    SetModel {
        model: ModelId,
        response_tx: oneshot::Sender<Result<(), Error>>,
    },
    SetConfigOption {
        config_id: SessionConfigId,
        value: SessionConfigValueId,
        response_tx: oneshot::Sender<Result<(), Error>>,
    },
    Cancel {
        response_tx: oneshot::Sender<Result<(), Error>>,
    },
    ReplayHistory {
        history: Vec<RolloutItem>,
        response_tx: oneshot::Sender<Result<(), Error>>,
    },
}

pub struct Thread {
    /// A sender for interacting with the thread.
    message_tx: mpsc::UnboundedSender<ThreadMessage>,
    /// A handle to the spawned task.
    _handle: tokio::task::JoinHandle<()>,
}

impl Thread {
    pub fn new(
        session_id: SessionId,
        thread: Arc<dyn CodexThreadImpl>,
        auth: Arc<AuthManager>,
        models_manager: Arc<dyn ModelsManagerImpl>,
        client_capabilities: Arc<Mutex<ClientCapabilities>>,
        config: Config,
        session_store: Option<SessionStore>,
    ) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        let actor = ThreadActor::new(
            auth,
            SessionClient::new(session_id, client_capabilities, session_store),
            thread,
            models_manager,
            config,
            message_rx,
        );
        let handle = tokio::task::spawn_local(actor.spawn());

        Self {
            message_tx,
            _handle: handle,
        }
    }

    pub async fn load(&self) -> Result<LoadSessionResponse, Error> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = ThreadMessage::Load { response_tx };
        drop(self.message_tx.send(message));

        response_rx
            .await
            .map_err(|e| Error::internal_error().data(e.to_string()))?
    }

    pub async fn config_options(&self) -> Result<Vec<SessionConfigOption>, Error> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = ThreadMessage::GetConfigOptions { response_tx };
        drop(self.message_tx.send(message));

        response_rx
            .await
            .map_err(|e| Error::internal_error().data(e.to_string()))?
    }

    pub async fn prompt(&self, request: PromptRequest) -> Result<StopReason, Error> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = ThreadMessage::Prompt {
            request,
            response_tx,
        };
        drop(self.message_tx.send(message));

        response_rx
            .await
            .map_err(|e| Error::internal_error().data(e.to_string()))??
            .await
            .map_err(|e| Error::internal_error().data(e.to_string()))?
    }

    pub async fn set_mode(&self, mode: SessionModeId) -> Result<(), Error> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = ThreadMessage::SetMode { mode, response_tx };
        drop(self.message_tx.send(message));

        response_rx
            .await
            .map_err(|e| Error::internal_error().data(e.to_string()))?
    }

    pub async fn set_model(&self, model: ModelId) -> Result<(), Error> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = ThreadMessage::SetModel { model, response_tx };
        drop(self.message_tx.send(message));

        response_rx
            .await
            .map_err(|e| Error::internal_error().data(e.to_string()))?
    }

    pub async fn set_config_option(
        &self,
        config_id: SessionConfigId,
        value: SessionConfigValueId,
    ) -> Result<(), Error> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = ThreadMessage::SetConfigOption {
            config_id,
            value,
            response_tx,
        };
        drop(self.message_tx.send(message));

        response_rx
            .await
            .map_err(|e| Error::internal_error().data(e.to_string()))?
    }

    pub async fn cancel(&self) -> Result<(), Error> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = ThreadMessage::Cancel { response_tx };
        drop(self.message_tx.send(message));

        response_rx
            .await
            .map_err(|e| Error::internal_error().data(e.to_string()))?
    }

    pub async fn replay_history(&self, history: Vec<RolloutItem>) -> Result<(), Error> {
        let (response_tx, response_rx) = oneshot::channel();

        let message = ThreadMessage::ReplayHistory {
            history,
            response_tx,
        };
        drop(self.message_tx.send(message));

        response_rx
            .await
            .map_err(|e| Error::internal_error().data(e.to_string()))?
    }
}

enum SubmissionState {
    /// Loading custom prompts from the project
    CustomPrompts(CustomPromptsState),
    /// User prompts + some slash commands like /init or /review
    Prompt(PromptState),
    /// Subtask, like /compact
    Task(TaskState),
    /// One-shot slash commands that return a single response event.
    OneShot(OneShotCommandState),
}

impl SubmissionState {
    fn is_active(&self) -> bool {
        match self {
            Self::CustomPrompts(state) => state.is_active(),
            Self::Prompt(state) => state.is_active(),
            Self::Task(state) => state.is_active(),
            Self::OneShot(state) => state.is_active(),
        }
    }

    async fn handle_event(&mut self, client: &SessionClient, event: EventMsg) {
        match self {
            Self::CustomPrompts(state) => state.handle_event(event),
            Self::Prompt(state) => state.handle_event(client, event).await,
            Self::Task(state) => state.handle_event(client, event).await,
            Self::OneShot(state) => state.handle_event(client, event).await,
        }
    }
}

struct CustomPromptsState {
    response_tx: Option<oneshot::Sender<Result<Vec<CustomPrompt>, Error>>>,
}

impl CustomPromptsState {
    fn new(response_tx: oneshot::Sender<Result<Vec<CustomPrompt>, Error>>) -> Self {
        Self {
            response_tx: Some(response_tx),
        }
    }

    fn is_active(&self) -> bool {
        let Some(response_tx) = &self.response_tx else {
            return false;
        };
        !response_tx.is_closed()
    }

    fn handle_event(&mut self, event: EventMsg) {
        match event {
            EventMsg::ListCustomPromptsResponse(ListCustomPromptsResponseEvent {
                custom_prompts,
            }) => {
                if let Some(tx) = self.response_tx.take() {
                    drop(tx.send(Ok(custom_prompts)));
                }
            }
            e => {
                warn!("Unexpected event: {e:?}");
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OneShotKind {
    McpTools,
    Skills,
}

struct OneShotCommandState {
    kind: OneShotKind,
    response_tx: Option<oneshot::Sender<Result<StopReason, Error>>>,
}

impl OneShotCommandState {
    fn new(kind: OneShotKind, response_tx: oneshot::Sender<Result<StopReason, Error>>) -> Self {
        Self {
            kind,
            response_tx: Some(response_tx),
        }
    }

    fn is_active(&self) -> bool {
        let Some(response_tx) = &self.response_tx else {
            return false;
        };
        !response_tx.is_closed()
    }

    async fn handle_event(&mut self, client: &SessionClient, event: EventMsg) {
        match (self.kind, event) {
            (OneShotKind::McpTools, EventMsg::McpListToolsResponse(event)) => {
                client
                    .send_agent_text(format_mcp_tools_message(&event))
                    .await;
                if let Some(tx) = self.response_tx.take() {
                    drop(tx.send(Ok(StopReason::EndTurn)));
                }
            }
            (OneShotKind::Skills, EventMsg::ListSkillsResponse(event)) => {
                client.send_agent_text(format_skills_message(&event)).await;
                if let Some(tx) = self.response_tx.take() {
                    drop(tx.send(Ok(StopReason::EndTurn)));
                }
            }
            (_, EventMsg::Error(err)) => {
                if let Some(tx) = self.response_tx.take() {
                    drop(tx.send(Err(Error::internal_error().data(err.message))));
                }
            }
            _ => {}
        }
    }
}

struct ActiveCommand {
    call_id: String,
    tool_call_id: ToolCallId,
    terminal_output: bool,
    output: String,
    file_extension: Option<String>,
}

struct PromptState {
    active_command: Option<ActiveCommand>,
    active_web_search: Option<String>,
    thread: Arc<dyn CodexThreadImpl>,
    event_count: usize,
    response_tx: Option<oneshot::Sender<Result<StopReason, Error>>>,
    submission_id: String,
    seen_message_deltas: bool,
    seen_reasoning_deltas: bool,
}

impl PromptState {
    fn new(
        thread: Arc<dyn CodexThreadImpl>,
        response_tx: oneshot::Sender<Result<StopReason, Error>>,
        submission_id: String,
    ) -> Self {
        Self {
            active_command: None,
            active_web_search: None,
            thread,
            event_count: 0,
            response_tx: Some(response_tx),
            submission_id,
            seen_message_deltas: false,
            seen_reasoning_deltas: false,
        }
    }

    fn is_active(&self) -> bool {
        let Some(response_tx) = &self.response_tx else {
            return false;
        };
        !response_tx.is_closed()
    }

    #[expect(clippy::too_many_lines)]
    async fn handle_event(&mut self, client: &SessionClient, event: EventMsg) {
        self.event_count += 1;

        // Complete any previous web search before starting a new one
        match &event {
            EventMsg::Error(..)
            | EventMsg::StreamError(..)
            | EventMsg::WebSearchBegin(..)
            | EventMsg::UserMessage(..)
            | EventMsg::ExecApprovalRequest(..)
            | EventMsg::ExecCommandBegin(..)
            | EventMsg::ExecCommandOutputDelta(..)
            | EventMsg::ExecCommandEnd(..)
            | EventMsg::McpToolCallBegin(..)
            | EventMsg::McpToolCallEnd(..)
            | EventMsg::ApplyPatchApprovalRequest(..)
            | EventMsg::PatchApplyBegin(..)
            | EventMsg::PatchApplyEnd(..)
            | EventMsg::TurnStarted(..)
            | EventMsg::TurnComplete(..)
            | EventMsg::TokenCount(..)
            | EventMsg::TurnDiff(..)
            | EventMsg::TurnAborted(..)
            | EventMsg::EnteredReviewMode(..)
            | EventMsg::ExitedReviewMode(..)
            | EventMsg::ShutdownComplete => {
                self.complete_web_search(client).await;
            }
            _ => {}
        }

        match event {
            EventMsg::TurnStarted(TurnStartedEvent {
                model_context_window,
            }) => {
                info!("Task started with context window of {model_context_window:?}");
            }
            EventMsg::ItemStarted(ItemStartedEvent { thread_id, turn_id, item }) => {

                info!("Item started with thread_id: {thread_id}, turn_id: {turn_id}, item: {item:?}");
            }
            EventMsg::UserMessage(UserMessageEvent {
                message,
                images: _,
                text_elements: _,
                local_images: _,
            }) => {
                info!("User message: {message:?}");
            }
            EventMsg::AgentMessageContentDelta(AgentMessageContentDeltaEvent { thread_id, turn_id, item_id, delta }) => {
                info!("Agent message content delta received: thread_id: {thread_id}, turn_id: {turn_id}, item_id: {item_id}, delta: {delta:?}");
                self.seen_message_deltas = true;
                client.send_agent_text(delta).await;
            }
            EventMsg::ReasoningContentDelta(ReasoningContentDeltaEvent { thread_id, turn_id, item_id, delta, summary_index: index })
            | EventMsg::ReasoningRawContentDelta(ReasoningRawContentDeltaEvent { thread_id, turn_id, item_id, delta, content_index: index }) => {
                info!("Agent reasoning content delta received: thread_id: {thread_id}, turn_id: {turn_id}, item_id: {item_id}, index: {index}, delta: {delta:?}");
                self.seen_reasoning_deltas = true;
                client.send_agent_thought(delta).await;
            }
            EventMsg::AgentReasoningSectionBreak(AgentReasoningSectionBreakEvent { item_id, summary_index}) => {
                info!("Agent reasoning section break received:  item_id: {item_id}, index: {summary_index}");
                // Make sure the section heading actually get spacing
                self.seen_reasoning_deltas = true;
                client.send_agent_thought("\n\n").await;
            }
            EventMsg::AgentMessage(AgentMessageEvent { message }) => {
                info!("Agent message (non-delta) received: {message:?}");
                // We didn't receive this message via streaming
                if !std::mem::take(&mut self.seen_message_deltas) {
                    client.send_agent_text(message).await;
                }
            }
            EventMsg::AgentReasoning(AgentReasoningEvent { text }) => {
                info!("Agent reasoning (non-delta) received: {text:?}");
                // We didn't receive this message via streaming
                if !std::mem::take(&mut self.seen_reasoning_deltas) {
                    client.send_agent_thought(text).await;
                }
            }
            EventMsg::PlanUpdate(UpdatePlanArgs { explanation, plan }) => {
                // Send this to the client via session/update notification
                info!("Agent plan updated. Explanation: {:?}", explanation);
                client.update_plan(plan).await;
            }
            EventMsg::WebSearchBegin(WebSearchBeginEvent { call_id }) => {
                info!("Web search started: call_id={}", call_id);
                // Create a ToolCall notification for the search beginning
                self.start_web_search(client, call_id).await;
            }
            EventMsg::WebSearchEnd(WebSearchEndEvent { call_id, query }) => {
                info!("Web search query received: call_id={call_id}, query={query}");
                // Send update that the search is in progress with the query
                // (WebSearchEnd just means we have the query, not that results are ready)
                self.update_web_search_query(client, call_id, query).await;
                // The actual search results will come through AgentMessage events
                // We mark as completed when a new tool call begins
            }
            EventMsg::ExecApprovalRequest(event) => {
                info!("Command execution started: call_id={}, command={:?}", event.call_id, event.command);
                if let Err(err) = self.exec_approval(client, event).await && let Some(response_tx) = self.response_tx.take() {
                    drop(response_tx.send(Err(err)));
                }
            }
            EventMsg::ExecCommandBegin(event) => {
                info!(
                    "Command execution started: call_id={}, command={:?}",
                    event.call_id, event.command
                );
                self.exec_command_begin(client, event).await;
            }
            EventMsg::ExecCommandOutputDelta(delta_event) => {
                self.exec_command_output_delta(client, delta_event).await;
            }
            EventMsg::ExecCommandEnd(end_event) => {
                info!(
                    "Command execution ended: call_id={}, exit_code={}",
                    end_event.call_id, end_event.exit_code
                );
                self.exec_command_end(client, end_event).await;
            }
            EventMsg::TerminalInteraction(event) => {
                info!(
                    "Terminal interaction: call_id={}, process_id={}, stdin={}",
                    event.call_id, event.process_id, event.stdin
                );
                self.terminal_interaction(client, event).await;
            }
            EventMsg::McpToolCallBegin(McpToolCallBeginEvent { call_id, invocation }) => {
                info!("MCP tool call begin: call_id={call_id}, invocation={} {}", invocation.server, invocation.tool);
                self.start_mcp_tool_call(client, call_id, invocation).await;
            }
            EventMsg::McpToolCallEnd(McpToolCallEndEvent { call_id, invocation, duration, result }) => {
                info!("MCP tool call ended: call_id={call_id}, invocation={} {}, duration={duration:?}", invocation.server, invocation.tool);
                self.end_mcp_tool_call(client, call_id, result).await;
            }
            EventMsg::ApplyPatchApprovalRequest(event) => {
                info!("Apply patch approval request: call_id={}, reason={:?}", event.call_id, event.reason);
                if let Err(err) = self.patch_approval(client, event).await && let Some(response_tx) = self.response_tx.take() {
                    drop(response_tx.send(Err(err)));
                }
            }
            EventMsg::PatchApplyBegin(event) => {
                info!("Patch apply begin: call_id={}, auto_approved={}", event.call_id,event.auto_approved);
                self.start_patch_apply(client, event).await;
            }
            EventMsg::PatchApplyEnd(event) => {
                info!("Patch apply end: call_id={}, success={}", event.call_id, event.success);
                self.end_patch_apply(client, event).await;
            }
            EventMsg::ItemCompleted(ItemCompletedEvent { thread_id, turn_id, item }) => {
                info!("Item completed: thread_id={}, turn_id={}, item={:?}", thread_id, turn_id, item);
            }
            EventMsg::TurnComplete(TurnCompleteEvent { last_agent_message}) => {
                info!(
                    "Task completed successfully after {} events. Last agent message: {last_agent_message:?}", self.event_count
                );
                if let Some(response_tx) = self.response_tx.take() {
                    response_tx.send(Ok(StopReason::EndTurn)).ok();
                }
            }
            EventMsg::UndoStarted(event) => {
                client
                    .send_agent_text(
                        event
                            .message
                            .unwrap_or_else(|| "Undo in progress...".to_string()),
                    )
                    .await;
            }
            EventMsg::UndoCompleted(event) => {
                let fallback = if event.success {
                    "Undo completed.".to_string()
                } else {
                    "Undo failed.".to_string()
                };
                client.send_agent_text(event.message.unwrap_or(fallback)).await;
            }
            EventMsg::StreamError(StreamErrorEvent { message , codex_error_info, additional_details }) => {
                error!("Handled error during turn: {message} {codex_error_info:?} {additional_details:?}");
            }
            EventMsg::Error(ErrorEvent { message, codex_error_info }) => {
                error!("Unhandled error during turn: {message} {codex_error_info:?}");
                if let Some(response_tx) = self.response_tx.take() {
                    response_tx.send(Err(Error::internal_error().data(json!({ "message": message, "codex_error_info": codex_error_info })))).ok();
                }
            }
            EventMsg::TurnAborted(TurnAbortedEvent { reason }) => {
                info!("Turn aborted: {reason:?}");
                if let Some(response_tx) = self.response_tx.take() {
                    response_tx.send(Ok(StopReason::Cancelled)).ok();
                }
            }
            EventMsg::ShutdownComplete => {
                info!("Agent shutting down");
                if let Some(response_tx) = self.response_tx.take() {
                    response_tx.send(Ok(StopReason::Cancelled)).ok();
                }
            }
            EventMsg::ViewImageToolCall(ViewImageToolCallEvent { call_id, path }) => {
                info!("ViewImageToolCallEvent received");
                let display_path = path.display().to_string();
                client
                    .send_notification(
                        SessionUpdate::ToolCall(
                            ToolCall::new(call_id, format!("View Image {display_path}"))
                                .kind(ToolKind::Read).status(ToolCallStatus::Completed)
                                .content(vec![ToolCallContent::Content(Content::new(ContentBlock::ResourceLink(ResourceLink::new(display_path.clone(), display_path.clone())
                            )
                        )
                    )]).locations(vec![ToolCallLocation::new(path)])))
                    .await;
            }
            EventMsg::EnteredReviewMode(review_request) => {
                info!("Review begin: request={review_request:?}");
            }
            EventMsg::ExitedReviewMode(event) => {
                info!("Review end: output={event:?}");
                if let Err(err) = self.review_mode_exit(client, event).await && let Some(response_tx) = self.response_tx.take() {
                    drop(response_tx.send(Err(err)));
                }
            }
            EventMsg::Warning(WarningEvent { message }) => {
                warn!("Warning: {message}");
            }
            EventMsg::McpStartupUpdate(McpStartupUpdateEvent { server, status }) => {
                info!("MCP startup update: server={server}, status={status:?}");
            }
            EventMsg::McpStartupComplete(McpStartupCompleteEvent {
                ready,
                failed,
                cancelled,
            }) => {
                info!(
                    "MCP startup complete: ready={ready:?}, failed={failed:?}, cancelled={cancelled:?}"
                );
            }
            EventMsg::ElicitationRequest(event) => {
                info!("Elicitation request: server={}, id={:?}, message={}", event.server_name, event.id, event.message);
                if let Err(err) = self.mcp_elicitation(client, event).await && let Some(response_tx) = self.response_tx.take() {
                    drop(response_tx.send(Err(err)));
                }
            }

            // Ignore these events
            EventMsg::AgentReasoningRawContent(..)
            | EventMsg::ThreadRolledBack(..)
            // In the future we can use this to update usage stats
            | EventMsg::TokenCount(..)
            // we already have a way to diff the turn, so ignore
            | EventMsg::TurnDiff(..)
            // Revisit when we can emit status updates
            | EventMsg::BackgroundEvent(..)
            | EventMsg::ContextCompacted(..)
            | EventMsg::SkillsUpdateAvailable
            // Old events
            | EventMsg::AgentMessageDelta(..) | EventMsg::AgentReasoningDelta(..) | EventMsg::AgentReasoningRawContentDelta(..)
            | EventMsg::RawResponseItem(..)
            | EventMsg::SessionConfigured(..)
            // TODO: Subagent UI?
            | EventMsg::CollabAgentSpawnBegin(..)
            | EventMsg::CollabAgentSpawnEnd(..)
            | EventMsg::CollabAgentInteractionBegin(..)
            | EventMsg::CollabAgentInteractionEnd(..)
            | EventMsg::CollabWaitingBegin(..)
            | EventMsg::CollabWaitingEnd(..)
            | EventMsg::CollabCloseBegin(..)
            | EventMsg::CollabCloseEnd(..) => {},
            e @ (EventMsg::McpListToolsResponse(..)
            // returned from Op::ListCustomPrompts, ignore
            | EventMsg::ListCustomPromptsResponse(..)
            | EventMsg::ListSkillsResponse(..)
            // Used for returning a single history entry
            | EventMsg::GetHistoryEntryResponse(..)
            | EventMsg::DeprecationNotice(..)
            |  EventMsg::RequestUserInput(..)
            | EventMsg::DynamicToolCallRequest(..)
            ) => {
                warn!("Unexpected event: {:?}", e);
            }
        }
    }

    async fn mcp_elicitation(
        &self,
        client: &SessionClient,
        event: ElicitationRequestEvent,
    ) -> Result<(), Error> {
        let raw_input = serde_json::json!(&event);
        let ElicitationRequestEvent {
            server_name,
            id,
            message,
        } = event;
        let tool_call_id = ToolCallId::new(match &id {
            RequestId::String(s) => s.clone(),
            RequestId::Integer(i) => i.to_string(),
        });
        let response = client
            .request_permission(
                ToolCallUpdate::new(
                    tool_call_id.clone(),
                    ToolCallUpdateFields::new()
                        .title(server_name.clone())
                        .status(ToolCallStatus::Pending)
                        .content(vec![message.into()])
                        .raw_input(raw_input),
                ),
                vec![
                    PermissionOption::new(
                        "approved",
                        "Yes, provide the requested info",
                        PermissionOptionKind::AllowOnce,
                    ),
                    PermissionOption::new(
                        "abort",
                        "No, but continue without it",
                        PermissionOptionKind::RejectOnce,
                    ),
                    PermissionOption::new(
                        "cancel",
                        "Cancel this request",
                        PermissionOptionKind::RejectOnce,
                    ),
                ],
            )
            .await?;

        let decision = match response.outcome {
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome { option_id, .. }) => {
                match option_id.0.as_ref() {
                    "approved" => ElicitationAction::Accept,
                    "abort" => ElicitationAction::Decline,
                    _ => ElicitationAction::Cancel,
                }
            }
            RequestPermissionOutcome::Cancelled | _ => ElicitationAction::Cancel,
        };

        self.thread
            .submit(Op::ResolveElicitation {
                server_name,
                request_id: id,
                decision,
            })
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;

        client
            .send_notification(SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
                tool_call_id,
                ToolCallUpdateFields::new().status(if decision == ElicitationAction::Accept {
                    ToolCallStatus::Completed
                } else {
                    ToolCallStatus::Failed
                }),
            )))
            .await;

        Ok(())
    }

    async fn review_mode_exit(
        &self,
        client: &SessionClient,
        event: ExitedReviewModeEvent,
    ) -> Result<(), Error> {
        let ExitedReviewModeEvent { review_output } = event;
        let Some(ReviewOutputEvent {
            findings,
            overall_correctness: _,
            overall_explanation,
            overall_confidence_score: _,
        }) = review_output
        else {
            return Ok(());
        };

        let text = if findings.is_empty() {
            let explanation = overall_explanation.trim();
            if explanation.is_empty() {
                "Reviewer failed to output a response"
            } else {
                explanation
            }
            .to_string()
        } else {
            format_review_findings_block(&findings, None)
        };

        client.send_agent_text(&text).await;
        Ok(())
    }

    async fn patch_approval(
        &self,
        client: &SessionClient,
        event: ApplyPatchApprovalRequestEvent,
    ) -> Result<(), Error> {
        let raw_input = serde_json::json!(&event);
        let ApplyPatchApprovalRequestEvent {
            call_id,
            changes,
            reason,
            // grant_root doesn't seem to be set anywhere on the codex side
            grant_root: _,
            turn_id: _,
        } = event;
        let (title, locations, content) = extract_tool_call_content_from_changes(changes);
        let response = client
            .request_permission(
                ToolCallUpdate::new(
                    call_id,
                    ToolCallUpdateFields::new()
                        .kind(ToolKind::Edit)
                        .status(ToolCallStatus::Pending)
                        .title(title)
                        .locations(locations)
                        .content(content.chain(reason.map(|r| r.into())).collect::<Vec<_>>())
                        .raw_input(raw_input),
                ),
                vec![
                    PermissionOption::new("approved", "Yes", PermissionOptionKind::AllowOnce),
                    PermissionOption::new(
                        "abort",
                        "No, provide feedback",
                        PermissionOptionKind::RejectOnce,
                    ),
                ],
            )
            .await?;

        let decision = match response.outcome {
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome { option_id, .. }) => {
                match option_id.0.as_ref() {
                    "approved" => ReviewDecision::Approved,
                    _ => ReviewDecision::Abort,
                }
            }
            RequestPermissionOutcome::Cancelled | _ => ReviewDecision::Abort,
        };

        self.thread
            .submit(Op::PatchApproval {
                id: self.submission_id.clone(),
                decision,
            })
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;
        Ok(())
    }

    async fn start_patch_apply(&self, client: &SessionClient, event: PatchApplyBeginEvent) {
        let raw_input = serde_json::json!(&event);
        let PatchApplyBeginEvent {
            call_id,
            auto_approved: _,
            changes,
            turn_id: _,
        } = event;

        let (title, locations, content) = extract_tool_call_content_from_changes(changes);

        client
            .send_tool_call(
                ToolCall::new(call_id, title)
                    .kind(ToolKind::Edit)
                    .status(ToolCallStatus::InProgress)
                    .locations(locations)
                    .content(content.collect())
                    .raw_input(raw_input),
            )
            .await;
    }

    async fn end_patch_apply(&self, client: &SessionClient, event: PatchApplyEndEvent) {
        let raw_output = serde_json::json!(&event);
        let PatchApplyEndEvent {
            call_id,
            stdout: _,
            stderr: _,
            success,
            changes,
            turn_id: _,
        } = event;

        let (title, locations, content) = if !changes.is_empty() {
            let (title, locations, content) = extract_tool_call_content_from_changes(changes);
            (Some(title), Some(locations), Some(content.collect()))
        } else {
            (None, None, None)
        };

        client
            .send_tool_call_update(ToolCallUpdate::new(
                call_id,
                ToolCallUpdateFields::new()
                    .status(if success {
                        ToolCallStatus::Completed
                    } else {
                        ToolCallStatus::Failed
                    })
                    .raw_output(raw_output)
                    .title(title)
                    .locations(locations)
                    .content(content),
            ))
            .await;
    }

    async fn start_mcp_tool_call(
        &self,
        client: &SessionClient,
        call_id: String,
        invocation: McpInvocation,
    ) {
        let title = format!("Tool: {}/{}", invocation.server, invocation.tool);
        client
            .send_tool_call(
                ToolCall::new(call_id, title)
                    .status(ToolCallStatus::InProgress)
                    .raw_input(serde_json::json!(&invocation)),
            )
            .await;
    }

    async fn end_mcp_tool_call(
        &self,
        client: &SessionClient,
        call_id: String,
        result: Result<CallToolResult, String>,
    ) {
        let is_error = match result.as_ref() {
            Ok(result) => result.is_error.unwrap_or_default(),
            Err(_) => true,
        };
        let raw_output = match result.as_ref() {
            Ok(result) => serde_json::json!(result),
            Err(err) => serde_json::json!(err),
        };

        client
            .send_tool_call_update(ToolCallUpdate::new(
                call_id,
                ToolCallUpdateFields::new()
                    .status(if is_error {
                        ToolCallStatus::Failed
                    } else {
                        ToolCallStatus::Completed
                    })
                    .raw_output(raw_output)
                    .content(result.ok().filter(|result| !result.content.is_empty()).map(
                        |result| {
                            result
                                .content
                                .into_iter()
                                .map(codex_content_to_acp_content)
                                .collect()
                        },
                    )),
            ))
            .await;
    }

    async fn exec_approval(
        &mut self,
        client: &SessionClient,
        event: ExecApprovalRequestEvent,
    ) -> Result<(), Error> {
        let raw_input = serde_json::json!(&event);
        let ExecApprovalRequestEvent {
            call_id,
            command: _,
            turn_id: _,
            cwd,
            reason,
            parsed_cmd,
            proposed_execpolicy_amendment,
        } = event;

        // Create a new tool call for the command execution
        let tool_call_id = ToolCallId::new(call_id.clone());
        let ParseCommandToolCall {
            title,
            terminal_output,
            file_extension,
            locations,
            kind,
        } = parse_command_tool_call(parsed_cmd, &cwd);
        self.active_command = Some(ActiveCommand {
            call_id,
            terminal_output,
            tool_call_id: tool_call_id.clone(),
            output: String::new(),
            file_extension,
        });

        let mut content = vec![];

        if let Some(reason) = reason {
            content.push(reason);
        }
        if let Some(amendment) = proposed_execpolicy_amendment {
            content.push(format!(
                "Proposed Amendment: {}",
                amendment.command().join("\n")
            ));
        }

        let content = if content.is_empty() {
            None
        } else {
            Some(vec![content.join("\n").into()])
        };

        let response = client
            .request_permission(
                ToolCallUpdate::new(
                    tool_call_id,
                    ToolCallUpdateFields::new()
                        .kind(kind)
                        .status(ToolCallStatus::Pending)
                        .title(title)
                        .raw_input(raw_input)
                        .content(content)
                        .locations(if locations.is_empty() {
                            None
                        } else {
                            Some(locations)
                        }),
                ),
                vec![
                    PermissionOption::new(
                        "approved-for-session",
                        "Always",
                        PermissionOptionKind::AllowAlways,
                    ),
                    PermissionOption::new("approved", "Yes", PermissionOptionKind::AllowOnce),
                    PermissionOption::new(
                        "abort",
                        "No, provide feedback",
                        PermissionOptionKind::RejectOnce,
                    ),
                ],
            )
            .await?;

        let decision = match response.outcome {
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome { option_id, .. }) => {
                match option_id.0.as_ref() {
                    "approved-for-session" => ReviewDecision::ApprovedForSession,
                    "approved" => ReviewDecision::Approved,
                    _ => ReviewDecision::Abort,
                }
            }
            RequestPermissionOutcome::Cancelled | _ => ReviewDecision::Abort,
        };

        self.thread
            .submit(Op::ExecApproval {
                id: self.submission_id.clone(),
                decision,
            })
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;

        Ok(())
    }

    async fn exec_command_begin(&mut self, client: &SessionClient, event: ExecCommandBeginEvent) {
        let raw_input = serde_json::json!(&event);
        let ExecCommandBeginEvent {
            turn_id: _,
            source: _,
            interaction_input: _,
            call_id,
            command: _,
            cwd,
            parsed_cmd,
            process_id: _,
        } = event;
        // Create a new tool call for the command execution
        let tool_call_id = ToolCallId::new(call_id.clone());
        let ParseCommandToolCall {
            title,
            file_extension,
            locations,
            terminal_output,
            kind,
        } = parse_command_tool_call(parsed_cmd, &cwd);

        let active_command = ActiveCommand {
            call_id: call_id.clone(),
            tool_call_id: tool_call_id.clone(),
            output: String::new(),
            file_extension,
            terminal_output,
        };
        let (content, meta) = if client.supports_terminal_output(&active_command) {
            let content = vec![ToolCallContent::Terminal(Terminal::new(call_id.clone()))];
            let meta = Some(Meta::from_iter([(
                "terminal_info".to_owned(),
                serde_json::json!({
                    "terminal_id": call_id,
                    "cwd": cwd
                }),
            )]));
            (content, meta)
        } else {
            (vec![], None)
        };

        self.active_command = Some(active_command);

        client
            .send_tool_call(
                ToolCall::new(tool_call_id, title)
                    .kind(kind)
                    .status(ToolCallStatus::InProgress)
                    .locations(locations)
                    .raw_input(raw_input)
                    .content(content)
                    .meta(meta),
            )
            .await;
    }

    async fn exec_command_output_delta(
        &mut self,
        client: &SessionClient,
        event: ExecCommandOutputDeltaEvent,
    ) {
        let ExecCommandOutputDeltaEvent {
            call_id,
            chunk,
            stream: _,
        } = event;
        // Stream output bytes to the display-only terminal via ToolCallUpdate meta.
        if let Some(active_command) = &mut self.active_command
            && *active_command.call_id == call_id
        {
            let data_str = String::from_utf8_lossy(&chunk).to_string();

            let update = if client.supports_terminal_output(active_command) {
                ToolCallUpdate::new(
                    active_command.tool_call_id.clone(),
                    ToolCallUpdateFields::new(),
                )
                .meta(Meta::from_iter([(
                    "terminal_output".to_owned(),
                    serde_json::json!({
                        "terminal_id": call_id,
                        "data": data_str
                    }),
                )]))
            } else {
                active_command.output.push_str(&data_str);
                let content = match active_command.file_extension.as_deref() {
                    Some("md") => active_command.output.clone(),
                    Some(ext) => format!(
                        "```{ext}\n{}\n```\n",
                        active_command.output.trim_end_matches('\n')
                    ),
                    None => format!(
                        "```sh\n{}\n```\n",
                        active_command.output.trim_end_matches('\n')
                    ),
                };
                ToolCallUpdate::new(
                    active_command.tool_call_id.clone(),
                    ToolCallUpdateFields::new().content(vec![content.into()]),
                )
            };

            client.send_tool_call_update(update).await;
        }
    }

    async fn exec_command_end(&mut self, client: &SessionClient, event: ExecCommandEndEvent) {
        let raw_output = serde_json::json!(&event);
        let ExecCommandEndEvent {
            turn_id: _,
            command: _,
            cwd: _,
            parsed_cmd: _,
            source: _,
            interaction_input: _,
            call_id,
            exit_code,
            stdout: _,
            stderr: _,
            aggregated_output: _,
            duration: _,
            formatted_output: _,
            process_id: _,
        } = event;
        if let Some(active_command) = self.active_command.take()
            && active_command.call_id == call_id
        {
            let is_success = exit_code == 0;

            client
                .send_tool_call_update(
                    ToolCallUpdate::new(
                        active_command.tool_call_id.clone(),
                        ToolCallUpdateFields::new()
                            .status(if is_success {
                                ToolCallStatus::Completed
                            } else {
                                ToolCallStatus::Failed
                            })
                            .raw_output(raw_output),
                    )
                    .meta(
                        client.supports_terminal_output(&active_command).then(|| {
                            Meta::from_iter([(
                                "terminal_exit".into(),
                                serde_json::json!({
                                    "terminal_id": call_id,
                                    "exit_code": exit_code,
                                    "signal": null
                                }),
                            )])
                        }),
                    ),
                )
                .await;
        }
    }

    async fn terminal_interaction(
        &mut self,
        client: &SessionClient,
        event: TerminalInteractionEvent,
    ) {
        let TerminalInteractionEvent {
            call_id,
            process_id: _,
            stdin,
        } = event;

        let stdin = format!("\n{stdin}\n");
        // Stream output bytes to the display-only terminal via ToolCallUpdate meta.
        if let Some(active_command) = &mut self.active_command
            && *active_command.call_id == call_id
        {
            let update = if client.supports_terminal_output(active_command) {
                ToolCallUpdate::new(
                    active_command.tool_call_id.clone(),
                    ToolCallUpdateFields::new(),
                )
                .meta(Meta::from_iter([(
                    "terminal_output".to_owned(),
                    serde_json::json!({
                        "terminal_id": call_id,
                        "data": stdin
                    }),
                )]))
            } else {
                active_command.output.push_str(&stdin);
                let content = match active_command.file_extension.as_deref() {
                    Some("md") => active_command.output.clone(),
                    Some(ext) => format!(
                        "```{ext}\n{}\n```\n",
                        active_command.output.trim_end_matches('\n')
                    ),
                    None => format!(
                        "```sh\n{}\n```\n",
                        active_command.output.trim_end_matches('\n')
                    ),
                };
                ToolCallUpdate::new(
                    active_command.tool_call_id.clone(),
                    ToolCallUpdateFields::new().content(vec![content.into()]),
                )
            };

            client.send_tool_call_update(update).await;
        }
    }

    async fn start_web_search(&mut self, client: &SessionClient, call_id: String) {
        self.active_web_search = Some(call_id.clone());
        client
            .send_tool_call(ToolCall::new(call_id, "Searching the Web").kind(ToolKind::Fetch))
            .await;
    }

    async fn update_web_search_query(
        &self,
        client: &SessionClient,
        call_id: String,
        query: String,
    ) {
        client
            .send_tool_call_update(ToolCallUpdate::new(
                call_id,
                ToolCallUpdateFields::new()
                    .status(ToolCallStatus::InProgress)
                    .title(format!("Searching for: {query}"))
                    .raw_input(serde_json::json!({
                        "query": query
                    })),
            ))
            .await;
    }

    async fn complete_web_search(&mut self, client: &SessionClient) {
        if let Some(call_id) = self.active_web_search.take() {
            client
                .send_tool_call_update(ToolCallUpdate::new(
                    call_id,
                    ToolCallUpdateFields::new().status(ToolCallStatus::Completed),
                ))
                .await;
        }
    }
}

struct ParseCommandToolCall {
    title: String,
    file_extension: Option<String>,
    terminal_output: bool,
    locations: Vec<ToolCallLocation>,
    kind: ToolKind,
}

fn parse_command_tool_call(parsed_cmd: Vec<ParsedCommand>, cwd: &Path) -> ParseCommandToolCall {
    let mut titles = Vec::new();
    let mut locations = Vec::new();
    let mut file_extension = None;
    let mut terminal_output = false;
    let mut kind = ToolKind::Execute;

    for cmd in parsed_cmd {
        let mut cmd_path = None;
        match cmd {
            ParsedCommand::Read { cmd: _, name, path } => {
                titles.push(format!("Read {name}"));
                file_extension = path
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_string());
                cmd_path = Some(path);
                kind = ToolKind::Read;
            }
            ParsedCommand::ListFiles { cmd: _, path } => {
                let dir = if let Some(path) = path.as_ref() {
                    &cwd.join(path)
                } else {
                    cwd
                };
                titles.push(format!("List {}", dir.display()));
                cmd_path = path.map(PathBuf::from);
                kind = ToolKind::Search;
            }
            ParsedCommand::Search { cmd, query, path } => {
                titles.push(match (query, path.as_ref()) {
                    (Some(query), Some(path)) => format!("Search {query} in {path}"),
                    (Some(query), None) => format!("Search {query}"),
                    _ => format!("Search {cmd}"),
                });
                kind = ToolKind::Search;
            }
            ParsedCommand::Unknown { cmd } => {
                titles.push(format!("Run {cmd}"));
                terminal_output = true;
            }
        }

        if let Some(path) = cmd_path {
            locations.push(ToolCallLocation::new(if path.is_relative() {
                cwd.join(&path)
            } else {
                path
            }));
        }
    }

    ParseCommandToolCall {
        title: titles.join(", "),
        file_extension,
        terminal_output,
        locations,
        kind,
    }
}

struct TaskState {
    prompt: PromptState,
}

impl TaskState {
    fn new(
        thread: Arc<dyn CodexThreadImpl>,
        response_tx: oneshot::Sender<Result<StopReason, Error>>,
        submission_id: String,
    ) -> Self {
        Self {
            prompt: PromptState::new(thread, response_tx, submission_id),
        }
    }

    fn is_active(&self) -> bool {
        self.prompt.is_active()
    }

    async fn handle_event(&mut self, client: &SessionClient, event: EventMsg) {
        self.prompt.handle_event(client, event).await;
    }
}

#[derive(Clone)]
struct SessionClient {
    session_id: SessionId,
    client: Arc<dyn Client>,
    client_capabilities: Arc<Mutex<ClientCapabilities>>,
    session_store: Option<SessionStore>,
}

impl SessionClient {
    fn new(
        session_id: SessionId,
        client_capabilities: Arc<Mutex<ClientCapabilities>>,
        session_store: Option<SessionStore>,
    ) -> Self {
        Self {
            session_id,
            client: ACP_CLIENT.get().expect("Client should be set").clone(),
            client_capabilities,
            session_store,
        }
    }

    #[cfg(test)]
    fn with_client(
        session_id: SessionId,
        client: Arc<dyn Client>,
        client_capabilities: Arc<Mutex<ClientCapabilities>>,
        session_store: Option<SessionStore>,
    ) -> Self {
        Self {
            session_id,
            client,
            client_capabilities,
            session_store,
        }
    }

    fn log_canonical(&self, kind: &str, data: serde_json::Value) {
        if let Some(store) = self.session_store.as_ref() {
            store.log(kind, data);
        }
    }

    fn supports_terminal_output(&self, active_command: &ActiveCommand) -> bool {
        active_command.terminal_output
            && self
                .client_capabilities
                .lock()
                .unwrap()
                .meta
                .as_ref()
                .is_some_and(|v| {
                    v.get("terminal_output")
                        .is_some_and(|v| v.as_bool().unwrap_or_default())
                })
    }

    async fn send_notification(&self, update: SessionUpdate) {
        if let Err(e) = self
            .client
            .session_notification(SessionNotification::new(self.session_id.clone(), update))
            .await
        {
            error!("Failed to send session notification: {:?}", e);
        }
    }

    async fn send_user_message(&self, text: impl Into<String>) {
        let text = text.into();
        self.log_canonical("acp.user_message_chunk", json!({ "text": text }));
        self.send_notification(SessionUpdate::UserMessageChunk(ContentChunk::new(
            text.into(),
        )))
        .await;
    }

    async fn send_agent_text(&self, text: impl Into<String>) {
        let text = text.into();
        self.log_canonical("acp.agent_message_chunk", json!({ "text": text }));
        self.send_notification(SessionUpdate::AgentMessageChunk(ContentChunk::new(
            text.into(),
        )))
        .await;
    }

    async fn send_agent_thought(&self, text: impl Into<String>) {
        let text = text.into();
        self.log_canonical("acp.agent_thought_chunk", json!({ "text": text }));
        self.send_notification(SessionUpdate::AgentThoughtChunk(ContentChunk::new(
            text.into(),
        )))
        .await;
    }

    async fn send_tool_call(&self, tool_call: ToolCall) {
        let value = serde_json::to_value(&tool_call).unwrap_or_else(|_| {
            json!({
                "debug": format!("{tool_call:?}")
            })
        });
        self.log_canonical("acp.tool_call", value);
        self.send_notification(SessionUpdate::ToolCall(tool_call))
            .await;
    }

    async fn send_tool_call_update(&self, update: ToolCallUpdate) {
        let value = serde_json::to_value(&update).unwrap_or_else(|_| {
            json!({
                "debug": format!("{update:?}")
            })
        });
        self.log_canonical("acp.tool_call_update", value);
        self.send_notification(SessionUpdate::ToolCallUpdate(update))
            .await;
    }

    /// Send a completed tool call (used for replay and simple cases)
    async fn send_completed_tool_call(
        &self,
        call_id: impl Into<ToolCallId>,
        title: impl Into<String>,
        kind: ToolKind,
        raw_input: Option<serde_json::Value>,
    ) {
        let mut tool_call = ToolCall::new(call_id, title)
            .kind(kind)
            .status(ToolCallStatus::Completed);
        if let Some(input) = raw_input {
            tool_call = tool_call.raw_input(input);
        }
        self.send_tool_call(tool_call).await;
    }

    /// Send a tool call completion update (used for replay)
    async fn send_tool_call_completed(
        &self,
        call_id: impl Into<ToolCallId>,
        raw_output: Option<serde_json::Value>,
    ) {
        let mut fields = ToolCallUpdateFields::new().status(ToolCallStatus::Completed);
        if let Some(output) = raw_output {
            fields = fields.raw_output(output);
        }
        self.send_tool_call_update(ToolCallUpdate::new(call_id, fields))
            .await;
    }

    async fn update_plan(&self, plan: Vec<PlanItemArg>) {
        self.log_canonical(
            "acp.plan",
            json!({
                "items": plan.iter().map(|p| {
                    json!({
                        "step": p.step,
                        "status": match p.status {
                            StepStatus::Pending => "pending",
                            StepStatus::InProgress => "in_progress",
                            StepStatus::Completed => "completed",
                        }
                    })
                }).collect::<Vec<_>>(),
            }),
        );
        self.send_notification(SessionUpdate::Plan(Plan::new(
            plan.into_iter()
                .map(|entry| {
                    PlanEntry::new(
                        entry.step,
                        PlanEntryPriority::Medium,
                        match entry.status {
                            StepStatus::Pending => PlanEntryStatus::Pending,
                            StepStatus::InProgress => PlanEntryStatus::InProgress,
                            StepStatus::Completed => PlanEntryStatus::Completed,
                        },
                    )
                })
                .collect(),
        )))
        .await;
    }

    async fn request_permission(
        &self,
        tool_call: ToolCallUpdate,
        options: Vec<PermissionOption>,
    ) -> Result<RequestPermissionResponse, Error> {
        self.log_canonical(
            "acp.request_permission",
            json!({
                "tool_call": serde_json::to_value(&tool_call).unwrap_or_else(|_| json!({"debug": format!("{tool_call:?}")})),
                "options": serde_json::to_value(&options).unwrap_or_else(|_| json!({"debug": format!("{options:?}")})),
            }),
        );

        let resp = self
            .client
            .request_permission(RequestPermissionRequest::new(
                self.session_id.clone(),
                tool_call,
                options,
            ))
            .await?;

        self.log_canonical(
            "acp.request_permission_response",
            json!({
                "outcome": serde_json::to_value(&resp.outcome).unwrap_or_else(|_| json!({"debug": format!("{:?}", resp.outcome)})),
            }),
        );

        Ok(resp)
    }
}

struct ThreadActor<A> {
    /// Allows for logging out from slash commands
    auth: A,
    /// Used for sending messages back to the client.
    client: SessionClient,
    /// The thread associated with this task.
    thread: Arc<dyn CodexThreadImpl>,
    /// The configuration for the thread.
    config: Config,
    /// The custom prompts loaded for this workspace.
    custom_prompts: Rc<RefCell<Vec<CustomPrompt>>>,
    /// The models available for this thread.
    models_manager: Arc<dyn ModelsManagerImpl>,
    /// A sender for each interested `Op` submission that needs events routed.
    submissions: HashMap<String, SubmissionState>,
    /// A receiver for incoming thread messages.
    message_rx: mpsc::UnboundedReceiver<ThreadMessage>,
    /// Last config options state we emitted to the client, used for deduping updates.
    last_sent_config_options: Option<Vec<SessionConfigOption>>,
    /// Cached session list for /load lookups.
    last_session_list: Vec<SessionListEntry>,
}

impl<A: Auth> ThreadActor<A> {
    fn new(
        auth: A,
        client: SessionClient,
        thread: Arc<dyn CodexThreadImpl>,
        models_manager: Arc<dyn ModelsManagerImpl>,
        config: Config,
        message_rx: mpsc::UnboundedReceiver<ThreadMessage>,
    ) -> Self {
        Self {
            auth,
            client,
            thread,
            config,
            custom_prompts: Rc::default(),
            models_manager,
            submissions: HashMap::new(),
            message_rx,
            last_sent_config_options: None,
            last_session_list: Vec::new(),
        }
    }

    async fn spawn(mut self) {
        loop {
            tokio::select! {
                biased;
                message = self.message_rx.recv() => match message {
                    Some(message) => self.handle_message(message).await,
                    None => break,
                },
                event = self.thread.next_event() => match event {
                    Ok(event) => self.handle_event(event).await,
                    Err(e) => {
                        error!("Error getting next event: {:?}", e);
                        break;
                    }
                }
            }
            // Litter collection of senders with no receivers
            self.submissions
                .retain(|_, submission| submission.is_active());
        }
    }

    async fn handle_message(&mut self, message: ThreadMessage) {
        match message {
            ThreadMessage::Load { response_tx } => {
                let result = self.handle_load().await;
                drop(response_tx.send(result));
                let client = self.client.clone();
                let mut available_commands = Self::builtin_commands();
                let load_custom_prompts = self.load_custom_prompts().await;
                let custom_prompts = self.custom_prompts.clone();

                // Have this happen after the session is loaded by putting it
                // in a separate task
                tokio::task::spawn_local(async move {
                    let mut new_custom_prompts = load_custom_prompts
                        .await
                        .map_err(|_| Error::internal_error())
                        .flatten()
                        .inspect_err(|e| error!("Failed to load custom prompts {e:?}"))
                        .unwrap_or_default();

                    for prompt in &new_custom_prompts {
                        available_commands.push(
                            AvailableCommand::new(
                                prompt.name.clone(),
                                prompt.description.clone().unwrap_or_default(),
                            )
                            .input(prompt.argument_hint.as_ref().map(
                                |hint| {
                                    AvailableCommandInput::Unstructured(
                                        UnstructuredCommandInput::new(hint.clone()),
                                    )
                                },
                            )),
                        );
                    }
                    std::mem::swap(
                        custom_prompts.borrow_mut().deref_mut(),
                        &mut new_custom_prompts,
                    );

                    client
                        .send_notification(SessionUpdate::AvailableCommandsUpdate(
                            AvailableCommandsUpdate::new(available_commands),
                        ))
                        .await;
                });
                if let Err(err) = self.handle_sessions_command().await {
                    error!("Failed to list sessions: {err:?}");
                }
            }
            ThreadMessage::GetConfigOptions { response_tx } => {
                let result = self.config_options().await;
                drop(response_tx.send(result));
            }
            ThreadMessage::Prompt {
                request,
                response_tx,
            } => {
                let result = self.handle_prompt(request).await;
                drop(response_tx.send(result));
            }
            ThreadMessage::SetMode { mode, response_tx } => {
                let result = self.handle_set_mode(mode).await;
                drop(response_tx.send(result));
                self.maybe_emit_config_options_update().await;
            }
            ThreadMessage::SetModel { model, response_tx } => {
                let result = self.handle_set_model(model).await;
                drop(response_tx.send(result));
                self.maybe_emit_config_options_update().await;
            }
            ThreadMessage::SetConfigOption {
                config_id,
                value,
                response_tx,
            } => {
                let result = self.handle_set_config_option(config_id, value).await;
                drop(response_tx.send(result));
            }
            ThreadMessage::Cancel { response_tx } => {
                let result = self.handle_cancel().await;
                drop(response_tx.send(result));
            }
            ThreadMessage::ReplayHistory {
                history,
                response_tx,
            } => {
                let result = self.handle_replay_history(history).await;
                drop(response_tx.send(result));
            }
        }
    }

    fn builtin_commands() -> Vec<AvailableCommand> {
        vec![
            // CLI parity: expose common Codex CLI slash commands in ACP clients.
            // Commands that depend on interactive TUI menus are implemented as
            // "show config options" / "print instructions" in this adapter.
            AvailableCommand::new("model", "choose what model and reasoning effort to use"),
            AvailableCommand::new("personality", "choose a communication style for responses"),
            AvailableCommand::new("approvals", "choose what Codex can do without approval"),
            AvailableCommand::new("permissions", "choose what Codex is allowed to do"),
            AvailableCommand::new("experimental", "toggle beta features"),
            AvailableCommand::new(
                "skills",
                "use skills to improve how Codex performs specific tasks",
            ),
            AvailableCommand::new("mcp", "list configured MCP tools"),
            AvailableCommand::new("status", "show current session configuration"),
            AvailableCommand::new("new", "start a new chat during a conversation"),
            AvailableCommand::new("resume", "resume a saved chat"),
            AvailableCommand::new("fork", "fork the current chat"),
            AvailableCommand::new("diff", "show git diff"),
            AvailableCommand::new("mention", "mention a file"),
            AvailableCommand::new("feedback", "send logs to maintainers"),
            AvailableCommand::new("review", "Review my current changes and find issues").input(
                AvailableCommandInput::Unstructured(UnstructuredCommandInput::new(
                    "optional custom review instructions",
                )),
            ),
            AvailableCommand::new(
                "review-branch",
                "Review the code changes against a specific branch",
            )
            .input(AvailableCommandInput::Unstructured(
                UnstructuredCommandInput::new("branch name"),
            )),
            AvailableCommand::new(
                "review-commit",
                "Review the code changes introduced by a commit",
            )
            .input(AvailableCommandInput::Unstructured(
                UnstructuredCommandInput::new("commit sha"),
            )),
            AvailableCommand::new(
                "init",
                "create an AGENTS.md file with instructions for Codex",
            ),
            AvailableCommand::new(
                "compact",
                "summarize conversation to prevent hitting the context limit",
            ),
            AvailableCommand::new("undo", "undo Codex’s most recent turn"),
            AvailableCommand::new("sessions", "list recent sessions for the current workspace"),
            AvailableCommand::new("load", "show instructions to open a previous session").input(
                AvailableCommandInput::Unstructured(UnstructuredCommandInput::new(
                    "session id or list number",
                )),
            ),
            AvailableCommand::new("logout", "logout of Codex"),
        ]
    }

    async fn load_custom_prompts(&mut self) -> oneshot::Receiver<Result<Vec<CustomPrompt>, Error>> {
        let (response_tx, response_rx) = oneshot::channel();
        let submission_id = match self.thread.submit(Op::ListCustomPrompts).await {
            Ok(id) => id,
            Err(e) => {
                drop(response_tx.send(Err(Error::internal_error().data(e.to_string()))));
                return response_rx;
            }
        };

        self.submissions.insert(
            submission_id,
            SubmissionState::CustomPrompts(CustomPromptsState::new(response_tx)),
        );

        response_rx
    }

    fn modes(&self) -> Option<SessionModeState> {
        let current_mode_id = APPROVAL_PRESETS
            .iter()
            .find(|preset| {
                &preset.approval == self.config.approval_policy.get()
                    && &preset.sandbox == self.config.sandbox_policy.get()
            })
            .map(|preset| SessionModeId::new(preset.id))?;

        Some(SessionModeState::new(
            current_mode_id,
            APPROVAL_PRESETS
                .iter()
                .map(|preset| {
                    SessionMode::new(preset.id, preset.label).description(preset.description)
                })
                .collect(),
        ))
    }

    async fn find_current_model(&self) -> Option<ModelId> {
        let model_presets = self.models_manager.list_models(&self.config).await;
        let config_model = self.get_current_model().await;
        let preset = model_presets
            .iter()
            .find(|preset| preset.model == config_model)?;

        let effort = self
            .config
            .model_reasoning_effort
            .and_then(|effort| {
                preset
                    .supported_reasoning_efforts
                    .iter()
                    .find_map(|e| (e.effort == effort).then_some(effort))
            })
            .unwrap_or(preset.default_reasoning_effort);

        Some(Self::model_id(&preset.id, effort))
    }

    fn model_id(id: &str, effort: ReasoningEffort) -> ModelId {
        ModelId::new(format!("{id}/{effort}"))
    }

    fn parse_model_id(id: &ModelId) -> Option<(String, ReasoningEffort)> {
        let (model, reasoning) = id.0.split_once('/')?;
        let reasoning = serde_json::from_value(reasoning.into()).ok()?;
        Some((model.to_owned(), reasoning))
    }

    async fn config_options(&self) -> Result<Vec<SessionConfigOption>, Error> {
        let mut options = Vec::new();

        if let Some(modes) = self.modes() {
            let select_options = modes
                .available_modes
                .into_iter()
                .map(|m| SessionConfigSelectOption::new(m.id.0, m.name).description(m.description))
                .collect::<Vec<_>>();

            options.push(
                SessionConfigOption::select(
                    "mode",
                    "Approval Preset",
                    modes.current_mode_id.0,
                    select_options,
                )
                .category(SessionConfigOptionCategory::Mode)
                .description("Choose an approval and sandboxing preset for your session"),
            );
        }

        let presets = self.models_manager.list_models(&self.config).await;

        let current_model = self.get_current_model().await;
        let current_preset = presets.iter().find(|p| p.model == current_model).cloned();

        let mut model_select_options = Vec::new();

        if current_preset.is_none() {
            // If no preset found, return the current model string as-is
            model_select_options.push(SessionConfigSelectOption::new(
                current_model.clone(),
                current_model.clone(),
            ));
        };

        model_select_options.extend(
            presets
                .into_iter()
                .filter(|model| model.show_in_picker || model.model == current_model)
                .map(|preset| {
                    SessionConfigSelectOption::new(preset.id, preset.display_name)
                        .description(preset.description)
                }),
        );

        options.push(
            SessionConfigOption::select("model", "Model", current_model, model_select_options)
                .category(SessionConfigOptionCategory::Model)
                .description("Choose which model Codex should use"),
        );

        // Reasoning effort selector (only if the current preset exists and has >1 supported effort)
        if let Some(preset) = current_preset
            && preset.supported_reasoning_efforts.len() > 1
        {
            let supported = &preset.supported_reasoning_efforts;

            let current_effort = self
                .config
                .model_reasoning_effort
                .and_then(|effort| {
                    supported
                        .iter()
                        .find_map(|e| (e.effort == effort).then_some(effort))
                })
                .unwrap_or(preset.default_reasoning_effort);

            let effort_select_options = supported
                .iter()
                .map(|e| {
                    SessionConfigSelectOption::new(
                        e.effort.to_string(),
                        e.effort.to_string().to_title_case(),
                    )
                    .description(e.description.clone())
                })
                .collect::<Vec<_>>();

            options.push(
                SessionConfigOption::select(
                    "reasoning_effort",
                    "Reasoning Effort",
                    current_effort.to_string(),
                    effort_select_options,
                )
                .category(SessionConfigOptionCategory::ThoughtLevel)
                .description("Choose how much reasoning effort the model should use"),
            );
        }

        let current_personality = self
            .config
            .model_personality
            .map(|p| p.to_string())
            .unwrap_or_else(|| "auto".to_string());
        let personality_select_options = vec![
            SessionConfigSelectOption::new("auto", "Auto")
                .description("Use the default personality (no override)"),
            SessionConfigSelectOption::new(Personality::Friendly.to_string(), "Friendly"),
            SessionConfigSelectOption::new(Personality::Pragmatic.to_string(), "Pragmatic"),
        ];
        options.push(
            SessionConfigOption::select(
                "personality",
                "Personality",
                current_personality,
                personality_select_options,
            )
            .category(SessionConfigOptionCategory::Other)
            .description("Choose a communication style for responses"),
        );

        Ok(options)
    }

    async fn list_sessions_for_cwd(&self) -> Result<Vec<SessionListEntry>, Error> {
        let page = RolloutRecorder::list_threads(
            &self.config.codex_home,
            SESSION_LIST_PAGE_SIZE,
            None,
            ThreadSortKey::UpdatedAt,
            &[
                SessionSource::Cli,
                SessionSource::VSCode,
                SessionSource::Unknown,
            ],
            None,
            self.config.model_provider_id.as_str(),
        )
        .await
        .map_err(|err| Error::internal_error().data(format!("failed to list sessions: {err}")))?;

        let sessions = page
            .items
            .into_iter()
            .filter_map(|item| {
                let session_meta_line = item.head.first().and_then(|first| {
                    serde_json::from_value::<SessionMetaLine>(first.clone()).ok()
                })?;

                if session_meta_line.meta.cwd != self.config.cwd {
                    return None;
                }

                let mut title = None;
                for value in item.head {
                    if let Ok(response_item) = serde_json::from_value::<ResponseItem>(value)
                        && let Some(turn_item) = parse_turn_item(&response_item)
                        && let TurnItem::UserMessage(user) = turn_item
                    {
                        if let Some(formatted) = format_session_title(&user.message()) {
                            title = Some(formatted);
                        }
                        break;
                    }
                }

                let updated_at = item.updated_at.clone().or(item.created_at.clone());

                Some(SessionListEntry {
                    id: SessionId::new(session_meta_line.meta.id.to_string()),
                    title,
                    updated_at,
                })
            })
            .collect::<Vec<_>>();

        Ok(sessions)
    }

    async fn maybe_emit_config_options_update(&mut self) {
        let config_options = self.config_options().await.unwrap_or_default();

        if self
            .last_sent_config_options
            .as_ref()
            .is_some_and(|prev| prev == &config_options)
        {
            return;
        }

        self.last_sent_config_options = Some(config_options.clone());

        self.client
            .send_notification(SessionUpdate::ConfigOptionUpdate(ConfigOptionUpdate::new(
                config_options,
            )))
            .await;
    }

    async fn handle_set_config_option(
        &mut self,
        config_id: SessionConfigId,
        value: SessionConfigValueId,
    ) -> Result<(), Error> {
        match config_id.0.as_ref() {
            "mode" => self.handle_set_mode(SessionModeId::new(value.0)).await,
            "model" => self.handle_set_config_model(value).await,
            "reasoning_effort" => self.handle_set_config_reasoning_effort(value).await,
            "personality" => self.handle_set_config_personality(value).await,
            _ => Err(Error::invalid_params().data("Unsupported config option")),
        }
    }

    async fn handle_set_config_model(&mut self, value: SessionConfigValueId) -> Result<(), Error> {
        let model_id = value.0;

        let presets = self.models_manager.list_models(&self.config).await;
        let preset = presets.iter().find(|p| p.id.as_str() == &*model_id);

        let model_to_use = preset
            .map(|p| p.model.clone())
            .unwrap_or_else(|| model_id.to_string());

        if model_to_use.is_empty() {
            return Err(Error::invalid_params().data("No model selected"));
        }

        let effort_to_use = if let Some(preset) = preset {
            if let Some(effort) = self.config.model_reasoning_effort
                && preset
                    .supported_reasoning_efforts
                    .iter()
                    .any(|e| e.effort == effort)
            {
                Some(effort)
            } else {
                Some(preset.default_reasoning_effort)
            }
        } else {
            // If the user selected a raw model string (not a known preset), don't invent a default.
            // Keep whatever was previously configured (or leave unset) so Codex can decide.
            self.config.model_reasoning_effort
        };

        self.thread
            .submit(Op::OverrideTurnContext {
                cwd: None,
                approval_policy: None,
                sandbox_policy: None,
                model: Some(model_to_use.clone()),
                effort: Some(effort_to_use),
                summary: None,
                collaboration_mode: None,
                personality: None,
            })
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;

        self.config.model = Some(model_to_use);
        self.config.model_reasoning_effort = effort_to_use;

        Ok(())
    }

    async fn handle_set_config_reasoning_effort(
        &mut self,
        value: SessionConfigValueId,
    ) -> Result<(), Error> {
        let effort: ReasoningEffort =
            serde_json::from_value(value.0.as_ref().into()).map_err(|_| Error::invalid_params())?;

        let current_model = self.get_current_model().await;
        let presets = self.models_manager.list_models(&self.config).await;
        let Some(preset) = presets.iter().find(|p| p.model == current_model) else {
            return Err(Error::invalid_params()
                .data("Reasoning effort can only be set for known model presets"));
        };

        if !preset
            .supported_reasoning_efforts
            .iter()
            .any(|e| e.effort == effort)
        {
            return Err(
                Error::invalid_params().data("Unsupported reasoning effort for selected model")
            );
        }

        self.thread
            .submit(Op::OverrideTurnContext {
                cwd: None,
                approval_policy: None,
                sandbox_policy: None,
                model: None,
                effort: Some(Some(effort)),
                summary: None,
                collaboration_mode: None,
                personality: None,
            })
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;

        self.config.model_reasoning_effort = Some(effort);

        Ok(())
    }

    async fn handle_set_config_personality(
        &mut self,
        value: SessionConfigValueId,
    ) -> Result<(), Error> {
        let raw = value.0;
        if raw.as_ref() == "auto" {
            // Best-effort: protocol doesn't support clearing personality overrides via
            // OverrideTurnContext, so we only clear our local config state.
            self.config.model_personality = None;
            return Ok(());
        }

        let personality: Personality =
            serde_json::from_value(raw.as_ref().into()).map_err(|_| Error::invalid_params())?;

        self.thread
            .submit(Op::OverrideTurnContext {
                cwd: None,
                approval_policy: None,
                sandbox_policy: None,
                model: None,
                effort: None,
                summary: None,
                collaboration_mode: None,
                personality: Some(personality),
            })
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;

        self.config.model_personality = Some(personality);

        Ok(())
    }

    async fn models(&self) -> Result<SessionModelState, Error> {
        let mut available_models = Vec::new();
        let config_model = self.get_current_model().await;

        let current_model_id = if let Some(model_id) = self.find_current_model().await {
            model_id
        } else {
            // If no preset found, return the current model string as-is
            let model_id = ModelId::new(self.get_current_model().await);
            available_models.push(ModelInfo::new(model_id.clone(), model_id.to_string()));
            model_id
        };

        available_models.extend(
            self.models_manager
                .list_models(&self.config)
                .await
                .iter()
                .filter(|model| model.show_in_picker || model.model == config_model)
                .flat_map(|preset| {
                    preset.supported_reasoning_efforts.iter().map(|effort| {
                        ModelInfo::new(
                            Self::model_id(&preset.id, effort.effort),
                            format!("{} ({})", preset.display_name, effort.effort),
                        )
                        .description(format!("{} {}", preset.description, effort.description))
                    })
                }),
        );

        Ok(SessionModelState::new(current_model_id, available_models))
    }

    async fn handle_load(&mut self) -> Result<LoadSessionResponse, Error> {
        Ok(LoadSessionResponse::new()
            .models(self.models().await?)
            .modes(self.modes())
            .config_options(self.config_options().await?))
    }

    async fn handle_sessions_command(&mut self) -> Result<(), Error> {
        let sessions = self.list_sessions_for_cwd().await?;
        self.last_session_list = sessions.clone();
        let message = format_session_list_message(&self.config.cwd, &sessions);
        self.client.send_agent_text(message).await;
        Ok(())
    }

    async fn handle_load_command(&mut self, rest: &str) -> Result<(), Error> {
        let selection = rest.trim();
        if selection.is_empty() {
            self.client
                .send_agent_text("Usage: /load <session id or list number>")
                .await;
            return Ok(());
        }

        let target_id = if let Ok(index) = selection.parse::<usize>() {
            if index == 0 || index > self.last_session_list.len() {
                None
            } else {
                Some(self.last_session_list[index - 1].id.clone())
            }
        } else {
            Some(SessionId::new(selection.to_string()))
        };

        let Some(session_id) = target_id else {
            self.client
                .send_agent_text(
                    "Unknown session selection. Run /sessions and pick a valid number.",
                )
                .await;
            return Ok(());
        };

        self.client
            .send_agent_text(format!(
                "Session switching must be initiated by the ACP client. In Zed, open the thread list and select session id: {}",
                session_id
            ))
            .await;
        Ok(())
    }

    async fn handle_prompt(
        &mut self,
        request: PromptRequest,
    ) -> Result<oneshot::Receiver<Result<StopReason, Error>>, Error> {
        let (response_tx, response_rx) = oneshot::channel();

        self.client
            .log_canonical("acp.prompt", summarize_prompt_for_log(&request.prompt));

        let items = build_prompt_items(request.prompt);
        let op;
        if let Some((name, rest)) = extract_slash_command(&items) {
            match name {
                // CLI parity commands that map to ACP config options / instructions.
                "model" => {
                    self.maybe_emit_config_options_update().await;
                    let current_model = self.get_current_model().await;
                    self.client
                        .send_agent_text(format!(
                            "Current model: {current_model}\n\nUse your client configuration UI (Config Options) to change `Model` and `Reasoning Effort`."
                        ))
                        .await;
                    drop(response_tx.send(Ok(StopReason::EndTurn)));
                    return Ok(response_rx);
                }
                "personality" => {
                    self.maybe_emit_config_options_update().await;
                    let current = self
                        .config
                        .model_personality
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "auto".to_string());
                    self.client
                        .send_agent_text(format!(
                            "Current personality: {current}\n\nUse your client configuration UI (Config Options) to change `Personality`."
                        ))
                        .await;
                    drop(response_tx.send(Ok(StopReason::EndTurn)));
                    return Ok(response_rx);
                }
                "approvals" | "permissions" => {
                    self.maybe_emit_config_options_update().await;
                    self.client
                        .send_agent_text(
                            "Use your client configuration UI (Config Options) to change `Approval Preset`.\n\nNote: this adapter currently models approvals and permissions together via `Approval Preset`."
                                .to_string(),
                        )
                        .await;
                    drop(response_tx.send(Ok(StopReason::EndTurn)));
                    return Ok(response_rx);
                }
                "experimental" => {
                    self.client
                        .send_agent_text(
                            "This CLI command depends on an interactive menu and is not yet supported in ACP.\nIf you have a specific feature you want toggled, say which and we can wire it up."
                                .to_string(),
                        )
                        .await;
                    drop(response_tx.send(Ok(StopReason::EndTurn)));
                    return Ok(response_rx);
                }
                "status" => {
                    self.maybe_emit_config_options_update().await;
                    let current_model = self.get_current_model().await;
                    let approval_preset = self
                        .modes()
                        .map(|m| m.current_mode_id.0.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let effort = self
                        .config
                        .model_reasoning_effort
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "auto".to_string());
                    let personality = self
                        .config
                        .model_personality
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "auto".to_string());
                    self.client
                        .send_agent_text(format!(
                            "Session status:\n- model: {current_model}\n- reasoning_effort: {effort}\n- personality: {personality}\n- approval_preset: {approval_preset}"
                        ))
                        .await;
                    drop(response_tx.send(Ok(StopReason::EndTurn)));
                    return Ok(response_rx);
                }
                "new" | "resume" | "fork" | "agent" => {
                    self.client
                        .send_agent_text(
                            "Session/thread switching must be initiated by the ACP client.\nIn Zed, use the Agent Panel thread list to start a new thread or pick a previous session."
                                .to_string(),
                        )
                        .await;
                    drop(response_tx.send(Ok(StopReason::EndTurn)));
                    return Ok(response_rx);
                }
                "mention" => {
                    self.client
                        .send_agent_text(
                            "Use `@file` mentions (or attach files) from your ACP client. This adapter already supports embedded context."
                                .to_string(),
                        )
                        .await;
                    drop(response_tx.send(Ok(StopReason::EndTurn)));
                    return Ok(response_rx);
                }
                "feedback" => {
                    self.client
                        .send_agent_text(
                            "In ACP, there is no built-in log upload flow yet. If you want, paste the relevant snippet from `logs/codex_chats/...` (redact secrets) and we can triage."
                                .to_string(),
                        )
                        .await;
                    drop(response_tx.send(Ok(StopReason::EndTurn)));
                    return Ok(response_rx);
                }
                "mcp" => op = Op::ListMcpTools,
                "skills" => {
                    op = Op::ListSkills {
                        cwds: vec![],
                        force_reload: false,
                    }
                }
                "diff" => {
                    // Best-effort: run git diff in the configured cwd. Output will stream through
                    // ExecCommand events like other command executions.
                    op = Op::RunUserShellCommand {
                        command: "git diff --no-color --".to_string(),
                    }
                }
                "compact" => op = Op::Compact,
                "undo" => op = Op::Undo,
                "sessions" => {
                    self.handle_sessions_command().await?;
                    drop(response_tx.send(Ok(StopReason::EndTurn)));
                    return Ok(response_rx);
                }
                "load" => {
                    self.handle_load_command(rest).await?;
                    drop(response_tx.send(Ok(StopReason::EndTurn)));
                    return Ok(response_rx);
                }
                "init" => {
                    op = Op::UserInput {
                        items: vec![UserInput::Text {
                            text: INIT_COMMAND_PROMPT.into(),
                            text_elements: vec![],
                        }],
                        final_output_json_schema: None,
                    }
                }
                "review" => {
                    let instructions = rest.trim();
                    let target = if instructions.is_empty() {
                        ReviewTarget::UncommittedChanges
                    } else {
                        ReviewTarget::Custom {
                            instructions: instructions.to_owned(),
                        }
                    };

                    op = Op::Review {
                        review_request: ReviewRequest {
                            user_facing_hint: Some(user_facing_hint(&target)),
                            target,
                        },
                    }
                }
                "review-branch" if !rest.is_empty() => {
                    let target = ReviewTarget::BaseBranch {
                        branch: rest.trim().to_owned(),
                    };
                    op = Op::Review {
                        review_request: ReviewRequest {
                            user_facing_hint: Some(user_facing_hint(&target)),
                            target,
                        },
                    }
                }
                "review-commit" if !rest.is_empty() => {
                    let target = ReviewTarget::Commit {
                        sha: rest.trim().to_owned(),
                        title: None,
                    };
                    op = Op::Review {
                        review_request: ReviewRequest {
                            user_facing_hint: Some(user_facing_hint(&target)),
                            target,
                        },
                    }
                }
                "logout" => {
                    self.auth.logout()?;
                    return Err(Error::auth_required());
                }
                _ => {
                    if let Some(prompt) =
                        expand_custom_prompt(name, rest, self.custom_prompts.borrow().as_ref())
                            .map_err(|e| Error::invalid_params().data(e.user_message()))?
                    {
                        op = Op::UserInput {
                            items: vec![UserInput::Text {
                                text: prompt,
                                text_elements: vec![],
                            }],
                            final_output_json_schema: None,
                        }
                    } else {
                        op = Op::UserInput {
                            items,
                            final_output_json_schema: None,
                        }
                    }
                }
            }
        } else {
            op = Op::UserInput {
                items,
                final_output_json_schema: None,
            }
        }

        let submission_id = self
            .thread
            .submit(op.clone())
            .await
            .map_err(|e| Error::internal_error().data(e.to_string()))?;

        info!("Submitted prompt with submission_id: {submission_id}");
        info!("Starting to wait for conversation events for submission_id: {submission_id}");

        self.client.log_canonical(
            "backend.codex.submit",
            json!({
                "submission_id": submission_id,
                "op_kind": op_kind_for_log(&op),
            }),
        );

        let state = match op {
            Op::Compact | Op::Undo => SubmissionState::Task(TaskState::new(
                self.thread.clone(),
                response_tx,
                submission_id.clone(),
            )),
            Op::ListMcpTools => SubmissionState::OneShot(OneShotCommandState::new(
                OneShotKind::McpTools,
                response_tx,
            )),
            Op::ListSkills { .. } => {
                SubmissionState::OneShot(OneShotCommandState::new(OneShotKind::Skills, response_tx))
            }
            _ => SubmissionState::Prompt(PromptState::new(
                self.thread.clone(),
                response_tx,
                submission_id.clone(),
            )),
        };

        self.submissions.insert(submission_id, state);

        Ok(response_rx)
    }

    async fn handle_set_mode(&mut self, mode: SessionModeId) -> Result<(), Error> {
        let preset = APPROVAL_PRESETS
            .iter()
            .find(|preset| mode.0.as_ref() == preset.id)
            .ok_or_else(Error::invalid_params)?;

        self.thread
            .submit(Op::OverrideTurnContext {
                cwd: None,
                approval_policy: Some(preset.approval),
                sandbox_policy: Some(preset.sandbox.clone()),
                model: None,
                effort: None,
                summary: None,
                collaboration_mode: None,
                personality: None,
            })
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;

        self.config
            .approval_policy
            .set(preset.approval)
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;
        self.config
            .sandbox_policy
            .set(preset.sandbox.clone())
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;

        match preset.sandbox {
            // Treat this user action as a trusted dir
            SandboxPolicy::DangerFullAccess
            | SandboxPolicy::WorkspaceWrite { .. }
            | SandboxPolicy::ExternalSandbox { .. } => {
                set_project_trust_level(
                    &self.config.codex_home,
                    &self.config.cwd,
                    TrustLevel::Trusted,
                )?;
            }
            SandboxPolicy::ReadOnly => {}
        }

        Ok(())
    }

    async fn get_current_model(&self) -> String {
        self.models_manager
            .get_model(&self.config.model, &self.config)
            .await
    }

    async fn handle_set_model(&mut self, model: ModelId) -> Result<(), Error> {
        // Try parsing as preset format, otherwise use as-is, fallback to config
        let (model_to_use, effort_to_use) = if let Some((m, e)) = Self::parse_model_id(&model) {
            (m, Some(e))
        } else {
            let model_str = model.0.to_string();
            let fallback = if !model_str.is_empty() {
                model_str
            } else {
                self.get_current_model().await
            };
            (fallback, self.config.model_reasoning_effort)
        };

        if model_to_use.is_empty() {
            return Err(Error::invalid_params().data("No model parsed or configured"));
        }

        self.thread
            .submit(Op::OverrideTurnContext {
                cwd: None,
                approval_policy: None,
                sandbox_policy: None,
                model: Some(model_to_use.clone()),
                effort: Some(effort_to_use),
                summary: None,
                collaboration_mode: None,
                personality: None,
            })
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;

        self.config.model = Some(model_to_use);
        self.config.model_reasoning_effort = effort_to_use;

        Ok(())
    }

    async fn handle_cancel(&mut self) -> Result<(), Error> {
        self.thread
            .submit(Op::Interrupt)
            .await
            .map_err(|e| Error::from(anyhow::anyhow!(e)))?;
        Ok(())
    }

    /// Replay conversation history to the client via session/update notifications.
    /// This is called when loading a session to stream all prior messages.
    ///
    /// We process both `EventMsg` and `ResponseItem`:
    /// - `EventMsg` for user/agent messages and reasoning (like the TUI does)
    /// - `ResponseItem` for tool calls only (not persisted as EventMsg)
    async fn handle_replay_history(&mut self, history: Vec<RolloutItem>) -> Result<(), Error> {
        for item in history {
            match item {
                RolloutItem::EventMsg(event_msg) => {
                    self.replay_event_msg(&event_msg).await;
                }
                RolloutItem::ResponseItem(response_item) => {
                    self.replay_response_item(&response_item).await;
                }
                // Skip SessionMeta, TurnContext, Compacted
                _ => {}
            }
        }
        Ok(())
    }

    /// Convert and send an EventMsg as ACP notification(s) during replay.
    /// Handles messages and reasoning - mirrors the live event handling in PromptState.
    async fn replay_event_msg(&self, msg: &EventMsg) {
        match msg {
            EventMsg::UserMessage(UserMessageEvent { message, .. }) => {
                self.client.send_user_message(message.clone()).await;
            }
            EventMsg::AgentMessage(AgentMessageEvent { message }) => {
                self.client.send_agent_text(message.clone()).await;
            }
            EventMsg::AgentReasoning(AgentReasoningEvent { text }) => {
                self.client.send_agent_thought(text.clone()).await;
            }
            EventMsg::AgentReasoningRawContent(AgentReasoningRawContentEvent { text }) => {
                self.client.send_agent_thought(text.clone()).await;
            }
            // Skip other event types during replay - they either:
            // - Are transient (deltas, turn lifecycle)
            // - Don't have direct ACP equivalents
            // - Are handled via ResponseItem instead
            _ => {}
        }
    }

    /// Parse apply_patch call input to extract patch content for display.
    /// Returns (title, locations, content) if successful.
    /// For CustomToolCall, the input is the patch string directly.
    fn parse_apply_patch_call(
        &self,
        input: &str,
    ) -> Option<(String, Vec<ToolCallLocation>, Vec<ToolCallContent>)> {
        // Try to parse the patch using codex-apply-patch parser
        let parsed = parse_patch(input).ok()?;

        let mut locations = Vec::new();
        let mut file_names = Vec::new();
        let mut content = Vec::new();

        for hunk in &parsed.hunks {
            match hunk {
                codex_apply_patch::Hunk::AddFile { path, contents } => {
                    let full_path = self.config.cwd.join(path);
                    file_names.push(path.display().to_string());
                    locations.push(ToolCallLocation::new(full_path.clone()));
                    // New file: no old_text, new_text is the contents
                    content.push(ToolCallContent::Diff(Diff::new(
                        full_path,
                        contents.clone(),
                    )));
                }
                codex_apply_patch::Hunk::DeleteFile { path } => {
                    let full_path = self.config.cwd.join(path);
                    file_names.push(path.display().to_string());
                    locations.push(ToolCallLocation::new(full_path.clone()));
                    // Delete file: old_text would be original content, new_text is empty
                    content.push(ToolCallContent::Diff(
                        Diff::new(full_path, "").old_text("[file deleted]"),
                    ));
                }
                codex_apply_patch::Hunk::UpdateFile {
                    path,
                    move_path,
                    chunks,
                } => {
                    let full_path = self.config.cwd.join(path);
                    let dest_path = move_path
                        .as_ref()
                        .map(|p| self.config.cwd.join(p))
                        .unwrap_or_else(|| full_path.clone());
                    file_names.push(path.display().to_string());
                    locations.push(ToolCallLocation::new(dest_path.clone()));

                    // Build old and new text from chunks
                    let old_lines: Vec<String> = chunks
                        .iter()
                        .flat_map(|c| c.old_lines.iter().cloned())
                        .collect();
                    let new_lines: Vec<String> = chunks
                        .iter()
                        .flat_map(|c| c.new_lines.iter().cloned())
                        .collect();

                    content.push(ToolCallContent::Diff(
                        Diff::new(dest_path, new_lines.join("\n")).old_text(old_lines.join("\n")),
                    ));
                }
            }
        }

        let title = if file_names.is_empty() {
            "Apply patch".to_string()
        } else {
            format!("Edit {}", file_names.join(", "))
        };

        Some((title, locations, content))
    }

    /// Parse shell function call arguments to extract command info for rich display.
    /// Returns (title, kind, locations) if successful.
    ///
    /// Handles both:
    /// - `shell` / `container.exec`: `command` is `Vec<String>`
    /// - `shell_command`: `command` is a `String` (shell script)
    fn parse_shell_function_call(
        &self,
        name: &str,
        arguments: &str,
    ) -> Option<(String, ToolKind, Vec<ToolCallLocation>)> {
        // Extract command and workdir based on tool type
        let (command_vec, workdir): (Vec<String>, Option<String>) = if name == "shell_command" {
            // shell_command: command is a string (shell script)
            #[derive(serde::Deserialize)]
            struct ShellCommandArgs {
                command: String,
                #[serde(default)]
                workdir: Option<String>,
            }
            let args: ShellCommandArgs = serde_json::from_str(arguments).ok()?;
            // Wrap in bash -lc for parsing
            (
                vec!["bash".to_string(), "-lc".to_string(), args.command],
                args.workdir,
            )
        } else {
            // shell / container.exec: command is Vec<String>
            #[derive(serde::Deserialize)]
            struct ShellArgs {
                command: Vec<String>,
                #[serde(default)]
                workdir: Option<String>,
            }
            let args: ShellArgs = serde_json::from_str(arguments).ok()?;
            (args.command, args.workdir)
        };

        let cwd = workdir
            .map(PathBuf::from)
            .unwrap_or_else(|| self.config.cwd.clone());

        let parsed_cmd = parse_command(&command_vec);
        let ParseCommandToolCall {
            title,
            file_extension: _,
            terminal_output: _,
            locations,
            kind,
        } = parse_command_tool_call(parsed_cmd, &cwd);

        Some((title, kind, locations))
    }

    /// Convert and send a single ResponseItem as ACP notification(s) during replay.
    /// Only handles tool calls - messages/reasoning are handled via EventMsg.
    async fn replay_response_item(&self, item: &ResponseItem) {
        match item {
            // Skip Message and Reasoning - these are handled via EventMsg
            ResponseItem::Message { .. } | ResponseItem::Reasoning { .. } => {}
            ResponseItem::FunctionCall {
                name,
                arguments,
                call_id,
                ..
            } => {
                // Check if this is a shell command - parse it like we do for LocalShellCall
                if matches!(name.as_str(), "shell" | "container.exec" | "shell_command")
                    && let Some((title, kind, locations)) =
                        self.parse_shell_function_call(name, arguments)
                {
                    self.client
                        .send_tool_call(
                            ToolCall::new(call_id.clone(), title)
                                .kind(kind)
                                .status(ToolCallStatus::Completed)
                                .locations(locations)
                                .raw_input(
                                    serde_json::from_str::<serde_json::Value>(arguments).ok(),
                                ),
                        )
                        .await;
                    return;
                }

                // Fall through to generic function call handling
                self.client
                    .send_completed_tool_call(
                        call_id.clone(),
                        name.clone(),
                        ToolKind::Other,
                        serde_json::from_str(arguments).ok(),
                    )
                    .await;
            }
            ResponseItem::FunctionCallOutput { call_id, output } => {
                self.client
                    .send_tool_call_completed(
                        call_id.clone(),
                        serde_json::to_value(&output.content).ok(),
                    )
                    .await;
            }
            ResponseItem::LocalShellCall {
                call_id: Some(call_id),
                action,
                status,
                ..
            } => {
                let codex_protocol::models::LocalShellAction::Exec(exec) = action;
                let cwd = exec
                    .working_directory
                    .as_ref()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| self.config.cwd.clone());

                // Parse the command to get rich info like the live event handler does
                let parsed_cmd = parse_command(&exec.command);
                let ParseCommandToolCall {
                    title,
                    file_extension: _,
                    terminal_output: _,
                    locations,
                    kind,
                } = parse_command_tool_call(parsed_cmd, &cwd);

                let tool_status = match status {
                    codex_protocol::models::LocalShellStatus::Completed => {
                        ToolCallStatus::Completed
                    }
                    codex_protocol::models::LocalShellStatus::InProgress
                    | codex_protocol::models::LocalShellStatus::Incomplete => {
                        ToolCallStatus::Failed
                    }
                };
                self.client
                    .send_tool_call(
                        ToolCall::new(call_id.clone(), title)
                            .kind(kind)
                            .status(tool_status)
                            .locations(locations),
                    )
                    .await;
            }
            ResponseItem::CustomToolCall {
                name,
                input,
                call_id,
                ..
            } => {
                // Check if this is an apply_patch call - show the patch content
                if name == "apply_patch"
                    && let Some((title, locations, content)) = self.parse_apply_patch_call(input)
                {
                    self.client
                        .send_tool_call(
                            ToolCall::new(call_id.clone(), title)
                                .kind(ToolKind::Edit)
                                .status(ToolCallStatus::Completed)
                                .locations(locations)
                                .content(content)
                                .raw_input(serde_json::from_str::<serde_json::Value>(input).ok()),
                        )
                        .await;
                    return;
                }

                // Fall through to generic custom tool call handling
                self.client
                    .send_completed_tool_call(
                        call_id.clone(),
                        name.clone(),
                        ToolKind::Other,
                        serde_json::from_str(input).ok(),
                    )
                    .await;
            }
            ResponseItem::CustomToolCallOutput { call_id, output } => {
                self.client
                    .send_tool_call_completed(
                        call_id.clone(),
                        Some(serde_json::Value::String(output.clone())),
                    )
                    .await;
            }
            ResponseItem::WebSearchCall { id, action, .. } => {
                let (title, call_id) = web_search_action_to_title_and_id(id, action);
                let Some(call_id) = call_id else {
                    return; // Skip unknown web search actions
                };
                self.client
                    .send_tool_call(
                        ToolCall::new(call_id, title)
                            .kind(ToolKind::Search)
                            .status(ToolCallStatus::Completed),
                    )
                    .await;
            }
            // Skip GhostSnapshot, Compaction, Other, LocalShellCall without call_id
            _ => {}
        }
    }

    async fn handle_event(&mut self, Event { id, msg }: Event) {
        if let Some(submission) = self.submissions.get_mut(&id) {
            submission.handle_event(&self.client, msg).await;
        } else {
            warn!("Received event for unknown submission ID: {id} {msg:?}");
        }
    }
}

fn build_prompt_items(prompt: Vec<ContentBlock>) -> Vec<UserInput> {
    prompt
        .into_iter()
        .filter_map(|block| match block {
            ContentBlock::Text(text_block) => Some(UserInput::Text {
                text: text_block.text,
                text_elements: vec![],
            }),
            ContentBlock::Image(image_block) => Some(UserInput::Image {
                image_url: format!("data:{};base64,{}", image_block.mime_type, image_block.data),
            }),
            ContentBlock::ResourceLink(ResourceLink { name, uri, .. }) => Some(UserInput::Text {
                text: format_uri_as_link(Some(name), uri),
                text_elements: vec![],
            }),
            ContentBlock::Resource(EmbeddedResource {
                resource:
                    EmbeddedResourceResource::TextResourceContents(TextResourceContents {
                        text,
                        uri,
                        ..
                    }),
                ..
            }) => Some(UserInput::Text {
                text: format!(
                    "{}\n<context ref=\"{uri}\">\n{text}\n</context>",
                    format_uri_as_link(None, uri.clone())
                ),
                text_elements: vec![],
            }),
            // Skip other content types for now
            ContentBlock::Audio(..) | ContentBlock::Resource(..) | _ => None,
        })
        .collect()
}

fn summarize_prompt_for_log(prompt: &[ContentBlock]) -> serde_json::Value {
    let log_embedded_context = std::env::var("ACP_LOG_EMBEDDED_CONTEXT")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    let max_text_chars: usize = std::env::var("ACP_LOG_MAX_TEXT_CHARS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(16_384);

    let mut text_blocks: Vec<String> = Vec::new();
    let mut resource_links: Vec<serde_json::Value> = Vec::new();
    let mut embedded_text_resources: Vec<serde_json::Value> = Vec::new();

    let mut image_count = 0usize;
    let mut audio_count = 0usize;
    let mut other_count = 0usize;

    for block in prompt {
        match block {
            ContentBlock::Text(t) => text_blocks.push(t.text.clone()),
            ContentBlock::ResourceLink(ResourceLink { name, uri, .. }) => {
                resource_links.push(json!({ "name": name, "uri": uri }));
            }
            ContentBlock::Resource(EmbeddedResource {
                resource:
                    EmbeddedResourceResource::TextResourceContents(TextResourceContents {
                        text,
                        uri,
                        ..
                    }),
                ..
            }) => {
                // Avoid duplicating large / sensitive context by default.
                let mut item = json!({
                    "uri": uri,
                    "text_len": text.len(),
                    "included": log_embedded_context,
                });

                if log_embedded_context {
                    let mut s = text.clone();
                    if s.chars().count() > max_text_chars {
                        s = s.chars().take(max_text_chars).collect::<String>() + "\n...[truncated]";
                    }
                    item["text"] = serde_json::Value::String(s);
                }
                embedded_text_resources.push(item);
            }
            ContentBlock::Image(_) => image_count += 1,
            ContentBlock::Audio(_) => audio_count += 1,
            _ => other_count += 1,
        }
    }

    let mut text = text_blocks.join("\n");
    if text.chars().count() > max_text_chars {
        text = text.chars().take(max_text_chars).collect::<String>() + "\n...[truncated]";
    }

    json!({
        "block_count": prompt.len(),
        "text": text,
        "resource_links": resource_links,
        "embedded_text_resources": embedded_text_resources,
        "image_count": image_count,
        "audio_count": audio_count,
        "other_count": other_count,
    })
}

fn op_kind_for_log(op: &Op) -> &'static str {
    match op {
        Op::UserInput { .. } => "user_input",
        Op::Review { .. } => "review",
        Op::Compact => "compact",
        Op::Undo => "undo",
        Op::ListMcpTools => "list_mcp_tools",
        Op::ListSkills { .. } => "list_skills",
        Op::RunUserShellCommand { .. } => "run_shell_command",
        Op::ListCustomPrompts => "list_custom_prompts",
        _ => "other",
    }
}

fn format_uri_as_link(name: Option<String>, uri: String) -> String {
    if let Some(name) = name
        && !name.is_empty()
    {
        format!("[@{name}]({uri})")
    } else if let Some(path) = uri.strip_prefix("file://") {
        let name = path.split('/').next_back().unwrap_or(path);
        format!("[@{name}]({uri})")
    } else if uri.starts_with("zed://") {
        let name = uri.split('/').next_back().unwrap_or(&uri);
        format!("[@{name}]({uri})")
    } else {
        uri
    }
}

fn codex_content_to_acp_content(content: mcp_types::ContentBlock) -> ToolCallContent {
    ToolCallContent::Content(Content::new(match content {
        mcp_types::ContentBlock::TextContent(mcp_types::TextContent {
            annotations, text, ..
        }) => ContentBlock::Text(
            TextContent::new(text).annotations(annotations.map(convert_annotations)),
        ),
        mcp_types::ContentBlock::ImageContent(mcp_types::ImageContent {
            annotations,
            data,
            mime_type,
            ..
        }) => ContentBlock::Image(
            ImageContent::new(data, mime_type).annotations(annotations.map(convert_annotations)),
        ),
        mcp_types::ContentBlock::AudioContent(mcp_types::AudioContent {
            annotations,
            data,
            mime_type,
            ..
        }) => ContentBlock::Audio(
            AudioContent::new(data, mime_type).annotations(annotations.map(convert_annotations)),
        ),
        mcp_types::ContentBlock::ResourceLink(mcp_types::ResourceLink {
            annotations,
            description,
            mime_type,
            name,
            size,
            title,
            uri,
            ..
        }) => ContentBlock::ResourceLink(
            ResourceLink::new(name, uri)
                .annotations(annotations.map(convert_annotations))
                .description(description)
                .mime_type(mime_type)
                .size(size)
                .title(title),
        ),
        mcp_types::ContentBlock::EmbeddedResource(mcp_types::EmbeddedResource {
            annotations,
            resource,
            ..
        }) => {
            let resource = match resource {
                mcp_types::EmbeddedResourceResource::TextResourceContents(
                    mcp_types::TextResourceContents {
                        mime_type,
                        text,
                        uri,
                    },
                ) => EmbeddedResourceResource::TextResourceContents(
                    TextResourceContents::new(text, uri).mime_type(mime_type),
                ),
                mcp_types::EmbeddedResourceResource::BlobResourceContents(
                    mcp_types::BlobResourceContents {
                        blob,
                        mime_type,
                        uri,
                    },
                ) => EmbeddedResourceResource::BlobResourceContents(
                    BlobResourceContents::new(blob, uri).mime_type(mime_type),
                ),
            };
            ContentBlock::Resource(
                EmbeddedResource::new(resource).annotations(annotations.map(convert_annotations)),
            )
        }
    }))
}

fn convert_annotations(
    mcp_types::Annotations {
        audience,
        last_modified,
        priority,
    }: mcp_types::Annotations,
) -> Annotations {
    Annotations::new()
        .audience(audience.map(|a| {
            a.into_iter()
                .map(|audience| match audience {
                    mcp_types::Role::Assistant => agent_client_protocol::Role::Assistant,
                    mcp_types::Role::User => agent_client_protocol::Role::User,
                })
                .collect::<Vec<_>>()
        }))
        .last_modified(last_modified)
        .priority(priority)
}

fn extract_tool_call_content_from_changes(
    changes: HashMap<PathBuf, FileChange>,
) -> (
    String,
    Vec<ToolCallLocation>,
    impl Iterator<Item = ToolCallContent>,
) {
    (
        format!(
            "Edit {}",
            changes.keys().map(|p| p.display().to_string()).join(", ")
        ),
        changes.keys().map(ToolCallLocation::new).collect(),
        changes.into_iter().map(|(path, change)| {
            ToolCallContent::Diff(match change {
                codex_core::protocol::FileChange::Add { content } => Diff::new(path, content),
                codex_core::protocol::FileChange::Delete { content } => {
                    Diff::new(path, String::new()).old_text(content)
                }
                codex_core::protocol::FileChange::Update {
                    unified_diff: _,
                    move_path,
                    old_content,
                    new_content,
                } => Diff::new(move_path.unwrap_or(path), new_content).old_text(old_content),
            })
        }),
    )
}

/// Extract title and call_id from a WebSearchAction (used for replay)
fn web_search_action_to_title_and_id(
    id: &Option<String>,
    action: &codex_protocol::models::WebSearchAction,
) -> (String, Option<String>) {
    match action {
        codex_protocol::models::WebSearchAction::Search { query } => {
            let title = query.clone().unwrap_or_else(|| "Web search".to_string());
            let call_id = id
                .clone()
                .or_else(|| Some(generate_fallback_id("web_search")));
            (title, call_id)
        }
        codex_protocol::models::WebSearchAction::OpenPage { url } => {
            let title = url.clone().unwrap_or_else(|| "Open page".to_string());
            let call_id = id
                .clone()
                .or_else(|| Some(generate_fallback_id("web_open")));
            (title, call_id)
        }
        codex_protocol::models::WebSearchAction::FindInPage { pattern, .. } => {
            let title = pattern
                .clone()
                .unwrap_or_else(|| "Find in page".to_string());
            let call_id = id
                .clone()
                .or_else(|| Some(generate_fallback_id("web_find")));
            (title, call_id)
        }
        codex_protocol::models::WebSearchAction::Other => ("Unknown".to_string(), None),
    }
}

/// Generate a fallback ID using UUID (used when id is missing)
fn generate_fallback_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4())
}

fn truncate_graphemes(text: &str, max_graphemes: usize) -> String {
    let mut graphemes = text.grapheme_indices(true);

    if let Some((byte_index, _)) = graphemes.nth(max_graphemes) {
        if max_graphemes >= 3 {
            let mut truncate_graphemes = text.grapheme_indices(true);
            if let Some((truncate_byte_index, _)) = truncate_graphemes.nth(max_graphemes - 3) {
                let truncated = &text[..truncate_byte_index];
                format!("{truncated}...")
            } else {
                text.to_string()
            }
        } else {
            let truncated = &text[..byte_index];
            truncated.to_string()
        }
    } else {
        text.to_string()
    }
}

fn format_session_title(message: &str) -> Option<String> {
    let normalized = message.replace(['\r', '\n'], " ");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(truncate_graphemes(trimmed, SESSION_TITLE_MAX_GRAPHEMES))
    }
}

fn format_session_list_message(cwd: &Path, sessions: &[SessionListEntry]) -> String {
    if sessions.is_empty() {
        return format!(
            "No previous sessions found for {}.\nStart chatting to create one.",
            cwd.display()
        );
    }

    let mut lines = Vec::with_capacity(sessions.len() + 2);
    lines.push(format!("Sessions for {}:", cwd.display()));
    for (index, entry) in sessions.iter().enumerate() {
        let title = entry.title.as_deref().unwrap_or("(untitled)");
        let updated = entry.updated_at.as_deref().unwrap_or("unknown");
        lines.push(format!(
            "{}) {} [id: {}] (updated: {})",
            index + 1,
            title,
            entry.id,
            updated
        ));
    }
    lines.push("Use /load <id or number> to show how to open a previous session.".to_string());
    lines.join("\n")
}

fn format_mcp_tools_message(event: &codex_core::protocol::McpListToolsResponseEvent) -> String {
    let mut tool_names = event.tools.keys().cloned().collect::<Vec<_>>();
    tool_names.sort();

    let mut lines = Vec::new();
    if tool_names.is_empty() {
        lines.push("No MCP tools configured.".to_string());
    } else {
        lines.push(format!("MCP tools ({}):", tool_names.len()));
        lines.extend(tool_names.into_iter().map(|name| format!("- {name}")));
    }

    if !event.auth_statuses.is_empty() {
        let mut statuses = event
            .auth_statuses
            .iter()
            .map(|(server, status)| format!("{server}: {status}"))
            .collect::<Vec<_>>();
        statuses.sort();
        lines.push(String::new());
        lines.push("MCP auth:".to_string());
        lines.extend(statuses.into_iter().map(|s| format!("- {s}")));
    }

    lines.join("\n")
}

fn format_skills_message(event: &codex_core::protocol::ListSkillsResponseEvent) -> String {
    let mut lines = Vec::new();
    if event.skills.is_empty() {
        return "No skills found.".to_string();
    }

    for entry in &event.skills {
        lines.push(format!("Skills for {}:", entry.cwd.display()));
        let mut skills = entry.skills.clone();
        skills.sort_by(|a, b| a.name.cmp(&b.name));

        if skills.is_empty() {
            lines.push("- (none)".to_string());
        } else {
            for skill in skills {
                let enabled = if skill.enabled { "enabled" } else { "disabled" };
                lines.push(format!(
                    "- {} ({:?}, {enabled}): {}",
                    skill.name, skill.scope, skill.description
                ));
            }
        }

        if !entry.errors.is_empty() {
            lines.push("Skill errors:".to_string());
            for err in &entry.errors {
                lines.push(format!("- {}: {}", err.path.display(), err.message));
            }
        }

        lines.push(String::new());
    }

    // Remove trailing blank line for cleaner display
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

/// Checks if a prompt is slash command
fn extract_slash_command(content: &[UserInput]) -> Option<(&str, &str)> {
    let line = content.first().and_then(|block| match block {
        UserInput::Text { text, .. } => Some(text),
        _ => None,
    })?;

    parse_slash_name(line)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use codex_core::{
        config::ConfigOverrides, models_manager::model_presets::all_model_presets,
        protocol::AgentMessageEvent,
    };
    use tokio::{
        sync::{Mutex, mpsc::UnboundedSender},
        task::LocalSet,
    };

    use super::*;

    #[tokio::test]
    async fn test_prompt() -> anyhow::Result<()> {
        let (session_id, client, _, message_tx, local_set) = setup(vec![]).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(session_id.clone(), vec!["Hi".into()]),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(matches!(
            &notifications[0].update,
            SessionUpdate::AgentMessageChunk(ContentChunk {
                content: ContentBlock::Text(TextContent { text, .. }),
                ..
            }) if text == "Hi"
        ));

        Ok(())
    }

    #[tokio::test]
    async fn test_slash_command_smoke_flow() -> anyhow::Result<()> {
        let (session_id, client, thread, message_tx, local_set) = setup(vec![]).await?;

        async fn send_prompt(
            message_tx: &UnboundedSender<ThreadMessage>,
            session_id: &SessionId,
            prompt: &str,
        ) -> anyhow::Result<StopReason> {
            let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();
            message_tx.send(ThreadMessage::Prompt {
                request: PromptRequest::new(session_id.clone(), vec![prompt.into()]),
                response_tx: prompt_response_tx,
            })?;
            Ok(prompt_response_rx.await??.await??)
        }

        // The thread actor runs on a tokio LocalSet. Drive the LocalSet concurrently with
        // sending prompts, otherwise the test will deadlock waiting for responses.
        tokio::try_join!(
            async {
                // Basic end-to-end smoke: CLI parity slash commands work and can be chained.
                assert_eq!(
                    send_prompt(&message_tx, &session_id, "/init").await?,
                    StopReason::EndTurn
                );
                assert_eq!(
                    send_prompt(&message_tx, &session_id, "Hello").await?,
                    StopReason::EndTurn
                );
                assert_eq!(
                    send_prompt(&message_tx, &session_id, "/review").await?,
                    StopReason::EndTurn
                );
                assert_eq!(
                    send_prompt(&message_tx, &session_id, "/compact").await?,
                    StopReason::EndTurn
                );

                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        let texts: Vec<String> = notifications
            .iter()
            .map(|n| match &n.update {
                SessionUpdate::AgentMessageChunk(ContentChunk {
                    content: ContentBlock::Text(TextContent { text, .. }),
                    ..
                }) => text.clone(),
                other => panic!("Unexpected notification update: {other:?}"),
            })
            .collect();

        assert_eq!(
            texts.as_slice(),
            &[
                INIT_COMMAND_PROMPT.to_string(),
                "Hello".to_string(),
                "current changes".to_string(),
                "Compact task completed".to_string(),
            ]
        );

        let ops = thread.ops.lock().unwrap();
        assert_eq!(
            ops.as_slice(),
            &[
                Op::UserInput {
                    items: vec![UserInput::Text {
                        text: INIT_COMMAND_PROMPT.to_string(),
                        text_elements: vec![]
                    }],
                    final_output_json_schema: None,
                },
                Op::UserInput {
                    items: vec![UserInput::Text {
                        text: "Hello".to_string(),
                        text_elements: vec![]
                    }],
                    final_output_json_schema: None,
                },
                Op::Review {
                    review_request: ReviewRequest {
                        user_facing_hint: Some(user_facing_hint(&ReviewTarget::UncommittedChanges)),
                        target: ReviewTarget::UncommittedChanges,
                    }
                },
                Op::Compact,
            ],
            "ops don't match {ops:?}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_compact() -> anyhow::Result<()> {
        let (session_id, client, thread, message_tx, local_set) = setup(vec![]).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(session_id.clone(), vec!["/compact".into()]),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(matches!(
            &notifications[0].update,
            SessionUpdate::AgentMessageChunk(ContentChunk {
                content: ContentBlock::Text(TextContent { text, .. }),
                ..
            }) if text == "Compact task completed"
        ));
        let ops = thread.ops.lock().unwrap();
        assert_eq!(ops.as_slice(), &[Op::Compact]);

        Ok(())
    }

    #[tokio::test]
    async fn test_undo() -> anyhow::Result<()> {
        let (session_id, client, thread, message_tx, local_set) = setup(vec![]).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(session_id.clone(), vec!["/undo".into()]),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        assert_eq!(
            notifications.len(),
            2,
            "notifications don't match {notifications:?}"
        );
        assert!(matches!(
            &notifications[0].update,
            SessionUpdate::AgentMessageChunk(ContentChunk {
                content: ContentBlock::Text(TextContent { text, .. }),
                ..
            }) if text == "Undo in progress..."
        ));
        assert!(matches!(
            &notifications[1].update,
            SessionUpdate::AgentMessageChunk(ContentChunk {
                content: ContentBlock::Text(TextContent { text, .. }),
                ..
            }) if text == "Undo completed."
        ));

        let ops = thread.ops.lock().unwrap();
        assert_eq!(ops.as_slice(), &[Op::Undo]);

        Ok(())
    }

    #[tokio::test]
    async fn test_mcp() -> anyhow::Result<()> {
        let (session_id, client, thread, message_tx, local_set) = setup(vec![]).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(session_id.clone(), vec!["/mcp".into()]),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(
            matches!(
                &notifications[0].update,
                SessionUpdate::AgentMessageChunk(ContentChunk {
                    content: ContentBlock::Text(TextContent { text, .. }),
                    ..
                }) if text.contains("MCP") // "No MCP tools..." or listing
            ),
            "notifications don't match {notifications:?}"
        );

        let ops = thread.ops.lock().unwrap();
        assert_eq!(ops.as_slice(), &[Op::ListMcpTools]);

        Ok(())
    }

    #[tokio::test]
    async fn test_skills() -> anyhow::Result<()> {
        let (session_id, client, thread, message_tx, local_set) = setup(vec![]).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(session_id.clone(), vec!["/skills".into()]),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(
            matches!(
                &notifications[0].update,
                SessionUpdate::AgentMessageChunk(ContentChunk {
                    content: ContentBlock::Text(TextContent { text, .. }),
                    ..
                }) if text.contains("demo-skill")
            ),
            "notifications don't match {notifications:?}"
        );

        let ops = thread.ops.lock().unwrap();
        assert_eq!(
            ops.as_slice(),
            &[Op::ListSkills {
                cwds: vec![],
                force_reload: false,
            }]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_init() -> anyhow::Result<()> {
        let (session_id, client, thread, message_tx, local_set) = setup(vec![]).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(session_id.clone(), vec!["/init".into()]),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(
            matches!(
                &notifications[0].update,
                SessionUpdate::AgentMessageChunk(ContentChunk {
                    content: ContentBlock::Text(TextContent { text, .. }), ..
                }) if text == INIT_COMMAND_PROMPT // we echo the prompt
            ),
            "notifications don't match {notifications:?}"
        );
        let ops = thread.ops.lock().unwrap();
        assert_eq!(
            ops.as_slice(),
            &[Op::UserInput {
                items: vec![UserInput::Text {
                    text: INIT_COMMAND_PROMPT.to_string(),
                    text_elements: vec![]
                }],
                final_output_json_schema: None,
            }],
            "ops don't match {ops:?}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_review() -> anyhow::Result<()> {
        let (session_id, client, thread, message_tx, local_set) = setup(vec![]).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(session_id.clone(), vec!["/review".into()]),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(
            matches!(
                &notifications[0].update,
                SessionUpdate::AgentMessageChunk(ContentChunk {
                    content: ContentBlock::Text(TextContent { text, .. }),
                    ..
                }) if text == "current changes" // we echo the prompt
            ),
            "notifications don't match {notifications:?}"
        );

        let ops = thread.ops.lock().unwrap();
        assert_eq!(
            ops.as_slice(),
            &[Op::Review {
                review_request: ReviewRequest {
                    user_facing_hint: Some(user_facing_hint(&ReviewTarget::UncommittedChanges)),
                    target: ReviewTarget::UncommittedChanges,
                }
            }],
            "ops don't match {ops:?}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_custom_review() -> anyhow::Result<()> {
        let (session_id, client, thread, message_tx, local_set) = setup(vec![]).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();
        let instructions = "Review what we did in agents.md";

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(
                session_id.clone(),
                vec![format!("/review {instructions}").into()],
            ),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(
            matches!(
                &notifications[0].update,
                SessionUpdate::AgentMessageChunk(ContentChunk {
                    content: ContentBlock::Text(TextContent { text, .. }),
                    ..
                }) if text == "Review what we did in agents.md" // we echo the prompt
            ),
            "notifications don't match {notifications:?}"
        );

        let ops = thread.ops.lock().unwrap();
        assert_eq!(
            ops.as_slice(),
            &[Op::Review {
                review_request: ReviewRequest {
                    user_facing_hint: Some(user_facing_hint(&ReviewTarget::Custom {
                        instructions: instructions.to_owned()
                    })),
                    target: ReviewTarget::Custom {
                        instructions: instructions.to_owned()
                    },
                }
            }],
            "ops don't match {ops:?}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_commit_review() -> anyhow::Result<()> {
        let (session_id, client, thread, message_tx, local_set) = setup(vec![]).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(session_id.clone(), vec!["/review-commit 123456".into()]),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(
            matches!(
                &notifications[0].update,
                SessionUpdate::AgentMessageChunk(ContentChunk {
                    content: ContentBlock::Text(TextContent { text, .. }),
                    ..
                }) if text == "commit 123456" // we echo the prompt
            ),
            "notifications don't match {notifications:?}"
        );

        let ops = thread.ops.lock().unwrap();
        assert_eq!(
            ops.as_slice(),
            &[Op::Review {
                review_request: ReviewRequest {
                    user_facing_hint: Some(user_facing_hint(&ReviewTarget::Commit {
                        sha: "123456".to_owned(),
                        title: None
                    })),
                    target: ReviewTarget::Commit {
                        sha: "123456".to_owned(),
                        title: None
                    },
                }
            }],
            "ops don't match {ops:?}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_branch_review() -> anyhow::Result<()> {
        let (session_id, client, thread, message_tx, local_set) = setup(vec![]).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(session_id.clone(), vec!["/review-branch feature".into()]),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(
            matches!(
                &notifications[0].update,
                SessionUpdate::AgentMessageChunk(ContentChunk {
                    content: ContentBlock::Text(TextContent { text, .. }),
                    ..
                }) if text == "changes against 'feature'" // we echo the prompt
            ),
            "notifications don't match {notifications:?}"
        );

        let ops = thread.ops.lock().unwrap();
        assert_eq!(
            ops.as_slice(),
            &[Op::Review {
                review_request: ReviewRequest {
                    user_facing_hint: Some(user_facing_hint(&ReviewTarget::BaseBranch {
                        branch: "feature".to_owned()
                    })),
                    target: ReviewTarget::BaseBranch {
                        branch: "feature".to_owned()
                    },
                }
            }],
            "ops don't match {ops:?}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_custom_prompts() -> anyhow::Result<()> {
        let custom_prompts = vec![CustomPrompt {
            name: "custom".to_string(),
            path: "/tmp/custom.md".into(),
            content: "Custom prompt with $1 arg.".into(),
            description: None,
            argument_hint: None,
        }];
        let (session_id, client, thread, message_tx, local_set) = setup(custom_prompts).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(session_id.clone(), vec!["/custom foo".into()]),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        let notifications = client.notifications.lock().unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(
            matches!(
                &notifications[0].update,
                SessionUpdate::AgentMessageChunk(ContentChunk {
                    content: ContentBlock::Text(TextContent { text, .. }),
                    ..
                }) if text == "Custom prompt with foo arg."
            ),
            "notifications don't match {notifications:?}"
        );

        let ops = thread.ops.lock().unwrap();
        assert_eq!(
            ops.as_slice(),
            &[Op::UserInput {
                items: vec![UserInput::Text {
                    text: "Custom prompt with foo arg.".into(),
                    text_elements: vec![]
                }],
                final_output_json_schema: None,
            }],
            "ops don't match {ops:?}"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_delta_deduplication() -> anyhow::Result<()> {
        let (session_id, client, _, message_tx, local_set) = setup(vec![]).await?;
        let (prompt_response_tx, prompt_response_rx) = tokio::sync::oneshot::channel();

        message_tx.send(ThreadMessage::Prompt {
            request: PromptRequest::new(session_id.clone(), vec!["test delta".into()]),
            response_tx: prompt_response_tx,
        })?;

        tokio::try_join!(
            async {
                let stop_reason = prompt_response_rx.await??.await??;
                assert_eq!(stop_reason, StopReason::EndTurn);
                drop(message_tx);
                anyhow::Ok(())
            },
            async {
                local_set.await;
                anyhow::Ok(())
            }
        )?;

        // We should only get ONE notification, not duplicates from both delta and non-delta
        let notifications = client.notifications.lock().unwrap();
        assert_eq!(
            notifications.len(),
            1,
            "Should only receive delta event, not duplicate non-delta. Got: {notifications:?}"
        );
        assert!(matches!(
            &notifications[0].update,
            SessionUpdate::AgentMessageChunk(ContentChunk {
                content: ContentBlock::Text(TextContent { text, .. }),
                ..
            }) if text == "test delta"
        ));

        Ok(())
    }

    async fn setup(
        custom_prompts: Vec<CustomPrompt>,
    ) -> anyhow::Result<(
        SessionId,
        Arc<StubClient>,
        Arc<StubCodexThread>,
        UnboundedSender<ThreadMessage>,
        LocalSet,
    )> {
        let session_id = SessionId::new("test");
        let client = Arc::new(StubClient::new());
        let session_client =
            SessionClient::with_client(session_id.clone(), client.clone(), Arc::default(), None);
        let conversation = Arc::new(StubCodexThread::new());
        let models_manager = Arc::new(StubModelsManager);
        let config = Config::load_with_cli_overrides_and_harness_overrides(
            vec![],
            ConfigOverrides::default(),
        )
        .await?;
        let (message_tx, message_rx) = tokio::sync::mpsc::unbounded_channel();

        let mut actor = ThreadActor::new(
            StubAuth,
            session_client,
            conversation.clone(),
            models_manager,
            config,
            message_rx,
        );
        actor.custom_prompts = Rc::new(RefCell::new(custom_prompts));

        let local_set = LocalSet::new();
        local_set.spawn_local(actor.spawn());
        Ok((session_id, client, conversation, message_tx, local_set))
    }

    struct StubAuth;

    impl Auth for StubAuth {
        fn logout(&self) -> Result<bool, Error> {
            Ok(true)
        }
    }

    struct StubModelsManager;

    #[async_trait::async_trait]
    impl ModelsManagerImpl for StubModelsManager {
        async fn get_model(&self, _model_id: &Option<String>, _config: &Config) -> String {
            all_model_presets()[0].to_owned().id
        }

        async fn list_models(&self, _config: &Config) -> Vec<ModelPreset> {
            all_model_presets().to_owned()
        }
    }

    struct StubCodexThread {
        current_id: AtomicUsize,
        ops: std::sync::Mutex<Vec<Op>>,
        op_tx: mpsc::UnboundedSender<Event>,
        op_rx: Mutex<mpsc::UnboundedReceiver<Event>>,
    }

    impl StubCodexThread {
        fn new() -> Self {
            let (op_tx, op_rx) = mpsc::unbounded_channel();
            StubCodexThread {
                current_id: AtomicUsize::new(0),
                ops: std::sync::Mutex::default(),
                op_tx,
                op_rx: Mutex::new(op_rx),
            }
        }
    }

    #[async_trait::async_trait]
    impl CodexThreadImpl for StubCodexThread {
        async fn submit(&self, op: Op) -> Result<String, CodexErr> {
            let id = self
                .current_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            self.ops.lock().unwrap().push(op.clone());

            match op {
                Op::ListMcpTools => {
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::McpListToolsResponse(
                                codex_core::protocol::McpListToolsResponseEvent {
                                    tools: std::collections::HashMap::new(),
                                    resources: std::collections::HashMap::new(),
                                    resource_templates: std::collections::HashMap::new(),
                                    auth_statuses: std::collections::HashMap::new(),
                                },
                            ),
                        })
                        .unwrap();
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::TurnComplete(TurnCompleteEvent {
                                last_agent_message: None,
                            }),
                        })
                        .unwrap();
                }
                Op::ListSkills { .. } => {
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::ListSkillsResponse(
                                codex_core::protocol::ListSkillsResponseEvent {
                                    skills: vec![codex_core::protocol::SkillsListEntry {
                                        cwd: PathBuf::from("/tmp/repo"),
                                        skills: vec![codex_core::protocol::SkillMetadata {
                                            name: "demo-skill".to_string(),
                                            description: "Demo skill".to_string(),
                                            short_description: None,
                                            interface: None,
                                            path: PathBuf::from("/tmp/repo/SKILL.md"),
                                            scope: codex_core::protocol::SkillScope::Repo,
                                            enabled: true,
                                        }],
                                        errors: vec![],
                                    }],
                                },
                            ),
                        })
                        .unwrap();
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::TurnComplete(TurnCompleteEvent {
                                last_agent_message: None,
                            }),
                        })
                        .unwrap();
                }
                Op::UserInput { items, .. } => {
                    let prompt = items
                        .into_iter()
                        .map(|i| match i {
                            UserInput::Text { text, .. } => text,
                            _ => unimplemented!(),
                        })
                        .join("\n");

                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::AgentMessageContentDelta(
                                AgentMessageContentDeltaEvent {
                                    thread_id: id.to_string(),
                                    turn_id: id.to_string(),
                                    item_id: id.to_string(),
                                    delta: prompt.clone(),
                                },
                            ),
                        })
                        .unwrap();
                    // Send non-delta event (should be deduplicated, but handled by deduplication)
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::AgentMessage(AgentMessageEvent { message: prompt }),
                        })
                        .unwrap();
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::TurnComplete(TurnCompleteEvent {
                                last_agent_message: None,
                            }),
                        })
                        .unwrap();
                }
                Op::Compact => {
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::TurnStarted(TurnStartedEvent {
                                model_context_window: None,
                            }),
                        })
                        .unwrap();
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::AgentMessage(AgentMessageEvent {
                                message: "Compact task completed".to_string(),
                            }),
                        })
                        .unwrap();
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::TurnComplete(TurnCompleteEvent {
                                last_agent_message: None,
                            }),
                        })
                        .unwrap();
                }
                Op::Undo => {
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::UndoStarted(codex_core::protocol::UndoStartedEvent {
                                message: Some("Undo in progress...".to_string()),
                            }),
                        })
                        .unwrap();
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::UndoCompleted(
                                codex_core::protocol::UndoCompletedEvent {
                                    success: true,
                                    message: Some("Undo completed.".to_string()),
                                },
                            ),
                        })
                        .unwrap();
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::TurnComplete(TurnCompleteEvent {
                                last_agent_message: None,
                            }),
                        })
                        .unwrap();
                }
                Op::Review { review_request } => {
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::EnteredReviewMode(review_request.clone()),
                        })
                        .unwrap();
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::ExitedReviewMode(ExitedReviewModeEvent {
                                review_output: Some(ReviewOutputEvent {
                                    findings: vec![],
                                    overall_correctness: String::new(),
                                    overall_explanation: review_request
                                        .user_facing_hint
                                        .clone()
                                        .unwrap_or_default(),
                                    overall_confidence_score: 1.,
                                }),
                            }),
                        })
                        .unwrap();
                    self.op_tx
                        .send(Event {
                            id: id.to_string(),
                            msg: EventMsg::TurnComplete(TurnCompleteEvent {
                                last_agent_message: None,
                            }),
                        })
                        .unwrap();
                }
                _ => {
                    unimplemented!()
                }
            }
            Ok(id.to_string())
        }

        async fn next_event(&self) -> Result<Event, CodexErr> {
            let Some(event) = self.op_rx.lock().await.recv().await else {
                return Err(CodexErr::InternalAgentDied);
            };
            Ok(event)
        }
    }

    struct StubClient {
        notifications: std::sync::Mutex<Vec<SessionNotification>>,
    }

    impl StubClient {
        fn new() -> Self {
            StubClient {
                notifications: std::sync::Mutex::default(),
            }
        }
    }

    #[async_trait::async_trait(?Send)]
    impl Client for StubClient {
        async fn request_permission(
            &self,
            _args: RequestPermissionRequest,
        ) -> Result<RequestPermissionResponse, Error> {
            unimplemented!()
        }

        async fn session_notification(&self, args: SessionNotification) -> Result<(), Error> {
            self.notifications.lock().unwrap().push(args);
            Ok(())
        }
    }
}
