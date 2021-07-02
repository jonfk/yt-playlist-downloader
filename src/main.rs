use google_youtube3::YouTube;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::instrument;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

#[tokio::main]
async fn main() {
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::INFO)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let hub = build_yt_api().await;

    update_recipe_playlists(&hub).await;
}

async fn build_yt_api() -> YouTube {
    let secret = yup_oauth2::read_application_secret("clientsecret.json")
        .await
        .expect("clientsecret.json");

    let mut auth =
        InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
            .persist_tokens_to_disk("tokencache.json")
            .build()
            .await
            .unwrap();

    let _scopes = &["https://www.googleapis.com/auth/youtube.readonly"];

    // match auth.token(scopes).await {
    //     Ok(token) => println!("The token is {:?}", token),
    //     Err(e) => println!("error: {:?}", e),
    // }

    YouTube::new(
        hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
        auth,
    )
}

async fn update_recipe_playlists(hub: &YouTube) {
    let playlists = get_all_playlists(&hub).await;

    let mut recipe_playlists: Vec<_> = playlists
        .into_iter()
        .filter(|pl| pl.title.to_ascii_lowercase().contains("recipe"))
        .collect();
    recipe_playlists.sort_by(|a, b| a.id.cmp(&b.id));

    let mut recipes_file = fs::File::create("recipes.json").await.unwrap();

    recipes_file
        .write_all(
            serde_json::to_string_pretty(&recipe_playlists)
                .unwrap()
                .as_bytes(),
        )
        .await
        .unwrap();
}

// https://developers.google.com/youtube/v3/docs/playlists/list
#[instrument(skip(hub))]
async fn get_all_playlists(hub: &YouTube) -> Vec<Playlist> {
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

// https://developers.google.com/youtube/v3/docs/playlistItems/list
#[instrument(skip(hub))]
async fn get_playlist_items(hub: &YouTube, id: &str) {
    let mut first = true;
    let mut next_page_token: Option<String> = None;
    let parts = vec!["snippet".to_string(), "contentDetails".to_string(), "id".to_string()];

    while first || next_page_token.is_some() {
        let mut call = hub.playlist_items().list(&parts).max_results(50).add_id(id);

        if let Some(page_token) = &next_page_token {
            call = call.page_token(page_token);
        }

        let (_, items_resp) = call.doit().await.unwrap();
        next_page_token = items_resp.next_page_token.clone();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub struct Playlist {
    id: String,
    title: String,
    published_at: String,
    item_count: u32,
    etag: String,
}

pub struct Video {
    id: String,
    title: String,
    video_published_at: String,
    start_at: String,
    end_at: String,
    video_id: String,
    note: String,
    published_at: String,
    description: String,

}

pub struct VideoThumbnails {}