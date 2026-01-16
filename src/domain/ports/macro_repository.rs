use crate::api::middleware::error::ApiResult;
use crate::database::Database;
use crate::domain::ports::user_repository::UserRepository;
use crate::models::{Macro, MacroAccess, MacroAction, MacroApplicationLog};

/// Repository for macro operations
#[derive(Clone)]
pub struct MacroRepository {
    db: Database,
}

impl MacroRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    // ===== Macro Operations =====

    pub async fn create_macro(&self, macro_obj: &Macro) -> ApiResult<()> {
        self.db.create_macro(macro_obj).await
    }

    pub async fn get_macro_by_id(&self, id: &str) -> ApiResult<Option<Macro>> {
        self.db.get_macro_by_id(id).await
    }

    pub async fn get_macro_by_name(&self, name: &str) -> ApiResult<Option<Macro>> {
        self.db.get_macro_by_name(name).await
    }

    pub async fn list_macros(&self) -> ApiResult<Vec<Macro>> {
        self.db.list_macros().await
    }

    pub async fn update_macro(&self, macro_obj: &Macro) -> ApiResult<()> {
        self.db.update_macro(macro_obj).await
    }

    pub async fn delete_macro(&self, id: &str) -> ApiResult<()> {
        self.db.delete_macro(id).await
    }

    pub async fn increment_macro_usage(&self, id: &str) -> ApiResult<()> {
        self.db.increment_macro_usage(id).await
    }

    // ===== Macro Action Operations =====

    pub async fn create_macro_action(&self, action: &MacroAction) -> ApiResult<()> {
        self.db.create_macro_action(action).await
    }

    pub async fn get_macro_actions(&self, macro_id: &str) -> ApiResult<Vec<MacroAction>> {
        self.db.get_macro_actions(macro_id).await
    }

    pub async fn delete_macro_actions(&self, macro_id: &str) -> ApiResult<()> {
        self.db.delete_macro_actions(macro_id).await
    }

    // ===== Macro Access Operations =====

    pub async fn create_macro_access(&self, access: &MacroAccess) -> ApiResult<()> {
        self.db.create_macro_access(access).await
    }

    pub async fn get_macro_access(&self, macro_id: &str) -> ApiResult<Vec<MacroAccess>> {
        self.db.get_macro_access(macro_id).await
    }

    pub async fn delete_macro_access(
        &self,
        macro_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> ApiResult<()> {
        self.db
            .delete_macro_access(macro_id, entity_type, entity_id)
            .await
    }

    pub async fn user_has_macro_access(&self, macro_id: &str, user_id: &str) -> ApiResult<bool> {
        self.db.user_has_macro_access(macro_id, user_id).await
    }

    pub async fn team_has_macro_access(&self, macro_id: &str, team_id: &str) -> ApiResult<bool> {
        self.db.team_has_macro_access(macro_id, team_id).await
    }

    // ===== Macro Application Log Operations =====

    pub async fn create_macro_application_log(&self, log: &MacroApplicationLog) -> ApiResult<()> {
        self.db.create_macro_application_log(log).await
    }

    pub async fn get_macro_application_logs(
        &self,
        macro_id: &str,
        limit: i32,
        offset: i32,
    ) -> ApiResult<Vec<MacroApplicationLog>> {
        self.db
            .get_macro_application_logs(macro_id, limit, offset)
            .await
    }

    // ===== Helper Methods for Context Loading =====

    pub async fn get_conversation_by_id(
        &self,
        conversation_id: &str,
    ) -> ApiResult<Option<crate::models::Conversation>> {
        self.db.get_conversation_by_id(conversation_id).await
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> ApiResult<Option<crate::models::User>> {
        self.db.get_user_by_id(user_id).await
    }

    pub async fn get_team_by_id(&self, team_id: &str) -> ApiResult<Option<crate::models::Team>> {
        self.db.get_team_by_id(team_id).await
    }

    pub async fn get_user_teams(&self, user_id: &str) -> ApiResult<Vec<crate::models::Team>> {
        self.db.get_user_teams(user_id).await
    }
}
