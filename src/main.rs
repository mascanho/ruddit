use base64::{Engine as _, engine::general_purpose};
use chrono::{Duration, Utc};
use clap::Parser;
use reqwest::Client;
use serde::Deserialize;
use std::env::{self};

use crate::{
    arguments::modeling::Args,
    database::adding::PostDataWrapper,
    settings::api_keys::{self, AppConfig},
};

pub mod actions;
pub mod ai;
pub mod arguments;
pub mod database;
pub mod exports;
pub mod settings;

#[derive(Deserialize, Debug)]
struct RedditPost {
    id: String,
    title: String,
    url: String,
    created_utc: f64,
    subreddit: String,
    // Add other fields you need
}

#[derive(Deserialize, Debug)]
struct RedditListingData {
    children: Vec<RedditListingChild>,
    after: Option<String>,
    before: Option<String>,
}

#[derive(Deserialize, Debug)]
struct RedditListingChild {
    kind: String,
    data: RedditPost,
}

#[derive(Deserialize, Debug)]
struct RedditListing {
    kind: String,
    data: RedditListingData,
}

#[derive(Deserialize, Debug)]
struct ListingData {
    children: Vec<PostDataWrapper>,
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

pub struct AppState {
    pub data: Vec<PostDataWrapper>,
}

impl AppState {
    pub fn new() -> Self {
        // Initialize database connection
        let db = database::adding::DB::new()
            .map_err(|e| RedditError::TokenExtraction)
            .unwrap();

        // Get data from database
        let reddits = db
            .get_db_results()
            .map_err(|e| RedditError::TokenExtraction)
            .unwrap();

        let vec = reddits;

        Self { data: vec }
    }
}

// Function to get access token from Reddit API
async fn get_access_token(client_id: String, client_secret: String) -> Result<String, RedditError> {
    let credentials = format!("{}:{}", client_id, client_secret);
    let encoded = general_purpose::STANDARD.encode(credentials);

    let client = Client::new();
    let response = client
        .post("https://www.reddit.com/api/v1/access_token")
        .header("Authorization", format!("Basic {}", encoded))
        .header("User-Agent", "RudditApp/0.1 by Ruddit")
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
async fn get_subreddit_posts(
    access_token: &str,
    subreddit: &str,
    relevance: &str,
) -> Result<Vec<PostDataWrapper>, RedditError> {
    let client = Client::new();
    let url = format!(
        "https://oauth.reddit.com/r/{}/{}?limit=100",
        subreddit, relevance
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "RustRedditApp/0.1 by YourUsername")
        .send()
        .await?;

    let listing: RedditListing = response.json().await?;

    let posts = listing
        .data
        .children
        .into_iter()
        .map(|child| PostDataWrapper {
            id: child.data.id.parse().unwrap_or(0),
            title: child.data.title,
            url: child.data.url,
            timestamp: child.data.created_utc as i64,
            formatted_date: database::adding::DB::format_timestamp(child.data.created_utc as i64)
                .expect("Failed to format timestamp"),
            relevance: relevance.to_string(),
            subreddit: child.data.subreddit,
        })
        .collect();

    for posts in &posts {
        println!("{:#?}", &posts);
    }

    Ok(posts)
}

async fn search_subreddit_posts(
    access_token: &str,
    query: &str,
    relevance: &str,
) -> Result<Vec<PostDataWrapper>, RedditError> {
    let client = Client::new();
    let url = format!(
        "https://oauth.reddit.com/search?q={}&limit=1000&t=all",
        query
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "RustRedditApp/0.1 by YourUsername")
        .send()
        .await?;

    let listing: RedditListing = response.json().await?;

    let posts = listing
        .data
        .children
        .into_iter()
        .map(|child| PostDataWrapper {
            id: child.data.id.parse().unwrap_or(0),
            title: child.data.title,
            url: child.data.url,
            timestamp: child.data.created_utc as i64,
            formatted_date: database::adding::DB::format_timestamp(child.data.created_utc as i64)
                .expect("Failed to format timestamp"),
            relevance: relevance.to_string(),
            subreddit: child.data.subreddit,
        })
        .collect();

    for post in &posts {
        println!("{:#?}", &post);
    }

    Ok(posts)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Config stuff from the settings file
    settings::api_keys::ConfigDirs::create_default_config().unwrap();

    // Read the config
    let config = settings::api_keys::ConfigDirs::read_config().unwrap_or_else(|err| {
        eprintln!("Warning: using default config because: {err}");
        AppConfig::default()
    });

    let api_keys = config.api_keys;
    let client_id = api_keys.REDDIT_API_ID;
    let client_secret = api_keys.REDDIT_API_SECRET;

    // If the user has not set the API keys and app config, prompt them to do so
    let token = match get_access_token(client_id, client_secret).await {
        Ok(t) if !t.is_empty() => t,
        Ok(_) => {
            eprintln!("Received an empty token. Check your API credentials.");
            settings::api_keys::ConfigDirs::edit_config_file()
                .unwrap_or_else(|e| eprintln!("Failed to open config file for editing: {e}"));
            return Ok(());
        }

        Err(e) => {
            eprintln!("Failed to retrieve access token: {:?}", e);
            settings::api_keys::ConfigDirs::edit_config_file()
                .unwrap_or_else(|e| eprintln!("Failed to open config file for editing: {e}"));
            return Ok(());
        }
    };

    // initiate clap / args
    let args = Args::parse();

    // Find-Search option
    if let (Some(keyword), Some(relevance)) = (args.find, &args.relevance) {
        let posts = search_subreddit_posts(&token, &keyword, &relevance)
            .await
            .expect("Failed to retrieve the posts data");
        let mut db = database::adding::DB::new()?;
        db.create_tables()?;
        db.append_results(&posts)?;

        for post in &posts {
            println!("{:#?}", post);
        }
        return Ok(());
    }

    // If the user needs to open the settings
    // Run it before all the other logic
    if args.settings {
        settings::api_keys::ConfigDirs::edit_config_file().unwrap();
    }

    // Query GEMINI
    if let Some(q) = args.gemini {
        match ai::gemini::ask_gemini(&q).await {
            Ok(structured_data) => {
                // Use serde_json to pretty-print the result
                match serde_json::to_string_pretty(&structured_data) {
                    Ok(pretty) => {
                        println!("{}", pretty);
                        return Ok(());
                    }
                    Err(e) => eprintln!("Error pretty-printing JSON: {}", e),
                }
            }
            Err(e) => eprintln!("Error from Gemini API call: {}", e),
        }
        return Ok(());
    }

    if args.export {
        exports::excel::create_excel().expect("Failed to export csv")
    } else if !args.export && !args.clear && !args.leads && !args.settings {
        // Only proceed if at least one argument is provided else use default values
        if args.subreddit.is_none() || args.subreddit.is_some() {
            let subreddit = args.subreddit.unwrap_or_else(|| "supplychain".to_string());
            let relevance = args.relevance.unwrap_or_else(|| "hot".to_string());

            println!(
                "Fetching posts from r/{} ({} posts)...",
                subreddit, relevance
            );

            // NOTE: Improve the search option to search subreddits using clap
            // let _search_posts = search_subreddit_posts(&token, &subreddit)
            //     .await
            //     .expect("Failed to retrieve the posts data");

            let posts = get_subreddit_posts(&token, &subreddit, &relevance)
                .await
                .expect("Failed to retrieve the posts data");

            println!("Saving {} posts to database...", posts.len());
            let mut db = database::adding::DB::new()?;
            db.create_tables()?;
            db.append_results(&posts)?;

            println!("Done!");
        } else {
            println!("No subreddit or relevance specified. Use --help for usage info.");
        }

        // Get the leads and print them
    } else if args.leads {
        let leads = ai::gemini::gemini_generate_leads()
            .await
            .expect("Failed to generate leads");

        match serde_json::to_string_pretty(&leads) {
            Ok(_) => return Ok(()),
            Err(e) => eprintln!("Error pretty-printing JSON: {}", e),
        }
    }

    // Clear the database
    if args.clear {
        database::clear::clear_database()?;
    }

    Ok(())
}
