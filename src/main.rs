use google_youtube3::YouTube;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::instrument;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

static RECIPE_PLAYLISTS_FILE: &str = "recipes_playlists.json";
static VIDEOS_DIR: &str = "videos";

lazy_static! {
    static ref WHITESPACE_RE: Regex = Regex::new(r"\s").unwrap();
}

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

#[instrument(skip(hub))]
async fn update_recipe_playlists(hub: &YouTube) {
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
async fn get_playlist_items(hub: &YouTube, id: &str) -> Vec<Video> {
    let mut first = true;
    let mut next_page_token: Option<String> = None;
    let parts = vec![
        "snippet".to_string(),
        "contentDetails".to_string(),
        "id".to_string(),
    ];
    let mut items = Vec::new();

    while first || next_page_token.is_some() {
        first = false;
        let mut call = hub.playlist_items().list(&parts).max_results(50).add_id(id);

        if let Some(page_token) = &next_page_token {
            call = call.page_token(page_token);
        }

        let (_, items_resp) = call.doit().await.unwrap();
        next_page_token = items_resp.next_page_token.clone();
        items.extend(items_resp.items.unwrap().iter().map(|item| {
            let content_details = item.content_details.clone().unwrap();
            let snippet = item.snippet.clone().unwrap();
            let thumbnails = snippet.thumbnails.unwrap();
            Video {
                id: item.id.clone().unwrap(),
                title: snippet.title.unwrap(),
                video_published_at: content_details.video_published_at.unwrap(),
                start_at: content_details.start_at.unwrap(),
                end_at: content_details.end_at.unwrap(),
                video_id: content_details.video_id.unwrap(),
                note: content_details.note.unwrap(),
                published_at: snippet.published_at.unwrap(),
                description: snippet.description.unwrap(),
                thumbnails: Thumbnails::from(&thumbnails),
            }
        }))
    }
    items
}

#[instrument(skip(hub))]
async fn update_playlist_items(hub: &YouTube, id: &str) {
    let videos = get_playlist_items(hub, id).await;
    let dir_path = Path::new(VIDEOS_DIR);
    fs::create_dir_all(dir_path).await.unwrap();

    for video in videos {
        let mut video_file_path = dir_path.to_owned();
        video_file_path.push(format!(
            "{}_{}.json",
            video.id,
            WHITESPACE_RE.replace_all(&video.title, "")
        ));
        let mut video_file = fs::File::create(&video_file_path).await.unwrap();

        video_file.write_all(serde_json::to_string_pretty(&video).unwrap().as_bytes()).await.unwrap();
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

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
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
    thumbnails: Thumbnails,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub struct Thumbnails {
    default: Thumbnail,
    high: Thumbnail,
    maxres: Thumbnail,
    medium: Thumbnail,
    standard: Thumbnail,
}

impl From<&google_youtube3::api::ThumbnailDetails> for Thumbnails {
    fn from(thumnails: &google_youtube3::api::ThumbnailDetails) -> Self {
        Thumbnails {
            default: Thumbnail::from(&thumnails.default.clone().unwrap()),
            high: Thumbnail::from(&thumnails.high.clone().unwrap()),
            maxres: Thumbnail::from(&thumnails.maxres.clone().unwrap()),
            medium: Thumbnail::from(&thumnails.medium.clone().unwrap()),
            standard: Thumbnail::from(&thumnails.standard.clone().unwrap()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub struct Thumbnail {
    height: u32,
    url: String,
    width: u32,
}

impl From<&google_youtube3::api::Thumbnail> for Thumbnail {
    fn from(thumnail: &google_youtube3::api::Thumbnail) -> Self {
        Thumbnail {
            height: thumnail.height.unwrap(),
            url: thumnail.url.clone().unwrap(),
            width: thumnail.width.unwrap(),
        }
    }
}
