use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::Deserialize;

// Define data structures for Reddit API response
#[derive(Deserialize, Debug)]
struct RedditListing {
    data: ListingData,
}

#[derive(Deserialize, Debug)]
struct ListingData {
    children: Vec<PostDataWrapper>,
}

#[derive(Deserialize, Debug)]
struct PostDataWrapper {
    data: PostData,
}

#[derive(Deserialize, Debug)]
struct PostData {
    title: String,
    url: String,
}

// Define a custom error type for better error handling
#[derive(Debug)]
enum RedditError {
    Reqwest(reqwest::Error),
    TokenExtraction,
}

impl From<reqwest::Error> for RedditError {
    fn from(err: reqwest::Error) -> Self {
        RedditError::Reqwest(err)
    }
}

// Function to get access token from Reddit API
async fn get_access_token(client_id: &str, client_secret: &str) -> Result<String, RedditError> {
    let credentials = format!("{}:{}", client_id, client_secret);
    let encoded = general_purpose::STANDARD.encode(credentials);

    let client = Client::new();
    let response = client
        .post("https://www.reddit.com/api/v1/access_token")
        .header("Authorization", format!("Basic {}", encoded))
        .header("User-Agent", "RustRedditApp/0.1 by YourUsername")
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;
    json["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or(RedditError::TokenExtraction)
}

// Function to fetch and print posts from a subreddit
async fn get_subreddit_posts(access_token: &str, subreddit: &str) -> Result<(), RedditError> {
    let client = Client::new();
    let url = format!("https://oauth.reddit.com/r/{}/new?limit=200", subreddit);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "RustRedditApp/0.1 by YourUsername")
        .send()
        .await?;

    let listing: RedditListing = response.json().await?;
    for post in listing.data.children {
        println!("{} - {}", post.data.title, post.data.url);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_id = "3uxvR7iY8Fe0cKRSTI3pdQ";
    let client_secret = "8kPXHK9zFmZLB73MlxqNiN6uLmNIhg";

    let token = get_access_token(client_id, client_secret)
        .await
        .expect("Failed to get access token");
    println!("Access Token: {}", token);

    get_subreddit_posts(&token, "software")
        .await
        .expect("error");
    Ok(())
}
