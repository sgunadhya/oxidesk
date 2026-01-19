use crate::infrastructure::http::middleware::error::ApiResult;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct Template {
    pub name: String,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
}

#[async_trait]
pub trait TemplateRepository: Send + Sync {
    async fn get_template(&self, name: &str) -> ApiResult<Option<Template>>;
    async fn save_template(&self, template: &Template) -> ApiResult<()>;
}
