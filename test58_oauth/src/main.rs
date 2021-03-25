mod error;
mod facebook_env_params;
mod app_middlewares;
mod constants;

use actix_files::{
    Files
};
use actix_web::{
    web::{
        self
    },
    guard::{
        self
    },
    HttpServer,
    App
};
use handlebars::{
    Handlebars
};
use log::{
    debug
};
use rand::{
    Rng
};
use actix_identity::{
    CookieIdentityPolicy, 
    IdentityService,
    Identity
};
use serde::{
    Deserialize
};
use crate::{
    error::{
        AppError
    },
    facebook_env_params::{
        FacebookEnvParams
    },
    app_middlewares::{
        create_error_middleware,
        create_check_login_middleware
    }
};

async fn index(handlebars: web::Data<Handlebars<'_>>) -> Result<web::HttpResponse, AppError> {
    let body = handlebars.render(constants::INDEX_TEMPLATE, &serde_json::json!({}))?;

    Ok(web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body))
}

async fn login_page(handlebars: web::Data<Handlebars<'_>>) -> Result<web::HttpResponse, AppError> {
    let body = handlebars.render(constants::LOGIN_TEMPLATE, &serde_json::json!({}))?;

    Ok(web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body))
}

/// Данный метод вызывается при нажатии на кнопку логина в Facebook
async fn login_with_facebook(req: actix_web::HttpRequest, fb_params: web::Data<FacebookEnvParams>) -> Result<web::HttpResponse, AppError> {
    debug!("Request object: {:?}", req);

    // Адрес нашего сайта + адрес коллбека
    /*let callback_site_address = {
        let site_addr = req
            .headers()
            .get(actix_web::http::header::ORIGIN)
            .and_then(|val|{
                val.to_str().ok()
            })
            .ok_or_else(||{
                AppError::ActixError(actix_web::error::ErrorBadRequest("Origin header get failed"))
            })?;
        format!("{}/facebook/auth_callback", site_addr)
    };*/
    let callback_site_address = {
        let conn_info = req.connection_info();
        format!("{scheme}://{host}/facebook/auth_callback", 
                    scheme = conn_info.scheme(),
                    host = conn_info.host())
    };

    // Создаем урл, на который надо будет редиректиться браузеру для логина
    // https://www.facebook.com/dialog/oauth\
    //      ?client_id=578516362116657\
    //      &redirect_uri=http://localhost/facebook-auth\
    //      &response_type=code\
    //      &scope=email,user_birthday
    let mut redirect_url = url::Url::parse("https://www.facebook.com/dialog/oauth")?;
    redirect_url
        .query_pairs_mut()
        .append_pair("client_id", &fb_params.client_id)
        .append_pair("redirect_uri", &callback_site_address)
        .append_pair("response_type", "code")
        .append_pair("scope", "email")
        .finish();

    debug!("Facebook url value: {}", redirect_url);

    // Возвращаем код 302 и Location в заголовках для перехода
    Ok(web::HttpResponse::Found()
        .header(actix_web::http::header::LOCATION, redirect_url.as_str())
        .finish())
}

/// Данный метод является адресом-коллбеком который вызывается после логина на facebook
#[derive(Debug, Deserialize)]
pub struct FacebookAuthParams{
    code: String
}
async fn facebook_auth_callback(req: actix_web::HttpRequest,
                                query_params: web::Query<FacebookAuthParams>, 
                                identity: Identity,
                                fb_params: web::Data<FacebookEnvParams>,
                                http_client: web::Data<reqwest::Client>) -> Result<web::HttpResponse, AppError> {

    let callback_site_address = {
        let conn_info = req.connection_info();
        format!("{scheme}://{host}/facebook/auth_callback", 
                    scheme = conn_info.scheme(),
                    host = conn_info.host())
    };

    debug!("Request object: {:?}", req);
    debug!("Facebook auth callback query params: {:?}", query_params);

    // Выполняем запрос для получения токена на основании кода у редиректа
    // TODO: Error "{\"error\":{\"message\":\"Error validating verification code. Please make sure your redirect_uri is identical to the one you used in the OAuth dialog request\",\"type\":\"OAuthException\",\"code\":100,\"error_subcode\":36008,\"fbtrace_id\":\"Ansfo6SIPxC6Q1ZX2IRTSba\"}}"
    // TODO: "{\"access_token\":\"EAAD9jJvESf0BAJOGgIKVyNbsPKawpqPk9og0lO0Y7iAiWkkB7vc3HOyeo2j9KRgBIpI1kubW6sv0eWO2ewpQAUuAfvZBYuMUR8XhXyehuWeLu4LQlPmDJuZAi7axZCfApKcHs5duxhaMPhvTDKcHZBvowouNojAm2xFHcqFWRQZDZD\",\"token_type\":\"bearer\",\"expires_in\":5181506}"
    let response = http_client
        .get("https://graph.facebook.com/oauth/access_token")
        .query(&[
            ("client_id", fb_params.client_id.as_str()),
            ("redirect_uri", callback_site_address.as_str()),   // TODO: Для чего он нужен?
            ("client_secret", fb_params.client_secret.as_str()),
            ("code", query_params.code.as_str())
        ])
        .send()
        .await?
        .text()
        .await?;

    debug!("Facebook token request response: {:?}", response);

    // Возвращаем код 302 и Location в заголовках для перехода
    Ok(web::HttpResponse::Found()
        .header(actix_web::http::header::LOCATION, constants::INDEX_PATH)
        .finish())
}

/// Функция непосредственного конфигурирования приложения
/// Для каждого потока исполнения будет создано свое приложение
fn configure_new_app(config: &mut web::ServiceConfig) {
    config
        .service(web::resource(constants::INDEX_PATH)
                    .wrap(create_check_login_middleware())
                    .route(web::route()
                            .guard(guard::Get())
                            .to(index)))
        .service(web::resource(constants::LOGIN_PATH)
                        .wrap(create_check_login_middleware())
                        .route(web::route()
                                .guard(guard::Get())
                                .to(login_page)))                         
        .service(web::scope("/facebook")
                    .service(web::resource("/login")
                                .route(web::route()
                                        .guard(guard::Post())
                                        .to(login_with_facebook)))
                    .service(web::resource("/auth_callback")
                                .route(web::route()
                                        .guard(guard::Get())
                                        .to(facebook_auth_callback))))
        .service(Files::new("static/css", "static/css"))
        .service(Files::new("static/js", "static/js"));
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    // Получаем параметры Facebook
    let facebook_env_params = web::Data::new(FacebookEnvParams::get_from_env());

    // Создаем шареную ссылку на обработчик шаблонов
    // Пример работы с шаблонами
    // https://github.com/actix/examples/tree/master/template_engines/handlebars
    let handlebars = {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_file(constants::INDEX_TEMPLATE, "templates/index.hbs")
            .unwrap();
        handlebars
            .register_template_file(constants::LOGIN_TEMPLATE, "templates/login.hbs")
            .unwrap();
        handlebars
            .register_template_file(constants::ERROR_TEMPLATE, "templates/error.hbs")
            .unwrap();        
        web::Data::new(handlebars)
    };

    // Ключ для шифрования кук, генерируется каждый раз при запуске сервера
    let private_key = rand::thread_rng().gen::<[u8; 32]>();

    // Создаем общего http клиента для разных запросов
    let http_client = web::Data::new(reqwest::Client::new());

    HttpServer::new(move ||{
            // Настраиваем middleware идентификации пользователя, делает зашифрованную куку у пользователя в браузере,
            // тем самым давая возможность проверять залогинен ли пользователь или нет
            let identity_middleware = {
                let policy = CookieIdentityPolicy::new(&private_key)
                    .name("auth-logic")
                    .max_age(60 * 60 * 24 * 30) // 30 дней максимум
                    .secure(false);
                IdentityService::new(policy)
            };

            // Приложение создается для каждого потока свое собственное
            App::new()
                .wrap(create_error_middleware())
                .wrap(identity_middleware)
                .wrap(actix_web::middleware::Logger::default())
                .app_data(handlebars.clone())
                .app_data(facebook_env_params.clone())
                .app_data(http_client.clone())
                .configure(configure_new_app)
        }) 
        .bind("127.0.0.1:8080")?
        .run()
        .await
}