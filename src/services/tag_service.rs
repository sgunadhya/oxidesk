use crate::{
    api::middleware::error::{ApiError, ApiResult},
    domain::ports::tag_repository::TagRepository,
    models::*,
};

/// Service for tag management operations (admin)
#[derive(Clone)]
pub struct TagService {
    tag_repo: TagRepository,
}

impl TagService {
    pub fn new(tag_repo: TagRepository) -> Self {
        Self { tag_repo }
    }

    /// Helper: Check if user has permission
    fn has_permission(&self, permissions: &[Permission], required: &str) -> bool {
        permissions.iter().any(|p| p.name == required)
    }

    /// Create a new tag (requires tags:create permission)
    pub async fn create_tag(
        &self,
        request: CreateTagRequest,
        permissions: &[Permission],
    ) -> ApiResult<Tag> {
        // 1. Check permission
        if !self.has_permission(permissions, "tags:create") {
            return Err(ApiError::Forbidden(
                "Missing permission: tags:create".to_string(),
            ));
        }

        // 2. Validate tag name
        if request.name.trim().is_empty() {
            return Err(ApiError::BadRequest("Tag name cannot be empty".to_string()));
        }

        if request.name.len() > 50 {
            return Err(ApiError::BadRequest(
                "Tag name cannot exceed 50 characters".to_string(),
            ));
        }

        // 3. Check if tag with same name already exists
        if let Some(_) = self.tag_repo.get_tag_by_name(&request.name).await? {
            return Err(ApiError::BadRequest(format!(
                "Tag with name '{}' already exists",
                request.name
            )));
        }

        // 4. Validate color format if provided
        if let Some(ref color) = request.color {
            if !color.starts_with('#') || color.len() != 7 {
                return Err(ApiError::BadRequest(
                    "Color must be in hex format (#RRGGBB)".to_string(),
                ));
            }
        }

        // 5. Create tag
        let tag = Tag::new(request.name, request.description, request.color);

        // 6. Save to database
        self.tag_repo.create_tag(&tag).await?;

        Ok(tag)
    }

    /// List all tags with pagination (requires tags:read permission)
    pub async fn list_tags(
        &self,
        limit: i64,
        offset: i64,
        permissions: &[Permission],
    ) -> ApiResult<(Vec<Tag>, i64)> {
        // 1. Check permission
        if !self.has_permission(permissions, "tags:read") {
            return Err(ApiError::Forbidden(
                "Missing permission: tags:read".to_string(),
            ));
        }

        // 2. Get tags from database
        self.tag_repo.list_tags(limit, offset).await
    }

    /// Get tag by ID (requires tags:read permission)
    pub async fn get_tag(&self, tag_id: &str, permissions: &[Permission]) -> ApiResult<Tag> {
        // 1. Check permission
        if !self.has_permission(permissions, "tags:read") {
            return Err(ApiError::Forbidden(
                "Missing permission: tags:read".to_string(),
            ));
        }

        // 2. Get tag from database
        self.tag_repo
            .get_tag_by_id(tag_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Tag {} not found", tag_id)))
    }

    /// Update tag properties (requires tags:update permission)
    pub async fn update_tag(
        &self,
        tag_id: &str,
        request: UpdateTagRequest,
        permissions: &[Permission],
    ) -> ApiResult<Tag> {
        // 1. Check permission
        if !self.has_permission(permissions, "tags:update") {
            return Err(ApiError::Forbidden(
                "Missing permission: tags:update".to_string(),
            ));
        }

        // 2. Verify tag exists
        let _tag = self
            .tag_repo
            .get_tag_by_id(tag_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Tag {} not found", tag_id)))?;

        // 3. Validate color format if provided
        if let Some(ref color) = request.color {
            if !color.starts_with('#') || color.len() != 7 {
                return Err(ApiError::BadRequest(
                    "Color must be in hex format (#RRGGBB)".to_string(),
                ));
            }
        }

        // 4. Update tag
        self.tag_repo
            .update_tag(tag_id, request.description, request.color)
            .await?;

        // 5. Return updated tag
        self.tag_repo
            .get_tag_by_id(tag_id)
            .await?
            .ok_or_else(|| ApiError::Internal("Tag disappeared after update".to_string()))
    }

    /// Delete tag (requires tags:delete permission)
    pub async fn delete_tag(&self, tag_id: &str, permissions: &[Permission]) -> ApiResult<()> {
        // 1. Check permission
        if !self.has_permission(permissions, "tags:delete") {
            return Err(ApiError::Forbidden(
                "Missing permission: tags:delete".to_string(),
            ));
        }

        // 2. Verify tag exists
        let _tag = self
            .tag_repo
            .get_tag_by_id(tag_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Tag {} not found", tag_id)))?;

        // 3. Delete tag (cascades to conversation_tags)
        self.tag_repo.delete_tag(tag_id).await?;

        Ok(())
    }

    /// Get user permissions (helper for service layer)
    pub async fn get_user_permissions(&self, user_id: &str) -> ApiResult<Vec<Permission>> {
        self.tag_repo.get_user_permissions(user_id).await
    }
}
