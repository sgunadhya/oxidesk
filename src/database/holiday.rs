use crate::{database::Database, ApiError, ApiResult};

impl Database {
    /// Create a new holiday
    pub async fn create_holiday(&self, holiday: &crate::models::Holiday) -> ApiResult<()> {
        sqlx::query(
            "INSERT INTO holidays (id, name, date, recurring, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&holiday.id)
        .bind(&holiday.name)
        .bind(&holiday.date)
        .bind(holiday.recurring)
        .bind(&holiday.created_at)
        .bind(&holiday.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a holiday by ID
    pub async fn get_holiday(&self, id: &str) -> ApiResult<Option<crate::models::Holiday>> {
        let holiday =
            sqlx::query_as::<_, crate::models::Holiday>("SELECT * FROM holidays WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(holiday)
    }

    /// Get all holidays
    pub async fn list_holidays(&self) -> ApiResult<Vec<crate::models::Holiday>> {
        let holidays =
            sqlx::query_as::<_, crate::models::Holiday>("SELECT * FROM holidays ORDER BY date ASC")
                .fetch_all(&self.pool)
                .await?;

        Ok(holidays)
    }

    /// Check if a specific date is a holiday
    pub async fn is_holiday(&self, date: &str) -> ApiResult<bool> {
        // Check for exact date match
        let exact_match = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM holidays WHERE date = ? AND recurring = 0",
        )
        .bind(date)
        .fetch_one(&self.pool)
        .await?;

        if exact_match > 0 {
            return Ok(true);
        }

        // Check for recurring holidays (same month-day)
        // Extract month-day from date (YYYY-MM-DD -> MM-DD)
        let month_day = &date[5..]; // Skip "YYYY-"

        let recurring_match = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM holidays WHERE substr(date, 6) = ? AND recurring = 1",
        )
        .bind(month_day)
        .fetch_one(&self.pool)
        .await?;

        Ok(recurring_match > 0)
    }

    /// Update a holiday
    pub async fn update_holiday(
        &self,
        id: &str,
        name: Option<&str>,
        date: Option<&str>,
        recurring: Option<bool>,
    ) -> ApiResult<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Get current holiday to preserve unchanged fields
        let current = self
            .get_holiday(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Holiday {} not found", id)))?;

        let updated_name = name.unwrap_or(&current.name);
        let updated_date = date.unwrap_or(&current.date);
        let updated_recurring = recurring.unwrap_or(current.recurring);

        sqlx::query(
            "UPDATE holidays SET name = ?, date = ?, recurring = ?, updated_at = ? WHERE id = ?",
        )
        .bind(updated_name)
        .bind(updated_date)
        .bind(updated_recurring)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a holiday
    pub async fn delete_holiday(&self, id: &str) -> ApiResult<()> {
        sqlx::query("DELETE FROM holidays WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
