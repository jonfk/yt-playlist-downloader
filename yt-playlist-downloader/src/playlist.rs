
use google_youtube3::YouTube;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::instrument;
use tracing::{info, Level};

use crate::{Playlist, RECIPE_PLAYLISTS_FILE};

#[instrument]
pub async fn read_recipe_playlists() -> Vec<Playlist> {
    let mut recipes_file = fs::File::open(RECIPE_PLAYLISTS_FILE).await.unwrap();

    let mut recipes_playlists = String::new();
    recipes_file.read_to_string(&mut recipes_playlists).await.unwrap();

    serde_json::from_str(&recipes_playlists).unwrap()
}

#[instrument(skip(hub))]
pub async fn update_recipe_playlists(hub: &YouTube) -> Vec<Playlist> {
    let playlists = get_all_playlists(&hub).await;

    let mut recipe_playlists: Vec<_> = playlists
        .into_iter()
        .filter(|pl| pl.title.to_ascii_lowercase().contains("recipe"))
        .collect();
    recipe_playlists.sort_by(|a, b| a.id.cmp(&b.id));

    let mut recipes_file = fs::File::create(RECIPE_PLAYLISTS_FILE).await.unwrap();

    recipes_file
        .write_all(
            serde_json::to_string_pretty(&recipe_playlists)
                .unwrap()
                .as_bytes(),
        )
        .await
        .unwrap();

    recipe_playlists
}

// https://developers.google.com/youtube/v3/docs/playlists/list
#[instrument(skip(hub))]
pub async fn get_all_playlists(hub: &YouTube) -> Vec<Playlist> {
    let parts = vec!["snippet".to_string(), "contentDetails".to_string()];
    let mut playlists = Vec::new();
    let mut first = true;
    let mut next_page_token: Option<String> = None;

    info!("downloading playlists");

    while first || next_page_token.is_some() {
        first = false;
        let mut playlist_call = hub.playlists().list(&parts).max_results(25).mine(true);

        if let Some(page_token) = &next_page_token {
            playlist_call = playlist_call.page_token(page_token);
        }

        let (_, playlists_resp) = playlist_call.doit().await.unwrap();

        next_page_token = playlists_resp.next_page_token;

        playlists.extend(playlists_resp.items.unwrap().iter().map(|playlist| {
            let content_details = playlist.clone().content_details.unwrap();
            let snippet = playlist.clone().snippet.unwrap();
            Playlist {
                title: snippet.title.unwrap(),
                published_at: snippet.published_at.unwrap(),
                item_count: content_details.item_count.unwrap(),
                id: playlist.clone().id.unwrap(),
                etag: playlist.clone().etag.unwrap(),
            }
        }));
    }

    playlists
}
