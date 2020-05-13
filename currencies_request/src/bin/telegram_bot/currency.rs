
use std::{
    // collections::HashSet,
    collections::HashMap,
    convert::TryFrom
    //pin::Pin,
    //future::Future
};
use chrono::prelude::*;
// use chrono::{
    // Utc,
    // DateTime
// };
// use futures::{
    // stream::{
        // StreamExt
    // }
// };
use telegram_bot::{
    // Api,
    Error,
    Message,
    CanSendMessage,
    ParseMode,
    UserId
};
// use reqwest::{
    //Client,
// };
use currencies_request::{
    CurrencyError,
    CurrencyResult,
    CurrencyMinimum,
    CurrencyValue,
    CurrencyType,
    //CurrencyChange,
    get_all_currencies
};
use sqlx::{
    prelude::*,
    // query,
    // Connect,
    // Connection,
    // Executor,
    // Cursor,
    sqlite::{
        SqliteConnection,
        // SqliteCursor,
        SqliteRow
    }
};
use crate::{
    app_context::{
        AppContext
    },
    bot_context::{
        BotContext
    }
};


pub struct CurrencyUsersStorrage{
    users_for_push: HashMap<UserId, CurrencyCheckStatus>
}

impl CurrencyUsersStorrage{
    pub async fn new(conn: &mut SqliteConnection) -> Self{
        //let mut conn = *conn;

        let result_ids: Vec<i64> = sqlx::query("SELECT user_id FROM monitoring_users")
            .map(|row: SqliteRow| {
                let user_id: i64 = row.get(0);
                user_id
            })
            .fetch_all(&mut *conn)
            .await
            .expect("Failed select users form DB");

        /*let conn_iter = std::iter::repeat(conn);
        let results: Vec<CurrencyCheckStatus> = tokio::stream::iter(results)
            .zip(tokio::stream::iter(conn_iter))
            .then(|(user_id, conn)| {
                println!("Load user with id from database: {}", user_id);

                // TODO: загрузка минимумов внутри объекта
                CurrencyCheckStatus::load(UserId::new(user_id), *conn)
            })
            .collect()
            .await;*/
        
        // TODO: убрать mut и переделать на stream
        let mut results: Vec<CurrencyCheckStatus> = vec![];
        results.reserve(result_ids.len());
        for id_val in result_ids{
            let status = CurrencyCheckStatus::load(UserId::new(id_val), &mut (*conn)).await;
            results.push(status);
        }
        
        let map: HashMap<UserId, CurrencyCheckStatus> = results
            .into_iter()
            .map(|val|{
                (val.user.clone(), val)
            })
            .collect();

        CurrencyUsersStorrage{
            users_for_push: map
        }
    }

    // TODO: Ошибка нормальная
    pub async fn add_user(&mut self, user: &UserId, conn: &mut SqliteConnection) -> Result<(), ()>{
        // TODO: Очистка минимумов перед добавлением
        let check = CurrencyCheckStatus::new(user.clone());

        let id_num: i64 = (*user).into();
        let insert_result = sqlx::query("INSERT INTO monitoring_users(user_id) VALUES (?)")
            .bind(id_num)
            .execute(conn)
            .await;

        match insert_result{
            Ok(_) => {
                self.users_for_push.insert(user.clone(), check);
                Ok(())
            },
            Err(e) => {
                println!("Insert user SQL error: {}", e);
                Err(())
            }
        }
    }

    // TODO: Ошибка нормальная
    pub async fn remove_user(&mut self, user: &UserId, conn: &mut SqliteConnection) -> Result<(), ()>{
        // TODO: Нужна ли транзакция? Можно ли как-то удалить все, что относится к user
        const SQL: &str =   "BEGIN; \
                                DELETE FROM currency_minimum WHERE user_id = ?; \
                                DELETE FROM monitoring_users WHERE user_id = ?; \
                            COMMIT;";

        // TODO: Очистка минимумов перед добавлением
        let id_num: i64 = (*user).into();
        let remove_result = sqlx::query(SQL)
            .bind(id_num)
            .bind(id_num)
            .execute(conn)
            .await;
        

        match remove_result{
            Ok(_)=>{
                self.users_for_push.remove(user);
                Ok(())    
            },
            Err(e)=>{
                println!("Delete user SQL error: {}", e);
                Err(())
            }
        }
    }

    // TODO: Mut iter
    /*fn iter_users(&self) -> std::collections::hash_map::Iter<UserId, CurrencyCheckStatus> {
        self.users_for_push.iter()
    }*/

    pub fn is_empty(&self) -> bool{
        self.users_for_push.is_empty()
    }
}

///////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct CurrencyCheckStatus {
    user: UserId,
    minimum_values: Vec<CurrencyMinimum>,
    //minimum_time: DateTime<Utc>,        // TODO:
    //last_check_time: DateTime<Utc>,     // TODO:
}

impl CurrencyCheckStatus{
    pub fn new(user: UserId) -> Self{
        CurrencyCheckStatus{
            user: user,
            minimum_values: vec![]
        }
    }
    pub async fn load(user: UserId, conn: &mut SqliteConnection) -> Self{
        const SQL: &str = "SELECT * FROM currency_minimum \
                            WHERE user_id = ?";
        let user_id: i64 = user.into();
        let results: Vec<CurrencyMinimum> = sqlx::query(SQL)
            .bind(user_id)
            .map(|row: SqliteRow| {
                let type_str: String = row.get("cur_type");
                let cur_type = CurrencyType::try_from(type_str.as_str()).expect("Invalid currency type");
                let res = CurrencyMinimum{
                    bank_name: row.get("bank_name"),
                    value: row.get("value"),
                    cur_type: cur_type,
                    update_time: None // TODO: ???
                };
                println!("Load user's minimum for id = {} from database: {:?}", user_id, res);
                res
            })
            .fetch_all(conn)
            .await
            .expect("Failed select user's minimums form DB");

        CurrencyCheckStatus{
            user: user,
            minimum_values: results
        }
    }
}

// Пустая реализация на базе PartialEq без перегрузок
impl Eq for CurrencyCheckStatus {
}

impl PartialEq for CurrencyCheckStatus{
    fn eq(&self, other: &CurrencyCheckStatus) -> bool{
        self.user.eq(&other.user)
    }
}

impl std::hash::Hash for CurrencyCheckStatus {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.user.hash(state);
    }
}

impl CurrencyCheckStatus{
}

///////////////////////////////////////////////////////////////////////////////////////

async fn process_currency_value(conn: &mut SqliteConnection,
                                user_id: &UserId, 
                                previous_value: Option<CurrencyMinimum>, 
                                bank_name: &str,
                                update_time: Option<DateTime<Utc>>,
                                received_value: &CurrencyValue) ->  Option<CurrencyMinimum> {
    // Если есть предыдущее значение
    if let Some(previous_value) = previous_value {
        // Проверяем минимум
        let is_lower = previous_value.value > received_value.buy;

        // TODO: Test
        if is_lower {
            // user_id integer,
            // bank_name varchar(16),
            // usd integer,
            // eur integer,
            // update_time varchar(32),

            println!("New minimum update");

            // TODO: Optimize
            let user_id_int: i64 = (*user_id).into();
            let type_str: &'static str = received_value.cur_type.into();
            let q = sqlx::query("BEGIN; \
                                DELETE FROM currency_minimum WHERE user_id = ? AND cur_type = ?; \
                                INSERT INTO currency_minimum(user_id, bank_name, value, cur_type, update_time) VALUES (?, ?, ?, ?, ?); \
                                COMMIT;")
                .bind(user_id_int)
                .bind(type_str)
                .bind(user_id_int)
                .bind(bank_name)
                .bind(&received_value.buy)
                .bind(type_str)
                .bind(""); // TODO: Date
            
            let query_result = q.execute(&mut (*conn)).await;
            match query_result{
                Ok(_)=>{
                    // Обновляем значение
                    let minimum = CurrencyMinimum{
                        bank_name: bank_name.to_string(),
                        value: received_value.buy,
                        cur_type: received_value.cur_type,
                        update_time: update_time
                    };

                    return Some(minimum);
                },
                Err(e) => {
                    println!("Insert new minimum error: {}", e);
                }
            }
        }
    }else{
        println!("New minimum");

        // TODO: Optimize
        let user_id_int: i64 = (*user_id).into();
        let type_str: &'static str = received_value.cur_type.into();
        let q = sqlx::query("BEGIN; \
                            DELETE FROM currency_minimum WHERE user_id = ? AND cur_type = ?; \
                            INSERT INTO currency_minimum(user_id, bank_name, value, cur_type, update_time) VALUES (?, ?, ?, ?, ?); \
                            COMMIT;")
            .bind(user_id_int)
            .bind(type_str)
            .bind(user_id_int)
            .bind(bank_name)
            .bind(&received_value.buy)
            .bind(type_str)
            .bind(""); // TODO: Date
        
        let query_result = q.execute(&mut (*conn)).await;
        match query_result{
            Ok(_)=>{
                // Иначе вкидываем новое
                let minimum = CurrencyMinimum{
                    bank_name: bank_name.to_string(),
                    value: received_value.buy,
                    cur_type: received_value.cur_type,
                    update_time: update_time
                };

                return Some(minimum)
            },
            Err(e) => {
                println!("Insert new minimum error: {}", e);
            }
        }
    }

    None
}

pub async fn check_currencies_update(bot_context: &mut BotContext) {
    // Если некому слать - выходим
    if bot_context.app_context.users_for_push.is_empty(){
        return;
    }

    // Получаем новые значения
    let received_bank_currencies: Vec<CurrencyResult> = 
        get_all_currencies(&bot_context.client).await
        .into_iter()
        .filter_map(|result|{
            result.ok()
        })
        .collect();

    // Если пусто - выходим
    if received_bank_currencies.is_empty(){
        return;
    }

    // Кому и какие обновления рассылаем
    let mut updates: HashMap<UserId, Vec<String>> = HashMap::new();

    // TODO: Переделать на Reactive
    // Идем по всем пользователям, у которых включено оповещение о снижении
    let AppContext {
        users_for_push: ref mut users_container,
        db_conn: ref mut conn,
        ..
    } = bot_context.app_context;

    // Перебираем информацию о каждом банке, которую получили
    let usd_received_minimum = received_bank_currencies
        .iter()
        .min_by(|val1, val2|{
            if val1.usd.buy < val2.usd.buy{
                std::cmp::Ordering::Less
            }else if val1.usd.buy > val2.usd.buy{
                std::cmp::Ordering::Greater
            }else{
                std::cmp::Ordering::Equal
            }
        });

    println!("Minimum USD: {:?}\n", usd_received_minimum);

    let eur_received_minimum = received_bank_currencies
        .iter()
        .min_by(|val1, val2|{
            if val1.usd.buy < val2.usd.buy{
                std::cmp::Ordering::Less
            }else if val1.usd.buy > val2.usd.buy{
                std::cmp::Ordering::Greater
            }else{
                std::cmp::Ordering::Equal
            }
        });

    println!("Minimum EUR: {:?}\n", usd_received_minimum);

    for (user_id, user_subscribe_info) in &mut users_container.users_for_push {
        let mut updates_for_user: Vec<String> = vec![];

        // Если у юзера есть предыдущие значения
        let previous_minimum_values: &mut Vec<CurrencyMinimum> = &mut user_subscribe_info.minimum_values;
        
        if let Some(usd_received_minimum) = usd_received_minimum {
            // Ищем предыдущее значение для банка у юзера
            let previous_usd: Option<CurrencyMinimum> = previous_minimum_values
                .iter()
                .find(|value|{
                    value.cur_type == CurrencyType::USD
                })
                .map(|ref_val|{
                    ref_val.clone()
                });

            // Если есть предыдущее значение
            let new_minimum = process_currency_value(conn,
                                                     user_id,
                                                     previous_usd,  
                                                     &usd_received_minimum.bank_name,
                                                     usd_received_minimum.update_time,
                                                     &usd_received_minimum.usd).await;

            if let Some(new_minimum) = new_minimum{
                let result = markdown_format_minimum(&new_minimum, &usd_received_minimum.usd);
                updates_for_user.push(result);

                let old = previous_minimum_values
                    .iter_mut()
                    .find(|val|{
                        val.cur_type == new_minimum.cur_type 
                    });
                if let Some(old) = old {
                    *old = new_minimum;
                }else{
                    previous_minimum_values.push(new_minimum);
                }
            }
        }

        if let Some(eur_received_minimum) = eur_received_minimum {
            // Ищем предыдущее значение для банка у юзера
            let previous_eur: Option<CurrencyMinimum> = previous_minimum_values
                .iter()
                .find(|value|{
                    value.cur_type == CurrencyType::EUR
                })
                .map(|ref_val|{
                    ref_val.clone()
                });


            // Если есть предыдущее значение
            // Если есть предыдущее значение
            let new_minimum = process_currency_value(conn,
                                                     user_id,
                                                     previous_eur,  
                                                     &eur_received_minimum.bank_name,
                                                     eur_received_minimum.update_time,
                                                     &eur_received_minimum.eur).await;

            if let Some(new_minimum) = new_minimum{
                let result = markdown_format_minimum(&new_minimum, &eur_received_minimum.eur);
                updates_for_user.push(result);

                let old = previous_minimum_values
                    .iter_mut()
                    .find(|val|{
                        val.cur_type == new_minimum.cur_type 
                    });
                if let Some(old) = old {
                    *old = new_minimum;
                }else{
                    previous_minimum_values.push(new_minimum);
                }
            }
        }        

        if !updates_for_user.is_empty(){
            updates.insert(user_id.clone(), updates_for_user);
        }
    }

    if updates.is_empty() {
        return;
    }

    let futures_iter = updates
        .into_iter()
        .map(|(user_id, updates_for_user)|{
            let text: String = updates_for_user
                .into_iter()
                .collect();
            let mut req = telegram_bot::types::requests::SendMessage::new(user_id, text);
            req.parse_mode(ParseMode::Markdown);
            bot_context.api.send(req)
        });

    futures::future::join_all(futures_iter).await;
}

fn markdown_format_currency_result(info: &CurrencyResult) -> String{
    let time_str: String = match info.update_time {
        Some(time) => time.format("%H:%M %d-%m-%Y").to_string(),
        None => "No time".into()
    };

    let bank_text = format!(   "*{} ({})*:\n\
                                ```\n$: buy = {:.2} {}, sell = {:.2} {}\n\
                                   €: buy = {:.2} {}, sell = {:.2} {}```\n",
            info.bank_name,
            time_str,
            info.usd.buy,
            info.usd.buy_change,
            info.usd.sell,
            info.usd.sell_change,
            info.eur.buy,
            info.eur.buy_change,
            info.eur.sell,
            info.eur.sell_change);

    bank_text
}

fn markdown_format_minimum(new_minimum: &CurrencyMinimum, previous_value: &CurrencyValue) -> String{
    let time_str: String = match new_minimum.update_time {
        Some(time) => time.format("%H:%M %d-%m-%Y").to_string(),
        None => "No time".into()
    };

    let bank_text = format!(   "New buy minimum\n\
                                *{} ({})*:\n\
                                ```\n\
                                {}: new = {:.2}, old = {:.2}```\n",
            new_minimum.bank_name,
            time_str,
            new_minimum.cur_type,
            new_minimum.value,
            previous_value.buy);

    bank_text
}

pub async fn process_currencies_command(bot_context: &BotContext, message: &Message) -> Result<(), Error> {
    let mut text = String::new();

    // Выводим текст, используем into_iter для потребляющего итератора
    for info in get_all_currencies(&bot_context.client).await {
        let info: Result<CurrencyResult, CurrencyError> = info;
        match info {
            Ok(info) =>{
                let info: CurrencyResult = info;

                let bank_text = markdown_format_currency_result(&info);
                
                text.push_str(bank_text.as_str())
            },
            Err(_e) => {
                // TODO: Вывод ошибок
                /*let row = Row::new(vec![
                    Cell::new(format!("{:?}", e).as_str()),
                ]);
                table.add_row(row);*/
                println!("{:?}", _e);
            }
        }
    }

    let mut private_messaage = message.from.text(text);
    bot_context.api.send(private_messaage.parse_mode(ParseMode::Markdown)).await?;

    Ok(())
}