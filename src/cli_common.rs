use agent_client_protocol::{
    Client, ContentBlock, ContentChunk, EmbeddedResource, EmbeddedResourceResource, ResourceLink,
    SessionId, SessionNotification, SessionUpdate, TextContent, TextResourceContents,
};
use tracing::error;

use crate::{ACP_CLIENT, resolve_session_alias};

pub fn prompt_blocks_to_text(blocks: &[ContentBlock]) -> String {
    let mut parts: Vec<String> = Vec::new();

    for block in blocks {
        match block {
            ContentBlock::Text(t) => parts.push(t.text.clone()),
            ContentBlock::ResourceLink(ResourceLink { name, uri, .. }) => {
                if !name.is_empty() {
                    parts.push(format!("[@{name}]({uri})"));
                } else {
                    parts.push(uri.clone());
                }
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
                parts.push(format!("[@embedded]({uri})"));
                parts.push(text.clone());
            }
            // Keep placeholders to preserve user intent without crashing downstream CLIs.
            ContentBlock::Image(_) => parts.push("[image omitted]".to_string()),
            ContentBlock::Audio(_) => parts.push("[audio omitted]".to_string()),
            _ => parts.push("[unsupported content omitted]".to_string()),
        }
    }

    while parts.last().is_some_and(|p| p.trim().is_empty()) {
        parts.pop();
    }

    parts.join("\n")
}

pub async fn send_agent_text(session_id: &SessionId, text: impl Into<String>) {
    let Some(client) = ACP_CLIENT.get() else {
        return;
    };
    let routed_session_id = resolve_session_alias(session_id);

    let update = SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
        TextContent::new(text.into()),
    )));

    if let Err(err) = client
        .session_notification(SessionNotification::new(routed_session_id, update))
        .await
    {
        error!("Failed to send agent text: {err:?}");
    }
}
