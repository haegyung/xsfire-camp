use agent_client_protocol::{
    Agent, AgentCapabilities, AuthenticateRequest, AuthenticateResponse, CancelNotification,
    ClientCapabilities, Error, Implementation, InitializeRequest, InitializeResponse,
    ListSessionsRequest, ListSessionsResponse, LoadSessionRequest, LoadSessionResponse,
    McpCapabilities, NewSessionRequest, NewSessionResponse, PromptCapabilities, PromptRequest,
    PromptResponse, ProtocolVersion, SessionCapabilities, SessionListCapabilities,
    SetSessionConfigOptionRequest, SetSessionConfigOptionResponse, SetSessionModeRequest,
    SetSessionModeResponse, SetSessionModelRequest, SetSessionModelResponse,
};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tracing::debug;

use crate::backend::BackendDriver;

pub struct AcpAgent {
    driver: Rc<dyn BackendDriver>,
    client_capabilities: Arc<Mutex<ClientCapabilities>>,
}

impl AcpAgent {
    pub fn new(
        driver: Rc<dyn BackendDriver>,
        client_capabilities: Arc<Mutex<ClientCapabilities>>,
    ) -> Self {
        Self {
            driver,
            client_capabilities,
        }
    }
}

#[async_trait::async_trait(?Send)]
impl Agent for AcpAgent {
    async fn initialize(&self, request: InitializeRequest) -> Result<InitializeResponse, Error> {
        let InitializeRequest {
            protocol_version,
            client_capabilities,
            client_info: _, // TODO: save and pass into backend somehow
            ..
        } = request;
        debug!("Received initialize request with protocol version {protocol_version:?}",);
        let protocol_version = ProtocolVersion::V1;

        *self.client_capabilities.lock().unwrap() = client_capabilities;

        let load_session = self.driver.supports_load_session();
        let mut agent_capabilities = AgentCapabilities::new()
            .prompt_capabilities(PromptCapabilities::new().embedded_context(true).image(true))
            .mcp_capabilities(McpCapabilities::new().http(true))
            .load_session(load_session);

        agent_capabilities.session_capabilities =
            SessionCapabilities::new().list(SessionListCapabilities::new());

        Ok(InitializeResponse::new(protocol_version)
            .agent_capabilities(agent_capabilities)
            .agent_info(
                Implementation::new("xsfire-camp", env!("CARGO_PKG_VERSION")).title("xsfire-camp"),
            )
            .auth_methods(self.driver.auth_methods()))
    }

    async fn authenticate(
        &self,
        request: AuthenticateRequest,
    ) -> Result<AuthenticateResponse, Error> {
        self.driver.authenticate(request).await
    }

    async fn new_session(&self, request: NewSessionRequest) -> Result<NewSessionResponse, Error> {
        self.driver.new_session(request).await
    }

    async fn load_session(
        &self,
        request: LoadSessionRequest,
    ) -> Result<LoadSessionResponse, Error> {
        self.driver.load_session(request).await
    }

    async fn list_sessions(
        &self,
        request: ListSessionsRequest,
    ) -> Result<ListSessionsResponse, Error> {
        self.driver.list_sessions(request).await
    }

    async fn prompt(&self, request: PromptRequest) -> Result<PromptResponse, Error> {
        self.driver.prompt(request).await
    }

    async fn cancel(&self, args: CancelNotification) -> Result<(), Error> {
        self.driver.cancel(args).await
    }

    async fn set_session_mode(
        &self,
        args: SetSessionModeRequest,
    ) -> Result<SetSessionModeResponse, Error> {
        self.driver.set_session_mode(args).await
    }

    async fn set_session_model(
        &self,
        args: SetSessionModelRequest,
    ) -> Result<SetSessionModelResponse, Error> {
        self.driver.set_session_model(args).await
    }

    async fn set_session_config_option(
        &self,
        args: SetSessionConfigOptionRequest,
    ) -> Result<SetSessionConfigOptionResponse, Error> {
        self.driver.set_session_config_option(args).await
    }
}
