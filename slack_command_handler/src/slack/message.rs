// use async_trait::{
//     async_trait
// };
// use actix_web::{
//     web::{
//         Data
//     }
// };
use reqwest::{
    Client
};
use serde_json::{
    json
};
use serde::{
    Deserialize
};
use super::{
    error::{
        SlackError
    }
};

////////////////////////////////////////////////////////////////

#[derive(Deserialize, Debug)]
pub struct MessageInfo{
    // bot_id: String,
    text: String,
    // username: String
}

////////////////////////////////////////////////////////////////

#[allow(dead_code)]
pub struct Message {
    client: Client,
    token: String,
    info: MessageInfo,
    channel_id: String,
    timestamp: String,
}

impl Message {
    pub fn new(client: Client, token: String, info: MessageInfo, channel_id: String, timestamp: String) -> Message{
        Message{
            client,
            token,
            info,
            channel_id,
            timestamp
        }
    }

    // pub fn get_timestamp(&self) -> &String{
    //     &self.timestamp
    // }

    // pub fn get_channel_id(&self) -> &String{
    //     &self.channel_id
    // }

    #[allow(dead_code)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub async fn update_text(&mut self, text: &str) -> Result<(), SlackError> {
        // https://api.slack.com/methods/chat.update#arg_ts

        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum MessageResponse{
            Ok{
                ok: bool,
                channel: String,
                text: String,
                ts: String
            },
            Err{
                ok: bool,
                error: String
            }
        };

        let data = json!({
            "channel": self.channel_id,
            "ts": self.timestamp,
            "attachments": [
                {
                    "text": text
                }
            ]
        });

        let response = self.client
            .post("https://slack.com/api/chat.update")
            .bearer_auth(&self.token)
            .header("Content-type", "application/json")
            .body(serde_json::to_string(&data).unwrap())
            .send()
            .await
            .map_err(|err|{
                SlackError::RequestErr(err)
            })?
            .json::<MessageResponse>()
            .await
            .map_err(|err|{
                SlackError::JsonParseError(err)
            })?;

        match response {
            MessageResponse::Ok{ok, ts, channel, ..} =>{
                if ok {
                    self.timestamp = ts;
                    self.channel_id = channel;

                    return Ok(());
                }else{
                    return Err(SlackError::Custom(format!("Slack message update response: {}", ok)))
                }
            },
            MessageResponse::Err{error, ..} => {
                return Err(SlackError::Custom(error))
            }
        }
    }
}