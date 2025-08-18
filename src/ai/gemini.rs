use anyhow::{Context, Result};
use gemini_rust::{Content, Gemini, Message, Role};
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::exports::excel;
use crate::{database, settings};

// Define GeminiError enum
#[derive(Debug)]
pub enum GeminiError {
    DatabaseError(String),
    ConfigError(String),
    GeminiApiError(String),
    JsonParsingError(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiResponse {
    answer: String,
    url: Option<String>,
    // Add other fields you expect
}

// Implement Display for GeminiError
impl fmt::Display for GeminiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GeminiError::DatabaseError(e) => write!(f, "Database error: {}", e),
            GeminiError::ConfigError(e) => write!(f, "Configuration error: {}", e),
            GeminiError::GeminiApiError(e) => write!(f, "Gemini API error: {}", e),
            GeminiError::JsonParsingError(e) => write!(f, "JSON parsing error: {}", e),
        }
    }
}

// Implement Error trait for GeminiError
impl std::error::Error for GeminiError {}

pub async fn ask_gemini(question: &str) -> Result<Value, GeminiError> {
    // Initialize database connection
    let db = database::adding::DB::new()
        .map_err(|e| GeminiError::DatabaseError(format!("Failed to connect to DB: {}", e)))?;

    // Get data from database
    let reddits = db
        .get_db_results()
        .map_err(|e| GeminiError::DatabaseError(format!("Failed to get DB results: {}", e)))?;

    // Convert data to JSON string
    let json_reddits = serde_json::to_string(&reddits).map_err(|e| {
        GeminiError::DatabaseError(format!("Failed to serialize DB data to JSON: {}", e))
    })?;

    // Get API key from configuration
    let api_key = settings::api_keys::ConfigDirs::read_config()
        .map_err(|e| GeminiError::ConfigError(e.to_string()))?
        .api_keys
        .GEMINI_API_KEY;

    let client = Gemini::new(api_key);

    let mut attempts = 0;
    let max_attempts = 2;
    let mut last_error = None;

    while attempts < max_attempts {
        attempts += 1;

        // Create system prompt - more strict on subsequent attempts
        let system_prompt = if attempts > 1 {
            format!(
                "Given the following data: {}. You are a structured data generator. \
                Your ONLY response should be a VALID JSON object containing the answers and URLs when needed. \
                The JSON must be properly formatted with double quotes for property names and strings. \
                Do NOT include any other text, explanations, or conversational phrases outside the JSON. \
                Do NOT wrap the JSON in markdown code blocks. Output ONLY the raw JSON. \
                Example of acceptable response: {{\"answer\": \"some answer\", \"url\": \"https://example.com\"}}",
                json_reddits
            )
        } else {
            format!(
                "Given the following data: {}. You are a structured data generator. \
                Your ONLY response should be a JSON object containing the answers and URLs when needed. \
                Do not include any other text, explanations, or conversational phrases.",
                json_reddits
            )
        };

        log::debug!("Attempt {} - System prompt: {}", attempts, system_prompt);

        // SPINNER SECTION
        // Create a flag to uontrol the spinner thread
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // Start spinner in a separate thread
        let spinner_handle = thread::spawn(move || {
            let spinner_chars = vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut i = 0;

            while running_clone.load(Ordering::Relaxed) {
                print!("\r{} Thinking... ", spinner_chars[i]);
                std::io::stdout().flush().unwrap();

                i = (i + 1) % spinner_chars.len();
                thread::sleep(Duration::from_millis(100));
            }

            // Clear the spinner line when done
            print!("\r{}", " ".repeat(20));
            print!("\r");
            std::io::stdout().flush().unwrap();
        });

        // Make API request
        let response = match client
            .generate_content()
            .with_system_prompt(&system_prompt)
            .with_user_message(question)
            .execute()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                running.store(false, Ordering::Relaxed);
                spinner_handle.join().unwrap();
                last_error = Some(GeminiError::GeminiApiError(format!(
                    "Failed to generate content: {}",
                    e
                )));
                continue;
            }
        };

        // Stop the spinner
        running.store(false, Ordering::Relaxed);
        spinner_handle.join().unwrap();

        let text_response = response.text();
        log::debug!("Raw Gemini API response: {}", text_response);

        let trimmed_response = text_response.trim();

        // Try to extract JSON from markdown code blocks if present
        let json_str = if trimmed_response.starts_with("```json") {
            trimmed_response
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim()
        } else if trimmed_response.starts_with("```") {
            trimmed_response
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim()
        } else {
            trimmed_response
        };

        log::debug!("Processed JSON string: {}", json_str);

        excel::export_gemini_to_excel(json_str).expect("Failed to export csv");

        // Try to parse the response
        match serde_json::from_str(json_str) {
            Ok(data) => {
                return Ok(data);
            }
            Err(e) => {
                last_error = Some(GeminiError::JsonParsingError(format!(
                    "Failed to parse JSON from API response: {}. Response was: {}",
                    e, text_response
                )));
            }
        }
    }

    Err(last_error.unwrap_or(GeminiError::GeminiApiError(
        "Unknown error after multiple attempts".to_string(),
    )))
}

// PROMPT GEMINI TO SELECTIVELY GET THE DATA BASED ON CONDITIONS
pub async fn gemini_generate_leads() -> Result<(), GeminiError> {
    let settings = settings::api_keys::ConfigDirs::read_config()
        .map_err(|e| GeminiError::ConfigError(e.to_string()))?;

    let question_vec = settings.api_keys.LEAD_KEYWORDS;

    // Get each keyword inside the vector and compose a string to pass to the API
    let question = question_vec
        .iter()
        .map(|q| q.to_string())
        .collect::<Vec<String>>()
        .join(" AND ");

    println!("Question: {}", &question);

    // Initialize database connection
    let db = database::adding::DB::new()
        .map_err(|e| GeminiError::DatabaseError(format!("Failed to connect to DB: {}", e)))?;

    // Get data from database
    let reddits = db
        .get_db_results()
        .map_err(|e| GeminiError::DatabaseError(format!("Failed to get DB results: {}", e)))?;

    // Convert data to JSON string
    let json_reddits = serde_json::to_string(&reddits).map_err(|e| {
        GeminiError::DatabaseError(format!("Failed to serialize DB data to JSON: {}", e))
    })?;

    // Get API key from configuration
    let api_key = settings::api_keys::ConfigDirs::read_config()
        .map_err(|e| GeminiError::ConfigError(e.to_string()))?
        .api_keys
        .GEMINI_API_KEY;

    let client = Gemini::new(api_key);

    let mut attempts = 0;
    let max_attempts = 2;
    let mut last_error = None;

    while attempts < max_attempts {
        attempts += 1;

        // Create system prompt - more strict on subsequent attempts
        let system_prompt = if attempts > 1 {
            format!(
                "Given the following data: {}. You are a structured data generator. \
                Your ONLY response should be a VALID JSON object containing the answers and URLs when needed. \
                The JSON must be properly formatted with double quotes for property names and strings. \
                Do NOT include any other text, explanations, or conversational phrases outside the JSON. \
                Do NOT wrap the JSON in markdown code blocks. Output ONLY the raw JSON. \
                Example of acceptable response: {{\"answer\": \"some answer\", \"url\": \"https://example.com\"}}",
                json_reddits
            )
        } else {
            format!(
                "Given the following data: {}. You are a structured data generator. \
                Your ONLY response should be a JSON object containing the answers and URLs when needed. \
                Do not include any other text, explanations, or conversational phrases.",
                json_reddits
            )
        };

        log::debug!("Attempt {} - System prompt: {}", attempts, system_prompt);

        // SPINNER SECTION
        // Create a flag to uontrol the spinner thread
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // Start spinner in a separate thread
        let spinner_handle = thread::spawn(move || {
            let spinner_chars = vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut i = 0;

            while running_clone.load(Ordering::Relaxed) {
                print!("\r{} Thinking... ", spinner_chars[i]);
                std::io::stdout().flush().unwrap();

                i = (i + 1) % spinner_chars.len();
                thread::sleep(Duration::from_millis(100));
            }

            // Clear the spinner line when done
            print!("\r{}", " ".repeat(20));
            print!("\r");
            std::io::stdout().flush().unwrap();
        });

        // Make API request
        let response = match client
            .generate_content()
            .with_system_prompt(&system_prompt)
            .with_user_message(&question)
            .execute()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                running.store(false, Ordering::Relaxed);
                spinner_handle.join().unwrap();
                last_error = Some(GeminiError::GeminiApiError(format!(
                    "Failed to generate content: {}",
                    e
                )));
                continue;
            }
        };

        // Stop the spinner
        running.store(false, Ordering::Relaxed);
        spinner_handle.join().unwrap();

        let text_response = response.text();
        log::debug!("Raw Gemini API response: {}", text_response);

        let trimmed_response = text_response.trim();

        // Try to extract JSON from markdown code blocks if present
        let json_str = if trimmed_response.starts_with("```json") {
            trimmed_response
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim()
        } else if trimmed_response.starts_with("```") {
            trimmed_response
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim()
        } else {
            trimmed_response
        };

        log::debug!("Processed JSON string: {}", json_str);

        // Try to parse the response
        match serde_json::from_str(json_str) {
            Ok(data) => {
                return Ok(data);
            }
            Err(e) => {
                last_error = Some(GeminiError::JsonParsingError(format!(
                    "Failed to parse JSON from API response: {}. Response was: {}",
                    e, text_response
                )));
            }
        }
    }

    Err(last_error.unwrap_or(GeminiError::GeminiApiError(
        "Unknown error after multiple attempts".to_string(),
    )))
}
