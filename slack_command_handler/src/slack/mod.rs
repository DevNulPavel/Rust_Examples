mod client;
mod view;
mod message;
mod error;
#[cfg(test)] mod tests;

pub use self::{
    client::{
        SlackClient,
        SlackMessageTaget
    },
    error::{
        SlackError
    },
    view::{
        View,
        ViewInfo,
        ViewActionHandler
    },
    message::{
        MessageInfo,
        Message
    }
};