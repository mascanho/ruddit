use std::fmt::format;

use gemini_rust::{Content, Gemini, Message, Role};

use crate::{database, settings};
pub async fn ask_gemini(question: &str) -> String {
    let db = database::adding::DB::new().unwrap();

    let reddits = db.get_db_results().unwrap();

    let json_reddits = serde_json::to_string(&reddits).unwrap();

    let api_key = settings::api_keys::ConfigDirs::read_config()
        .unwrap()
        .api_keys
        .GEMINI_API_KEY;

    let client = Gemini::new(api_key);

    let system_prompt = format!(
        "Given the following data {:#?}, output the answers into json format when URLs are needed",
        &json_reddits,
    );

    let response = client
        .generate_content()
        .with_system_prompt(system_prompt)
        .with_user_message(question)
        .execute()
        .await
        .expect("Failed to generate content with GEMINI");

    println!("");
    println!("🤖 Response: {:#?}", &response.text());

    response.text().to_string()
}
