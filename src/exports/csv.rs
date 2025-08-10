use std::fs;

use crate::database;
use chrono::Local;
use directories::UserDirs;
use rust_xlsxwriter::{Format, FormatAlign, Workbook, XlsxError};

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
