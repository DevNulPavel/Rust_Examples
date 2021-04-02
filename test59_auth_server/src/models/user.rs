use std::{borrow::{
        Borrow
    }, ops::Deref, sync::{
        Arc
    }};
use uuid::{
    Uuid
};
use chrono::{
    NaiveDateTime
};
use serde::{
    Serialize,
    Deserialize
};
use validator::{
    Validate
};
use tracing::{
    instrument
};
use sqlx::{
    PgPool
};
use crate::{
    error::{
        AppError
    }
};

/////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct CreateUserConfig {
    pub user_name: String,
    pub email: String,
    pub password_hash: String,
    pub password_salt: String
}

//////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct UpdateUserConfig {
    pub full_name: Option<String>,
    pub bio: Option<String>,
    pub image: Option<String>
}

/////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Serialize, Deserialize)]
pub struct UserData {
    pub id: Uuid,
    pub user_name: String,
    pub email: String,
    pub password_hash: String,
    pub password_salt: String,
    pub full_name: Option<String>,
    pub bio: Option<String>,
    pub user_image: Option<String>,
    pub create_at: NaiveDateTime,
    pub updated_at: NaiveDateTime
}

/////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct User{
    db: Arc<PgPool>,
    data: UserData
}

impl User {
    #[instrument]
    pub async fn create_new(db: Arc<PgPool>, info: CreateUserConfig) -> Result<User, AppError>{
        // TODO: Где валидировать?
        // info.validate()?;

        let data = sqlx::query_as!(UserData,
                r#"   
                    INSERT INTO users(user_name, email, password_hash, password_salt)
                    VALUES ($1, $2, $3, $4) 
                    RETURNING *
                "#, info.user_name, info.email, info.password_hash, info.password_salt)
            .fetch_one(db.as_ref())
            .await
            .map_err(AppError::from)?;
        Ok(User{
            db,
            data
        })
    }

    #[instrument(fields(id = %id.borrow()))]
    pub async fn find_by_uuid<ID: Borrow<Uuid>>(db: Arc<PgPool>, id: ID) -> Result<Option<User>, AppError> {
        let user_opt = sqlx::query_as!(UserData,
                r#"   
                    SELECT *
                    FROM users
                    WHERE id = $1
                "#, id.borrow())
            .fetch_optional(db.as_ref())
            .await
            .map_err(AppError::from)?
            .map(|data| {
                User{
                    data,
                    db
                }
            });
        Ok(user_opt)
    }

    #[instrument]
    pub async fn find_by_user_name(db: Arc<PgPool>, user_name: &str) -> Result<Option<User>, AppError> {
        let user_opt = sqlx::query_as!(UserData,
                r#"   
                    SELECT *
                    FROM users
                    WHERE user_name = $1
                "#, user_name)
            .fetch_optional(db.as_ref())
            .await
            .map_err(AppError::from)?
            .map(|data| {
                User{
                    data,
                    db
                }
            });
        Ok(user_opt)
    }

    #[instrument]
    pub async fn update_profile_info(&mut self, info: UpdateUserConfig) -> Result<(), AppError>{
        let new_data = sqlx::query_as!(UserData,
                r#"   
                    UPDATE users
                    SET full_name = $1, bio = $2, user_image = $3
                    WHERE id = $4
                    RETURNING *
                "#, info.full_name, info.bio, info.image, self.data.id)
            .fetch_one(self.db.as_ref())
            .await
            .map_err(AppError::from)?;
        self.data = new_data;
        Ok(())
    }

    pub fn get_data(&self) -> &UserData{
        &self.deref()
    }
}

impl std::ops::Deref for User{
    type Target = UserData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}