use google_youtube3::YouTube;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

#[tokio::main]
async fn main() {
    let secret = yup_oauth2::read_application_secret("clientsecret.json")
        .await
        .expect("clientsecret.json");

    let mut auth =
        InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
            .persist_tokens_to_disk("tokencache.json")
            .build()
            .await
            .unwrap();

    let scopes = &["https://www.googleapis.com/auth/youtube.readonly"];

    // match auth.token(scopes).await {
    //     Ok(token) => println!("The token is {:?}", token),
    //     Err(e) => println!("error: {:?}", e),
    // }

    let mut hub = YouTube::new(
        hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
        auth,
    );

    let playlists = get_all_playlists(&hub).await;

    let recipe_playlists: Vec<_> = playlists.into_iter().filter(|pl| pl.title.starts_with("recipe")).collect();

    println!("{:?}", recipe_playlists)
}

async fn get_all_playlists(hub: &YouTube) -> Vec<Playlist> {
    let parts = vec!["snippet".to_string(), "contentDetails".to_string()];
    let mut playlists = Vec::new();
    let mut first = true;
    let mut next_page_token: Option<String> = None;

    // https://developers.google.com/youtube/v3/docs/playlists/list

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

#[derive(Debug, Clone)]
pub struct Playlist {
    title: String,
    published_at: String,
    item_count: u32,
    id: String,
    etag: String,
}
