use agent_client_protocol::schema::{
    AudioContent, ContentBlock, EmbeddedResourceResource, ImageContent, TextContent,
};

#[derive(Debug)]
pub(super) struct AcpOutputBuffer {
    text: String,
    limit_bytes: usize,
}

impl AcpOutputBuffer {
    pub(super) fn new(limit_bytes: usize) -> Self {
        Self {
            text: String::new(),
            limit_bytes,
        }
    }

    pub(super) fn push_str(&mut self, text: &str) -> Result<(), AcpOutputLimitExceeded> {
        if self.text.len().saturating_add(text.len()) > self.limit_bytes {
            return Err(AcpOutputLimitExceeded {
                limit_bytes: self.limit_bytes,
            });
        }
        self.text.push_str(text);
        Ok(())
    }

    pub(super) fn as_str(&self) -> &str {
        &self.text
    }

    pub(super) fn into_string(self) -> String {
        self.text
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AcpOutputLimitExceeded {
    pub(super) limit_bytes: usize,
}

pub(super) fn acp_output_limit_error(
    limit: AcpOutputLimitExceeded,
) -> agent_client_protocol::Error {
    agent_client_protocol::Error::internal_error().data(format!(
        "ACP agent output exceeded the {} byte limit",
        limit.limit_bytes
    ))
}

pub(super) fn content_block_to_string(block: &ContentBlock) -> String {
    match block {
        ContentBlock::Text(TextContent { text, .. }) => text.clone(),
        ContentBlock::Image(ImageContent { mime_type, .. }) => {
            format!("[Image: {mime_type}]")
        }
        ContentBlock::Audio(AudioContent { mime_type, .. }) => {
            format!("[Audio: {mime_type}]")
        }
        ContentBlock::ResourceLink(link) => link.uri.clone(),
        ContentBlock::Resource(resource) => match &resource.resource {
            EmbeddedResourceResource::TextResourceContents(text) => text.uri.clone(),
            EmbeddedResourceResource::BlobResourceContents(blob) => blob.uri.clone(),
            _ => "[Unknown resource type]".to_owned(),
        },
        _ => "[Unknown content type]".to_owned(),
    }
}
