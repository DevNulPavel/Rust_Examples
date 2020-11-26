use log::{
    debug,
    // info,
    error
};
use actix_web::{ 
    web::{
        Data
    },
    // Responder,
    HttpResponse
};
use serde_json::{
    Value
};
use crate::{
    jenkins::{
        api::{
            request_jenkins_job_info,
            Parameter,
            ChoiseList,
            ChoiseInfo
        }
    },
    ApplicationData
};
use super::{
    parameters::{
        WindowParametersViewInfo
    }
};

/// Получаем из вьюшки имя нашего таргета
fn get_selected_target(view: &WindowParametersViewInfo) -> Option<&str>{
    view.state.values
        .get("build_target_block_id")
        .and_then(|val|{
            val.get("build_target_action_id")
        })
        .and_then(|val|{
            val.get("selected_option")
        })
        .and_then(|val|{
            val.get("value")
        })
        .and_then(|val|{
            val.as_str()
        })
}

fn param_to_json_field(param: Parameter) -> Value {
    // Примеры компонентов
    // https://api.slack.com/reference/block-kit/block-elements
    // https://app.slack.com/block-kit-builder/
    match param {
        Parameter::Boolean{name, ..} => {
            /*serde_json::json!({
                "type": "actions",
                "element": {
                    "type": "checkboxes",
                    "options": [
                        {
                            "text": {
                                "type": "plain_text",
                                "text": name,
                                "emoji": true
                            },
                            "value": "value-0"
                        }
                    ]
                }
            })*/
            serde_json::json!({
                "type": "section",
                "text": {
                    "type": "plain_text",
                    "text": name,
                    "emoji": true
                }
            })
        },
        Parameter::Choice{name, ..} => {
            /*serde_json::json!({
                "type": "section",
                "text": {
                    "type": "plain_text",
                    "text": name,
                    "emoji": true
                }
            })*/
            serde_json::json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": name
                },
                "accessory": {
                    "type": "radio_buttons",
                    "options": [
                        {
                            "text": {
                                "type": "plain_text",
                                "text": "*this is plain_text text*",
                                "emoji": true
                            },
                            "value": "value-0"
                        }
                    ],
                    "action_id": "radio_buttons-action"
                }
            })
        },
        Parameter::ChoiceSimple{name, ..} => {
            serde_json::json!({
                "type": "section",
                "text": {
                    "type": "plain_text",
                    "text": name,
                    "emoji": true
                }
            })
        },
        Parameter::Git{name, ..} => {
            serde_json::json!({
                "type": "section",
                "text": {
                    "type": "plain_text",
                    "text": name,
                    "emoji": true
                }
            })
        },
        Parameter::String{name, ..} => {
            serde_json::json!({
                "type": "input",
                //"block_id": "input123",
                "label": {
                    "type": "plain_text",
                    "text": name
                },
                "element": {
                    "type": "plain_text_input",
                    "action_id": "plain_input",
                    "placeholder": {
                        "type": "plain_text",
                        "text": "Enter some plain text"
                    }
                }
            })
        }
    }
}

// https://api.slack.com/surfaces/modals/using
pub async fn open_build_properties_window_by_reponse(view: WindowParametersViewInfo, app_data: Data<ApplicationData>) -> HttpResponse {
    // https://api.slack.com/surfaces/modals/using#preparing_for_modals

    // Получаем из недр Json имя нужного нам таргета сборки
    let selected_target = {
        match get_selected_target(&view) {
            Some(target) => {
                target
            },
            None =>{
                // TODO: Error
                error!("Select target error");
                return HttpResponse::Ok()
                    .body(format!("Select target error"))
            }
        }        
    };

    // Запрашиваем список параметров данного таргета
    let parameters = match request_jenkins_job_info(&app_data.http_client, 
                                                    &app_data.jenkins_auth,
                                                    selected_target).await{
        Ok(parameters) => {
            parameters
        },
        Err(err) => {
            error!("Job info request error: {:?}", err);
            return HttpResponse::Ok()
                .body(format!("Select target error: {:?}", err));
        }
    };

    debug!("Parameters list: {:?}", parameters);

    // Параметры конвертируем в поля на окне
    let parameter_blocks = parameters
        .into_iter()
        .map(|param|{
            param_to_json_field(param)
        })
        .collect::<Vec<serde_json::Value>>();

    // TODO: Не конвертировать туда-сюда json
    // let j = r#"
    //     {
    //     "id": "demo-deserialize-max",
    //     "values": [
    //     ]
    //     }
    // "#;
    let new_window = serde_json::json!(
        {
            "response_action": "push",
            "view": {
                "type": "modal",
                "callback_id": "modal-identifier",
                "title": {
                    "type": "plain_text",
                    "text": "Updated view"
                },
                "submit": {
                    "type": "plain_text",
                    "text": "Submit",
                    "emoji": true
                },
                "close": {
                    "type": "plain_text",
                    "text": "Cancel",
                    "emoji": true
                },
                "blocks": parameter_blocks
            }
        }                    
    );

    /*[
                    {
                        "type": "input",
                        //"block_id": "input123",
                        "label": {
                            "type": "plain_text",
                            "text": "asd"
                        },
                        "element": {
                            "type": "plain_text_input",
                            "action_id": "plain_input",
                            "placeholder": {
                                "type": "plain_text",
                                "text": "Enter some plain text"
                            }
                        }
                    },
                    {
                        "type": "section",
                        "text": {
                          "type": "plain_text",
                          "text": "Check out these rad radio buttons"
                        },
                        "accessory": {
                            "type": "radio_buttons",
                            "action_id": "this_is_an_action_id",
                            "initial_option": {
                                "value": "A1",
                                "text": {
                                    "type": "plain_text",
                                    "text": "Radio 1"
                                }
                            },
                            "options": [
                                {
                                    "value": "A1",
                                    "text": {
                                        "type": "plain_text",
                                        "text": "Radio 1"
                                    }
                                },
                                {
                                    "value": "A2",
                                    "text": {
                                        "type": "plain_text",
                                        "text": "Radio 2"
                                    }
                                }
                            ]
                        }
                    }                    
                    /*{
                        "type": "image",
                        "image_url": "https://api.slack.com/img/blocks/bkb_template_images/plants.png",
                        "alt_text": "Plants"
                      },
                    {
                        "type": "context",
                        "elements": [
                          {
                            "type": "mrkdwn",
                            "text": "_Two of the author's cats sit aloof from the austere challenges of modern society_"
                          }
                        ]
                    }*/
                ]
        */

    HttpResponse::Ok()
        .header("Content-type", "application/json")
        .body(serde_json::to_string(&new_window).unwrap())
}