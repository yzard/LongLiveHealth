use super::{Database, queries};
use crate::{
    error::AppError,
    models::{ListRequest, User},
};

impl Database {
    pub async fn initialize_admin(
        &self,
        hash: &str,
        reset: bool,
        now: i64,
    ) -> Result<(), AppError> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query(queries::INITIALIZE_ADMIN)
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(hash)
            .bind(now)
            .execute(&mut *transaction)
            .await?;
        if reset {
            let id: String = sqlx::query_scalar(queries::RESET_ADMIN)
                .bind(hash)
                .fetch_one(&mut *transaction)
                .await?;
            sqlx::query(queries::REVOKE_USER_SESSIONS)
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            sqlx::query(queries::CLEAR_ADMIN_LOGIN_WINDOW)
                .bind(crate::authentication::digest(b"admin"))
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn username_for_id(&self, user_id: &str) -> Result<String, AppError> {
        sqlx::query_scalar(queries::USERNAME_BY_ID)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn users(&self, request: &ListRequest) -> Result<Vec<User>, AppError> {
        request.validate()?;
        Ok(sqlx::query_as(queries::LIST_USERS)
            .bind(&request.after_id)
            .bind(&request.after_id)
            .bind(request.limit)
            .fetch_all(&self.pool)
            .await?)
    }

    pub async fn change_own_password(
        &self,
        user_id: &str,
        version: i64,
        token_hash: &str,
        hash: &str,
        now: i64,
    ) -> Result<(), AppError> {
        let mut transaction = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query_scalar::<_, String>(queries::CHANGE_OWN_PASSWORD)
            .bind(hash)
            .bind(user_id)
            .bind(version)
            .bind(token_hash)
            .bind(now)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(AppError::Unauthorized)?;
        sqlx::query(queries::REVOKE_USER_SESSIONS)
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}
