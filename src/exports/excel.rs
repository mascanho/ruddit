use std::fs;

use crate::{
    ai::gemini::GeminiResponse,
    database::{self, adding::PostDataWrapper},
};
use chrono::Local;
use directories::UserDirs;
use rust_xlsxwriter::{Format, FormatAlign, Workbook, XlsxError};
use serde_json::Value;

pub fn create_excel() -> Result<(), Box<dyn std::error::Error>> {
    // Get data from database with proper error handling
    let db = database::adding::DB::new()?;
    let data = db.get_db_results()?;

    let user_dirs = UserDirs::new().ok_or("Failed to get user directories")?;
    let desktop = user_dirs
        .desktop_dir()
        .ok_or("Failed to get desktop directory")?;

    println!("Exporting {} records to Excel", data.len());

    // Create new workbook
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Reddit Posts")?;

    // Create header format
    let header_format = Format::new().set_align(FormatAlign::Center).set_bold();

    // Write headers
    let headers = [
        // "Timestamp",
        "Date",
        "Title",
        "URL",
        "Relevance",
        "Subreddit",
    ];

    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col as u16, *header, &header_format)?;
    }

    // Write data rows
    for (row, result) in data.iter().enumerate() {
        let row_num = (row + 1) as u32;
        let cells = [
            // result.timestamp.to_string(),
            result.formatted_date.clone(),
            result.title.clone(),
            result.url.clone(),
            result.relevance.clone(),
            result.subreddit.clone(),
        ];

        for (col, cell) in cells.iter().enumerate() {
            worksheet.write_string(row_num, col as u16, cell)?;
        }
    }

    // Auto-fit columns for better readability
    worksheet.autofit();

    // Save to file with timestamp
    let filename = format!(
        "Reddit_data_{}.xlsx",
        Local::now().format("%d-%m-%Y_%H-%M-%S")
    );

    let folder_name = "Reddit_data";
    let folder_path = desktop.join(folder_name);

    if let Err(e) = fs::create_dir_all(&folder_path) {
        eprintln!("Failed to create directory: {}", e);
        return Err(e.into());
    }

    workbook.save(folder_path.join(filename.as_str()))?;

    println!("Successfully exported to {:?}", desktop.join(folder_name));
    Ok(())
}

// Export the filtered data by the LLM into a .xlsx
pub fn export_gemini_to_excel(gemini_data: &str) -> Result<(), XlsxError> {
    // convert json string to vector of PostDataWrapper

    let gemini_data: Value = serde_json::from_str(gemini_data).unwrap();

    println!("Exporting {:#?} records to Excel", gemini_data);

    let user_dirs = UserDirs::new()
        .ok_or("Failed to get user directories")
        .expect("Failed to get direcotry to save");

    let desktop = user_dirs
        .desktop_dir()
        .ok_or("Failed to get desktop directory")
        .expect("Failed to get desktop directory");

    //println!("Exporting {} records to Excel", gemini_data.len());

    // Create new workbook
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Reddit Posts")?;

    // Create header format
    let header_format = Format::new().set_align(FormatAlign::Center).set_bold();

    // Write headers
    let headers = [
        // "Timestamp",
        "Date",
        "Title",
        "URL",
        "Relevance",
        "Subreddit",
    ];

    for (col, header) in headers.iter().enumerate() {
        worksheet.write_string_with_format(0, col as u16, *header, &header_format)?;
    }

    // Write data rows
    //for (row, result) in gemini_data.lines().enumerate() {
    //    let row_num = (row + 1) as u32;
    //    let cells = [
    //        // result.timestamp.to_string(),
    //        result.formatted_date.clone(),
    //        result.title.clone(),
    //        result.url.clone(),
    //        result.relevance.clone(),
    //        result.subreddit.clone(),
    //    ];
    //
    //    for (col, cell) in cells.iter().enumerate() {
    //        worksheet.write_string(row_num, col as u16, cell)?;
    //    }
    //}

    // Auto-fit columns for better readability
    worksheet.autofit();

    // Save to file with timestamp
    let filename = format!(
        "Reddit_data_{}.xlsx",
        Local::now().format("%d-%m-%Y_%H-%M-%S")
    );

    let folder_name = "Reddit_data";
    let folder_path = desktop.join(folder_name);

    if let Err(e) = fs::create_dir_all(&folder_path) {
        eprintln!("Failed to create directory: {}", e);
        return Err(e.into());
    }

    workbook.save(folder_path.join(filename.as_str()))?;

    Ok(())
}
