use crate::api::middleware::error::ApiResult;
use crate::models::{Contact, ContactChannel, User};
use async_trait::async_trait;

#[async_trait]
pub trait ContactRepository: Send + Sync {
    async fn create_contact(&self, contact: &Contact) -> ApiResult<()>;
    async fn find_contact_by_user_id(&self, user_id: &str) -> ApiResult<Option<Contact>>;
    async fn create_contact_channel(&self, channel: &ContactChannel) -> ApiResult<()>;
    async fn get_contact_by_email(&self, email: &str) -> ApiResult<Option<Contact>>;
    async fn create_contact_from_message(
        &self,
        email: &str,
        full_name: Option<&str>,
        inbox_id: &str,
    ) -> ApiResult<String>;
    async fn update_contact(&self, contact_id: &str, first_name: Option<String>) -> ApiResult<()>;
    async fn find_contact_channels(&self, contact_id: &str) -> ApiResult<Vec<ContactChannel>>;
    async fn delete_contact(&self, contact_id: &str) -> ApiResult<()>;
    async fn list_contacts(&self, limit: i64, offset: i64) -> ApiResult<Vec<(User, Contact)>>;
    async fn count_contacts(&self) -> ApiResult<i64>;
}
