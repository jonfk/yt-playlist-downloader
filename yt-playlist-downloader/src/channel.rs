use google_youtube3::YouTube;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::instrument;
use tracing::{info, Level};

use crate::Thumbnails;

pub struct Channel {
    pub id: String,
    pub title: String,
    pub thumbnails: Thumbnails,
}

pub async fn update_channels(hub: &YouTube, channel_ids: Vec<String>) {}

async fn get_channel(hub: &YouTube, channel_ids: Vec<String>) -> Vec<Channel> {
    let parts = vec!["snippet".to_string(), "id".to_string()];
    let mut channels = Vec::new();
    let mut first = true;
    let mut next_page_token: Option<String> = None;

    while first || next_page_token.is_some() {
        first = false;
        let mut channels_call = hub.channels().list(&parts).max_results(50);
        for channel_id in &channel_ids {
            channels_call = channels_call.add_id(channel_id);
        }
        if let Some(page_token) = &next_page_token {
            channels_call = channels_call.page_token(page_token);
        }

        let (_, channels_resp) = channels_call.doit().await.unwrap();
        next_page_token = channels_resp.clone().next_page_token;

        channels.extend(channels_resp.items.unwrap().into_iter().map(|channel| {
            let snippet = channel.snippet.unwrap();
            Channel {
                id: channel.id.unwrap(),
                title: snippet.title.unwrap(),
                thumbnails: Thumbnails::from(&snippet.thumbnails.unwrap()),
            }
        }));
    }

    channels
}
