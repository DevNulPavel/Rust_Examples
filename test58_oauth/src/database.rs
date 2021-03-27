use std::{
    env::{
        self
    }
};
use log::{
    debug
};
use sqlx::{
    // prelude::{
    //     *
    // },
    sqlite::{
        SqlitePool
    }
};
use crate::{
    error::{
        AppError
    }
};

////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct UserInfo {
    pub user_uuid: String
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Database{
    db: SqlitePool
}
impl Database{
    pub async fn open() -> Database {
        let sqlite_conn = SqlitePool::connect(&env::var("DATABASE_URL")
                                                .expect("DATABASE_URL env variable is missing"))
            .await
            .expect("Database connection failed");

        // Включаем все миграции базы данных сразу в наш бинарник, выполняем при старте
        sqlx::migrate!("./migrations")
            .run(&sqlite_conn)
            .await
            .expect("database migration failed");

        Database{
            db: sqlite_conn
        }
    }

    /// Пытаемся найти нового пользователя для FB ID 
    pub async fn try_to_find_user_with_fb_id(&self, id: &str) -> Result<Option<UserInfo>, AppError>{
        sqlx::query_as!(UserInfo,
                        r#"   
                            SELECT app_users.user_uuid 
                            FROM app_users 
                            INNER JOIN facebook_users 
                                    ON facebook_users.app_user_id = app_users.id
                            WHERE facebook_users.facebook_uid = ?
                        "#, id)
            .fetch_optional(&self.db)
            .await
            .map_err(AppError::from)
    }

    pub async fn insert_uuid_for_facebook_user(&self, uuid: &str, fb_uid: &str) -> Result<(), AppError>{
        // Стартуем транзакцию, если будет ошибка, то вызовется rollback автоматически в drop
        // если все хорошо, то руками вызываем commit
        let transaction = self.db.begin().await?;

        // TODO: ???
        // Если таблица иммет поле INTEGER PRIMARY KEY тогда last_insert_rowid - это алиас
        // Но вроде бы наиболее надежный способ - это сделать подзапрос
        let new_row_id = sqlx::query!(r#"
                        INSERT INTO app_users(user_uuid)
                            VALUES (?);
                        INSERT INTO facebook_users(facebook_uid, app_user_id)
                            VALUES (?, (SELECT id FROM app_users WHERE user_uuid = ?));
                    "#, uuid, fb_uid, uuid)
            .execute(&self.db)
            .await?
            .last_insert_rowid();

        transaction.commit().await?;

        debug!("New facebook user included: row_id = {}", new_row_id);

        Ok(())
    }

    /// Пытаемся найти нового пользователя для FB ID 
    /*pub async fn try_find_user_with_uuid(&self, uuid: &str) -> Result<Option<UserInfo>, AppError>{
        sqlx::query_as!(UserInfo,
                        r#"   
                            SELECT app_users.user_uuid 
                            FROM app_users 
                            WHERE app_users.user_uuid = ?
                        "#, uuid)
            .fetch_optional(&self.db)
            .await
            .map_err(AppError::from)
    }*/

    /// Пытаемся найти нового пользователя для FB ID 
    pub async fn does_user_uuid_exist(&self, uuid: &str) -> Result<bool, AppError>{
        // TODO: Более оптимальный вариант
        let res = sqlx::query_as!(UserInfo,
                                    r#"   
                                        SELECT user_uuid 
                                        FROM app_users 
                                        WHERE app_users.user_uuid = ?
                                    "#, uuid)
            .fetch_optional(&self.db)
            .await
            .map_err(AppError::from)?;
        
        Ok(res.is_some())
    }
}