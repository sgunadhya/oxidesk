use crate::domain::ports::template_repository::{Template, TemplateRepository};
use crate::infrastructure::http::middleware::error::{ApiError, ApiResult};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

#[derive(Clone)]
pub struct LocalTemplateRepository {
    base_path: PathBuf,
}

impl LocalTemplateRepository {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn resolve_path(&self, name: &str) -> PathBuf {
        self.base_path.join(name)
    }
}

#[async_trait]
impl TemplateRepository for LocalTemplateRepository {
    async fn get_template(&self, name: &str) -> ApiResult<Option<Template>> {
        let path = self.resolve_path(name);

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to read template: {}", e)))?;

        // Simple assumption: Accessing file by name returns it.
        // Logic to split subject/body or manage html/text versions typically requires a convention.
        // For this abstraction, let's assume the 'name' is the filename for the HTML body,
        // and we might look for .txt or .subject files if needed, or parse frontmatter.
        // Given 'agent_reply_email.html' usage, let's keep it simple:
        // returned Template struct allows structured data, but file is just raw HTML.
        // We will return the content in body_html.

        Ok(Some(Template {
            name: name.to_string(),
            subject: "".to_string(), // Subject might be dynamic or stored elsewhere
            body_html: content,
            body_text: "".to_string(),
        }))
    }

    async fn save_template(&self, template: &Template) -> ApiResult<()> {
        let path = self.resolve_path(&template.name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                ApiError::Internal(format!("Failed to create template directory: {}", e))
            })?;
        }

        fs::write(&path, &template.body_html)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to write template: {}", e)))?;
        Ok(())
    }
}
