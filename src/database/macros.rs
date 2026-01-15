use crate::api::middleware::error::ApiResult;
use crate::database::Database;
use crate::models::{Macro, MacroAccess, MacroAction, MacroApplicationLog};
use sqlx::Row;

impl Database {
    // ===== Macro Operations =====

    pub async fn create_macro(&self, macro_obj: &Macro) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO macros (id, name, message_content, created_by, created_at, updated_at, usage_count, access_control)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&macro_obj.id)
        .bind(&macro_obj.name)
        .bind(&macro_obj.message_content)
        .bind(&macro_obj.created_by)
        .bind(&macro_obj.created_at)
        .bind(&macro_obj.updated_at)
        .bind(macro_obj.usage_count)
        .bind(&macro_obj.access_control)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_macro_by_id(&self, id: &str) -> ApiResult<Option<Macro>> {
        let row = sqlx::query(
            "SELECT id, name, message_content, created_by, created_at, updated_at, usage_count, access_control
             FROM macros
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Macro {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                message_content: row.try_get("message_content")?,
                created_by: row.try_get("created_by")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                usage_count: row.try_get("usage_count")?,
                access_control: row.try_get("access_control")?,
                actions: None, // Actions loaded separately
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_macro_by_name(&self, name: &str) -> ApiResult<Option<Macro>> {
        let row = sqlx::query(
            "SELECT id, name, message_content, created_by, created_at, updated_at, usage_count, access_control
             FROM macros
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(Macro {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                message_content: row.try_get("message_content")?,
                created_by: row.try_get("created_by")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                usage_count: row.try_get("usage_count")?,
                access_control: row.try_get("access_control")?,
                actions: None, // Actions loaded separately
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_macros(&self) -> ApiResult<Vec<Macro>> {
        let rows = sqlx::query(
            "SELECT id, name, message_content, created_by, created_at, updated_at, usage_count, access_control
             FROM macros
             ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut macros = Vec::new();
        for row in rows {
            macros.push(Macro {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                message_content: row.try_get("message_content")?,
                created_by: row.try_get("created_by")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                usage_count: row.try_get("usage_count")?,
                access_control: row.try_get("access_control")?,
                actions: None, // Actions loaded separately
            });
        }

        Ok(macros)
    }

    pub async fn update_macro(&self, macro_obj: &Macro) -> ApiResult<()> {
        sqlx::query(
            "UPDATE macros
             SET name = ?, message_content = ?, updated_at = ?, access_control = ?
             WHERE id = ?",
        )
        .bind(&macro_obj.name)
        .bind(&macro_obj.message_content)
        .bind(&macro_obj.updated_at)
        .bind(&macro_obj.access_control)
        .bind(&macro_obj.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_macro(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM macros WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn increment_macro_usage(&self, id: &str) -> ApiResult<()> {
        sqlx::query("UPDATE macros SET usage_count = usage_count + 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ===== Macro Action Operations =====

    pub async fn create_macro_action(&self, action: &MacroAction) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO macro_actions (id, macro_id, action_type, action_value, action_order)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&action.id)
        .bind(&action.macro_id)
        .bind(&action.action_type)
        .bind(&action.action_value)
        .bind(action.action_order)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_macro_actions(&self, macro_id: &str) -> ApiResult<Vec<MacroAction>> {
        let rows = sqlx::query(
            "SELECT id, macro_id, action_type, action_value, action_order
             FROM macro_actions
             WHERE macro_id = ?
             ORDER BY action_order ASC",
        )
        .bind(macro_id)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            actions.push(MacroAction {
                id: row.try_get("id")?,
                macro_id: row.try_get("macro_id")?,
                action_type: row.try_get("action_type")?,
                action_value: row.try_get("action_value")?,
                action_order: row.try_get("action_order")?,
            });
        }

        Ok(actions)
    }

    pub async fn delete_macro_actions(&self, macro_id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM macro_actions WHERE macro_id = ?")
            .bind(macro_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ===== Macro Access Operations =====

    pub async fn create_macro_access(&self, access: &MacroAccess) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO macro_access (id, macro_id, entity_type, entity_id, granted_at, granted_by)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&access.id)
        .bind(&access.macro_id)
        .bind(&access.entity_type)
        .bind(&access.entity_id)
        .bind(&access.granted_at)
        .bind(&access.granted_by)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_macro_access(&self, macro_id: &str) -> ApiResult<Vec<MacroAccess>> {
        let rows = sqlx::query(
            "SELECT id, macro_id, entity_type, entity_id, granted_at, granted_by
             FROM macro_access
             WHERE macro_id = ?",
        )
        .bind(macro_id)
        .fetch_all(&self.pool)
        .await?;

        let mut accesses = Vec::new();
        for row in rows {
            accesses.push(MacroAccess {
                id: row.try_get("id")?,
                macro_id: row.try_get("macro_id")?,
                entity_type: row.try_get("entity_type")?,
                entity_id: row.try_get("entity_id")?,
                granted_at: row.try_get("granted_at")?,
                granted_by: row.try_get("granted_by")?,
            });
        }

        Ok(accesses)
    }

    pub async fn delete_macro_access(
        &self,
        macro_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> ApiResult<()> {
        sqlx::query(
            "DELETE FROM macro_access
             WHERE macro_id = ? AND entity_type = ? AND entity_id = ?",
        )
        .bind(macro_id)
        .bind(entity_type)
        .bind(entity_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn user_has_macro_access(&self, macro_id: &str, user_id: &str) -> ApiResult<bool> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM macro_access
             WHERE macro_id = ? AND entity_type = 'user' AND entity_id = ?",
        )
        .bind(macro_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i32 = row.try_get("count")?;
        Ok(count > 0)
    }

    pub async fn team_has_macro_access(&self, macro_id: &str, team_id: &str) -> ApiResult<bool> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count
             FROM macro_access
             WHERE macro_id = ? AND entity_type = 'team' AND entity_id = ?",
        )
        .bind(macro_id)
        .bind(team_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i32 = row.try_get("count")?;
        Ok(count > 0)
    }

    // ===== Macro Application Log Operations =====

    pub async fn create_macro_application_log(&self, log: &MacroApplicationLog) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO macro_application_logs (id, macro_id, agent_id, conversation_id, applied_at, actions_queued, variables_replaced)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&log.id)
        .bind(&log.macro_id)
        .bind(&log.agent_id)
        .bind(&log.conversation_id)
        .bind(&log.applied_at)
        .bind(&log.actions_queued)
        .bind(log.variables_replaced)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_macro_application_logs(
        &self,
        macro_id: &str,
        limit: i32,
        offset: i32,
    ) -> ApiResult<Vec<MacroApplicationLog>> {
        let rows = sqlx::query(
            "SELECT id, macro_id, agent_id, conversation_id, applied_at, actions_queued, variables_replaced
             FROM macro_application_logs
             WHERE macro_id = ?
             ORDER BY applied_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(macro_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(MacroApplicationLog {
                id: row.try_get("id")?,
                macro_id: row.try_get("macro_id")?,
                agent_id: row.try_get("agent_id")?,
                conversation_id: row.try_get("conversation_id")?,
                applied_at: row.try_get("applied_at")?,
                actions_queued: row.try_get("actions_queued")?,
                variables_replaced: row.try_get("variables_replaced")?,
            });
        }

        Ok(logs)
    }
}
