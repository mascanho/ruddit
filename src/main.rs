use base64::{Engine as _, engine::general_purpose};

use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    arguments::modeling::Args,
    database::adding::{CommentDataWrapper, PostDataWrapper},
    settings::api_keys::AppConfig,
};
use std::process::Command;

pub mod actions;
pub mod ai;
pub mod arguments;
pub mod database;
pub mod exports;
pub mod settings;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RedditPost {
    id: String,
    title: String,
    url: String,
    created_utc: f64,
    subreddit: String,
    permalink: String,
    selftext: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum RedditData {
    Post(RedditPost),
    Comment(RedditComment),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RedditComment {
    id: String,
    body: String,
    author: String,
    created_utc: f64,
    score: i32,
    permalink: String,
    parent_id: String,
    #[serde(default)]
    replies: serde_json::Value,
}

#[derive(Deserialize, Debug, Clone)]
struct RedditListingData {
    children: Vec<RedditListingChild>,
}

#[derive(Deserialize, Debug, Clone)]
struct RedditListingChild {
    data: RedditData,
}

#[derive(Deserialize, Debug, Clone)]
struct RedditListing {
    data: RedditListingData,
}

// Define a custom error type for better error handling
#[derive(Debug)]
#[allow(dead_code)]
enum RedditError {
    Reqwest(reqwest::Error),
    TokenExtraction,
}

impl From<reqwest::Error> for RedditError {
    fn from(e: reqwest::Error) -> Self {
        RedditError::Reqwest(e)
    }
}

pub struct AppState {
    pub data: Vec<PostDataWrapper>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        // Initialize database connection
        let db = database::adding::DB::new()
            .map_err(|_e| RedditError::TokenExtraction)
            .unwrap();

        // Get data from database
        let reddits = db
            .get_db_results()
            .map_err(|_e| RedditError::TokenExtraction)
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
        .filter_map(|child| {
            if let RedditData::Post(post) = &child.data {
                Some(PostDataWrapper {
                    id: post.id.parse().unwrap_or(0),
                    title: post.title.clone(),
                    url: post.url.clone(),
                    timestamp: post.created_utc as i64,
                    formatted_date: database::adding::DB::format_timestamp(post.created_utc as i64)
                        .expect("Failed to format timestamp"),
                    relevance: relevance.to_string(),
                    subreddit: post.subreddit.clone(),
                    permalink: format!("https://reddit.com{}", post.permalink.clone()),
                })
            } else {
                None
            }
        })
        .collect();

    for posts in &posts {
        println!("{:#?}", &posts);
    }

    Ok(posts)
}

async fn get_post_comments(
    access_token: &str,
    post_id: &str,
) -> Result<Vec<RedditListing>, RedditError> {
    let client = Client::new();
    let url = format!("https://oauth.reddit.com/comments/{}", post_id);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "RustRedditApp/0.1 by YourUsername")
        .send()
        .await?;

    let listings: Vec<RedditListing> = response.json().await?;

    if listings.len() < 2 {
        println!("Warning: Unexpected response format");
        return Ok(Vec::new());
    }

    Ok(listings)
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
        .filter_map(|child| {
            if let RedditData::Post(post) = &child.data {
                Some(PostDataWrapper {
                    id: post.id.parse().unwrap_or(0),
                    title: post.title.clone(),
                    url: post.url.clone(),
                    timestamp: post.created_utc as i64,
                    formatted_date: database::adding::DB::format_timestamp(post.created_utc as i64)
                        .expect("Failed to format timestamp"),
                    relevance: relevance.to_string(),
                    subreddit: post.subreddit.clone(),
                    permalink: format!("https://reddit.com{}", post.permalink.clone()),
                })
            } else {
                None
            }
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
    let client_id = api_keys.reddit_api_id;
    let client_secret = api_keys.reddit_api_secret;

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

    // Handle comment fetching
    if let Some(post_id) = args.comments {
        println!("Fetching comments for post {}...", post_id);

        let post_details = get_post_comments(&token, &post_id)
            .await
            .expect("Failed to retrieve comments");

        // Extract post title and subreddit from the first listing
        let post_data = match &post_details[0].data.children[0].data {
            RedditData::Post(post) => post,
            _ => panic!("Expected post data"),
        };
        let post_title = post_data.title.clone();
        let subreddit = post_data.subreddit.clone();

        // Get comments from second listing
        let comments = post_details[1]
            .data
            .children
            .iter()
            .filter_map(|child| {
                if let RedditData::Comment(comment) = &child.data {
                    Some(comment.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Convert to CommentDataWrapper
        let comment_wrappers: Vec<CommentDataWrapper> = comments
            .iter()
            .map(|comment| CommentDataWrapper {
                id: comment.id.clone(),
                post_id: post_id.clone(),
                body: comment.body.clone(),
                author: comment.author.clone(),
                timestamp: comment.created_utc as i64,
                formatted_date: database::adding::DB::format_timestamp(comment.created_utc as i64)
                    .expect("Failed to format timestamp"),
                score: comment.score,
                permalink: comment.permalink.clone(),
                parent_id: comment.parent_id.clone(),
                subreddit: subreddit.clone(),
                post_title: post_title.clone(),
            })
            .collect();

        println!("\nFound {} comments", comment_wrappers.len());

        // Print comments in a readable format
        for (i, comment) in comment_wrappers.iter().enumerate() {
            println!("\nComment #{}", i + 1);
            println!("Subreddit: r/{}", comment.subreddit);
            println!("Post: {}", comment.post_title);
            println!("Author: u/{}", comment.author);
            println!("Score: {} points", comment.score);
            println!("Posted: {}", comment.formatted_date);
            println!("Link: https://reddit.com{}", comment.permalink);
            println!("\nContent:");
            println!("{}\n", comment.body.replace("\\n", "\n").trim());
            println!("{}", "-".repeat(80));
        }

        // Save to database
        let mut db = database::adding::DB::new()?;
        db.create_comments_table()?;
        db.append_comments(&comment_wrappers)?;

        println!("\nComments saved to database!");

        // Export comments to Excel if export flag is set
        if args.export {
            if let Err(e) = exports::excel::export_comments_from_db(&post_id) {
                eprintln!("Failed to export comments to Excel: {}", e);
            } else {
                println!("Comments successfully exported to Excel!");
            }
        }
        return Ok(());
    }

    // Find-Search option
    if let (Some(keyword), Some(relevance)) = (args.find, &args.relevance) {
        let posts = search_subreddit_posts(&token, &keyword, relevance)
            .await
            .expect("Failed to retrieve the posts data");
        let mut db = database::adding::DB::new()?;
        db.create_tables()?;
        db.append_results(&posts)?;
        println!(
            "Successfully appended {} new posts to database",
            posts.len()
        );
        return Ok(());
    }

    // If the user needs to open the settings
    // Run it before all the other logic
    if args.settings {
        settings::api_keys::ConfigDirs::edit_config_file().unwrap();
    }

    // Open database folder if requested
    if args.open_db {
        let base_dirs = directories::BaseDirs::new().expect("Failed to get base directories");
        let db_path = base_dirs.config_dir().join("ruddit");

        #[cfg(target_os = "macos")]
        Command::new("open")
            .arg(&db_path)
            .spawn()
            .expect("Failed to open database folder")
            .wait()?;

        #[cfg(target_os = "windows")]
        Command::new("explorer")
            .arg(&db_path)
            .spawn()
            .expect("Failed to open database folder")
            .wait()?;

        #[cfg(target_os = "linux")]
        Command::new("xdg-open")
            .arg(&db_path)
            .spawn()
            .expect("Failed to open database folder")
            .wait()?;

        println!("Opening database folder: {:?}", db_path);
        return Ok(());
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
        match exports::excel::create_excel() {
            Ok(_) => println!("Successfully exported data to Excel"),
            Err(e) => eprintln!("Failed to export data: {}", e),
        }
    } else if !args.export && !args.clear && !args.leads && !args.settings {
        // Only proceed if at least one argument is provided else use default values
        if args.subreddit.is_none() || args.subreddit.is_some() {
            let subreddit = args.subreddit.unwrap_or_else(|| "supplychain".to_string());
            let relevance = args.relevance.unwrap_or_else(|| "hot".to_string());

            println!(
                "Fetching posts from r/{} ({} posts)...",
                subreddit, relevance
            );

            let posts = get_subreddit_posts(&token, &subreddit, &relevance)
                .await
                .expect("Failed to retrieve the posts data");

            println!("Saving {} posts to database...", posts.len());
            let mut db = database::adding::DB::new()?;
            db.create_tables()?;
            db.append_results(&posts)?;
            println!(
                "Successfully appended {} new posts to database",
                posts.len()
            );
            // Also fetch and save comments for each post
            println!("Fetching comments for posts...");
            for post in &posts {
                if let Ok(post_comments) = get_post_comments(&token, &post.id.to_string()).await {
                    if let Some(post_data) = post_comments.first() {
                        if let RedditData::Post(_post_info) = &post_data.data.children[0].data {
                            let comments = post_comments[1]
                                .data
                                .children
                                .iter()
                                .filter_map(|child| {
                                    if let RedditData::Comment(comment) = &child.data {
                                        Some(CommentDataWrapper {
                                            id: comment.id.clone(),
                                            post_id: post.id.to_string(),
                                            body: comment.body.clone(),
                                            author: comment.author.clone(),
                                            timestamp: comment.created_utc as i64,
                                            formatted_date: database::adding::DB::format_timestamp(
                                                comment.created_utc as i64,
                                            )
                                            .expect("Failed to format timestamp"),
                                            score: comment.score,
                                            permalink: comment.permalink.clone(),
                                            parent_id: comment.parent_id.clone(),
                                            subreddit: post.subreddit.clone(),
                                            post_title: post.title.clone(),
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>();

                            if !comments.is_empty() {
                                db.create_comments_table()?;
                                db.append_comments(&comments)?;
                            }
                        }
                    }
                }
            }

            println!("Done! Posts and comments saved to database.");
        } else {
            println!("No subreddit or relevance specified. Use --help for usage info.");
        }
    } else if args.leads {
        println!("Analyzing posts and comments for leads...");
        match ai::gemini::gemini_generate_leads().await {
            Ok(_) => {
                println!("Lead analysis completed successfully!");
                println!("Results have been exported to Excel in the Reddit_data folder.");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Failed to generate leads: {}", e);
                return Ok(());
            }
        }
    }

    // Clear the database
    if args.clear {
        database::clear::clear_database()?;
    }

    Ok(())
}
