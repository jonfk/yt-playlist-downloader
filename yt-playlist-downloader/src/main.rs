use clap::Clap;
use google_youtube3::YouTube;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{info, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

static RECIPE_PLAYLISTS_FILE: &str = "recipes_playlists.json";
static VIDEOS_DIR: &str = "videos";
static THUMBNAILS_DIR: &str = "video_thumbnails";

lazy_static! {
    static ref WHITESPACE_RE: Regex = Regex::new(r"\s").unwrap();
}

mod playlist;
mod video;
mod channel;

#[derive(Clap)]
#[clap(version = "1.0", author = "Jonathan Fok kan <jfokkan@gmail.com>")]
struct CliOpts {
    #[clap(long)]
    update_playlists: bool,

    #[clap(long)]
    update_videos: bool,

    #[clap(long)]
    update_channels: bool,

    #[clap(long)]
    check_video: bool,

    #[clap(long)]
    all: bool,
}

#[tokio::main]
async fn main() {
    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(EnvFilter::from_default_env())
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cli_opts = CliOpts::parse();

    let hub = build_yt_api().await;

    let playlists = if cli_opts.all || cli_opts.update_playlists {
        playlist::update_recipe_playlists(&hub).await
    } else {
        playlist::read_recipe_playlists().await
    };

    let videos = if cli_opts.all || cli_opts.update_videos {
        video::update_all_playlists_items(&hub, playlists.into_iter().map(|p| p.id).collect()).await
    } else {
        video::read_all_videos().await
    };

    if cli_opts.all || cli_opts.update_channels {

    }

    if cli_opts.all || cli_opts.check_video {
        info!("TODO check videos");
    }

    info!("Update Completed");
}

async fn build_yt_api() -> YouTube {
    let secret = yup_oauth2::read_application_secret("clientsecret.json")
        .await
        .expect("clientsecret.json");

    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("tokencache.json")
        .build()
        .await
        .unwrap();

    // let _scopes = &["https://www.googleapis.com/auth/youtube.readonly"];

    // match auth.token(scopes).await {
    //     Ok(token) => println!("The token is {:?}", token),
    //     Err(e) => println!("error: {:?}", e),
    // }

    YouTube::new(
        hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
        auth,
    )
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
    title: String,
    video_published_at: Option<String>,
    start_at: Option<String>,
    end_at: Option<String>,
    video_id: String,
    published_at: String,
    description: String,
    thumbnails: Thumbnails,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub struct Thumbnails {
    default: Thumbnail,
    medium: Thumbnail,
    high: Thumbnail,
    standard: Option<Thumbnail>,
    maxres: Option<Thumbnail>,
}

impl From<&google_youtube3::api::ThumbnailDetails> for Thumbnails {
    fn from(thumnails: &google_youtube3::api::ThumbnailDetails) -> Self {
        Thumbnails {
            default: Thumbnail::from(&thumnails.default.clone().unwrap()),
            medium: Thumbnail::from(&thumnails.medium.clone().unwrap()),
            high: Thumbnail::from(&thumnails.high.clone().unwrap()),
            standard: thumnails.standard.clone().map(|t| Thumbnail::from(&t)),
            maxres: thumnails.maxres.clone().map(|t| Thumbnail::from(&t)),
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
