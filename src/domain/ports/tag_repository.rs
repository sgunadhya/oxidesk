use crate::{
    infrastructure::http::middleware::error::ApiResult,
    infrastructure::persistence::Database,
    domain::entities::{Permission, Tag},
};

#[derive(Clone)]
pub struct TagRepository {
    db: Database,
}

impl TagRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Get tag by name
    pub async fn get_tag_by_name(&self, name: &str) -> ApiResult<Option<Tag>> {
        self.db.get_tag_by_name(name).await
    }

    /// Create a new tag
    pub async fn create_tag(&self, tag: &Tag) -> ApiResult<()> {
        self.db.create_tag(tag).await
    }

    /// List all tags with pagination
    pub async fn list_tags(&self, limit: i64, offset: i64) -> ApiResult<(Vec<Tag>, i64)> {
        self.db.list_tags(limit, offset).await
    }

    /// Get tag by ID
    pub async fn get_tag_by_id(&self, tag_id: &str) -> ApiResult<Option<Tag>> {
        self.db.get_tag_by_id(tag_id).await
    }

    /// Update tag properties
    pub async fn update_tag(
        &self,
        tag_id: &str,
        description: Option<String>,
        color: Option<String>,
    ) -> ApiResult<()> {
        self.db.update_tag(tag_id, description, color).await
    }

    /// Delete tag
    pub async fn delete_tag(&self, tag_id: &str) -> ApiResult<()> {
        self.db.delete_tag(tag_id).await
    }

    /// Get user permissions
    pub async fn get_user_permissions(&self, user_id: &str) -> ApiResult<Vec<Permission>> {
        self.db.get_user_permissions(user_id).await
    }
}
