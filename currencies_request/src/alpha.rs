use std::{
    time::Duration,
    collections::HashMap,
};
use chrono::prelude::*;
use reqwest::{
    Client,
    ClientBuilder,
};
use serde::{
    Deserialize, 
    Serialize
};
use crate::{
    errors::CurrencyError,
    types::{
        CurrencyResult,
        CurrencyValue,
        CurrencyChange,
        CurrencyType::{
            self,
            EUR,
            USD
        },
    }
};
use derive_new::new;


#[derive(Serialize, Deserialize, Debug)]
struct AlphaCurrency{
    #[serde(rename(serialize = "type", deserialize = "type"))] // https://serde.rs/field-attrs.html
    type_val: String,
    date: String,
    value: f32,
    order: String
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(new)]
struct AlphaBuyAndSellInfo<'a>{
    cur_type: CurrencyType,
    buy: &'a AlphaCurrency,
    sell: &'a AlphaCurrency,
    update_time: Option<DateTime<Utc>>
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

impl CurrencyValue{
    fn from_alpha<'a>(info: AlphaBuyAndSellInfo<'a>) -> Result<Self, CurrencyError> {
        // Изменения в стоимости
        let buy_change = order_to_change(info.buy, info.cur_type)?;
        let sell_change = order_to_change(info.sell, info.cur_type)?;

        let usd_result = CurrencyValue::new(info.cur_type, 
                                                           info.buy.value, 
                                                           info.sell.value, 
                                                           buy_change, 
                                                           sell_change);
        Ok(usd_result)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

fn get_buy_and_sell(info: &Vec<AlphaCurrency>, cur_type: CurrencyType) -> Result<AlphaBuyAndSellInfo, CurrencyError>{
    let buy: &AlphaCurrency = info
        .iter()
        .find(|val|{
            val.type_val.eq("buy")
        })
        .ok_or(CurrencyError::NoBuyInfo(cur_type))?;

    let sell: &AlphaCurrency = info
        .iter()
        .find(|val|{
            val.type_val.eq("sell")
        })
        .ok_or(CurrencyError::NoSellInfo(cur_type))?;

    // Время
    let chrono_time = Utc.datetime_from_str(buy.date.as_str(), "%Y-%m-%d %H:%M:%S"); // "2014-11-28 12:00:09" "2020-05-07 12:29:00" "2020-05-07 12:29:00"
    //println!("{:?}", chrono_time);

    Ok(AlphaBuyAndSellInfo::new(cur_type, buy, sell, chrono_time.ok()))
}

fn order_to_change(cur: &AlphaCurrency, cur_type: CurrencyType) -> Result<CurrencyChange, CurrencyError> {
    match cur.order.as_str() {
        "-" => Ok(CurrencyChange::Increase),
        "+" => Ok(CurrencyChange::Decrease),
        "0" => Ok(CurrencyChange::NoChange),
        _ => return Err(CurrencyError::NoChangeInfo(cur_type))
    }
}

pub async fn get_currencies_from_alpha() -> Result<CurrencyResult, CurrencyError> {
    // Создаем клиента для запроса
    let client: Client = ClientBuilder::new()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(3))
        .build()?;

    // Получаем json
    // "https://alfabank.ru/ext-json/0.2/exchange/cash?offset=0&limit=1&mode=rest"
    let json: HashMap<String, Vec<AlphaCurrency>> = client
        .get("https://alfabank.ru/ext-json/0.2/exchange/cash")
        .query(&[("offset", "0"), ("mode", "rest")])
        .send()
        .await?
        .json()
        .await?;

    //println!("{:?}", json);

    // Дергаем значения
    let usd: &Vec<AlphaCurrency> = json
        .get("usd")
        .ok_or(CurrencyError::NoData(USD))?;
    let eur: &Vec<AlphaCurrency> = json
        .get("eur")
        .ok_or(CurrencyError::NoData(EUR))?;

    // Получаем фактические результаты
    let usd_info = get_buy_and_sell(usd, USD)?;
    let eur_info = get_buy_and_sell(eur, EUR)?;

    // Время обновления
    let time = usd_info.update_time;

    // Создаем универсальные структурки с результатом
    let usd_result = CurrencyValue::from_alpha(usd_info)?;
    let eur_result = CurrencyValue::from_alpha(eur_info)?;
    
    Ok(CurrencyResult::new(usd_result, eur_result, time))
}