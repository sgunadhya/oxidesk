use crate::infrastructure::http::middleware::error::ApiResult;
use crate::domain::entities::{User, UserType};
use async_trait::async_trait;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create_user(&self, user: &User) -> ApiResult<()>;
    async fn get_user_by_email_and_type(
        &self,
        email: &str,
        user_type: &UserType,
    ) -> ApiResult<Option<User>>;
    async fn get_user_by_id(&self, id: &str) -> ApiResult<Option<User>>;
    async fn update_user_email(&self, id: &str, email: &str, updated_at: &str) -> ApiResult<()>;
    async fn soft_delete_user(&self, user_id: &str, deleted_by: &str) -> ApiResult<()>;
    async fn restore_user(&self, user_id: &str) -> ApiResult<()>;
    async fn list_users(
        &self,
        limit: i64,
        offset: i64,
        user_type_filter: Option<UserType>,
    ) -> ApiResult<(Vec<User>, i64)>;
    async fn get_users_by_usernames(&self, usernames: &[String]) -> ApiResult<Vec<User>>;
    async fn get_users_by_ids(&self, ids: &[String]) -> ApiResult<Vec<User>>;
    async fn count_admin_users(&self) -> ApiResult<i64>;
    async fn delete_user(&self, user_id: &str) -> ApiResult<()>;
}
