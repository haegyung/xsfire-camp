//! Codex ACP - An Agent Client Protocol implementation for Codex.
#![deny(clippy::print_stdout, clippy::print_stderr)]

use agent_client_protocol::AgentSideConnection;
use codex_common::CliConfigOverrides;
use codex_core::config::{Config, ConfigOverrides};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::{io::Result as IoResult, rc::Rc};
use tokio::task::LocalSet;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing_subscriber::EnvFilter;

mod acp_agent;
pub mod backend;
mod claude_code_agent;
mod cli_common;
mod codex_agent;
mod gemini_agent;
mod local_spawner;
mod multi_backend;
mod prompt_args;
mod session_store;
mod thread;

pub static ACP_CLIENT: OnceLock<Arc<AgentSideConnection>> = OnceLock::new();
static SESSION_ALIASES: OnceLock<Arc<Mutex<HashMap<String, agent_client_protocol::SessionId>>>> =
    OnceLock::new();

fn session_aliases() -> &'static Arc<Mutex<HashMap<String, agent_client_protocol::SessionId>>> {
    SESSION_ALIASES.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

pub fn register_session_alias(
    child: &agent_client_protocol::SessionId,
    parent: &agent_client_protocol::SessionId,
) {
    session_aliases()
        .lock()
        .unwrap()
        .insert(child.0.to_string(), parent.clone());
}

pub fn resolve_session_alias(
    session_id: &agent_client_protocol::SessionId,
) -> agent_client_protocol::SessionId {
    session_aliases()
        .lock()
        .unwrap()
        .get(session_id.0.as_ref())
        .cloned()
        .unwrap_or_else(|| session_id.clone())
}

/// Run the Codex ACP agent.
///
/// This sets up an ACP agent that communicates over stdio, bridging
/// the ACP protocol with the existing codex-rs infrastructure.
///
/// # Errors
///
/// If unable to parse the config or start the program.
pub async fn run_main(
    codex_linux_sandbox_exe: Option<PathBuf>,
    cli_config_overrides: CliConfigOverrides,
    backend_kind: backend::BackendKind,
) -> IoResult<()> {
    // Install a simple subscriber so `tracing` output is visible.
    // Users can control the log level with `RUST_LOG`.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Parse CLI overrides and load configuration
    let cli_kv_overrides = cli_config_overrides.parse_overrides().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("error parsing -c overrides: {e}"),
        )
    })?;

    let config_overrides = ConfigOverrides {
        codex_linux_sandbox_exe,
        ..ConfigOverrides::default()
    };

    let config =
        Config::load_with_cli_overrides_and_harness_overrides(cli_kv_overrides, config_overrides)
            .await
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("error loading config: {e}"),
                )
            })?;

    let client_capabilities: Arc<Mutex<agent_client_protocol::ClientCapabilities>> = Arc::default();

    let driver: Rc<dyn backend::BackendDriver> = match backend_kind {
        backend::BackendKind::Codex => Rc::new(codex_agent::CodexDriver::new(
            config,
            client_capabilities.clone(),
        )),
        backend::BackendKind::ClaudeCode => Rc::new(claude_code_agent::ClaudeCodeDriver::new()),
        backend::BackendKind::Gemini => Rc::new(gemini_agent::GeminiCliDriver::new()),
        backend::BackendKind::Multi => Rc::new(multi_backend::MultiBackendDriver::new(
            Rc::new(codex_agent::CodexDriver::new(
                config.clone(),
                client_capabilities.clone(),
            )),
            Rc::new(claude_code_agent::ClaudeCodeDriver::new()),
            Rc::new(gemini_agent::GeminiCliDriver::new()),
        )),
    };

    // Create our Agent implementation with notification channel.
    // This keeps the ACP surface stable while allowing backend selection internally.
    let agent = Rc::new(acp_agent::AcpAgent::new(driver, client_capabilities));

    let stdin = tokio::io::stdin().compat();
    let stdout = tokio::io::stdout().compat_write();

    // Run the I/O task to handle the actual communication
    LocalSet::new()
        .run_until(async move {
            // Create the ACP connection
            let (client, io_task) = AgentSideConnection::new(agent.clone(), stdout, stdin, |fut| {
                tokio::task::spawn_local(fut);
            });

            if ACP_CLIENT.set(Arc::new(client)).is_err() {
                return Err(std::io::Error::other("ACP client already set"));
            }

            io_task
                .await
                .map_err(|e| std::io::Error::other(format!("ACP I/O error: {e}")))
        })
        .await?;

    Ok(())
}

// Re-export the MCP server types for compatibility
pub use codex_mcp_server::{
    CodexToolCallParam, CodexToolCallReplyParam, ExecApprovalElicitRequestParams,
    ExecApprovalResponse, PatchApprovalElicitRequestParams, PatchApprovalResponse,
};
