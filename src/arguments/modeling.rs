use clap::Parser;

//TODO: implement token input with clap

#[derive(Debug, Parser)]
#[command(name = "Ruddit")]
#[command(version = "0.1")]
#[command(about = "A simple sales CLI", long_about = None)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// API key for authentication
    #[arg(short, long, help = "API key for authentication")]
    pub apikey: Option<String>,

    /// Gemini model key
    #[arg(short, long, help = "Query Gemini to discover insights in your data")]
    pub gemini: Option<String>,

    /// Subreddit name to fetch posts from
    #[arg(short, long, help = "Subreddit name to fetch posts from")]
    pub subreddit: Option<String>,

    /// Relevance filter for search results
    #[arg(short, long, help = "Relevance filter for search results")]
    pub relevance: Option<String>,

    /// Export the results
    #[arg(short, long, help = "Export the results")]
    pub export: bool,

    /// Clear cached data
    #[arg(short, long, help = "Clear cached data")]
    pub clear: bool,

    /// Search for a specific keyword (requires --relevance)
    #[arg(
        short,
        long,
        requires = "relevance",
        help = "Search for a specific keyword (requires --relevance)"
    )]
    pub find: Option<String>,

    /// Export pre-defined data based on arguments and config file
    #[arg(
        short,
        long,
        help = "Export pre-defined data based on arguments and config file"
    )]
    pub leads: bool,

    /// Open the configuration file in the OS default editor
    #[arg(
        short = 'S',
        long,
        help = "Open the configuration file in the OS default editor"
    )]
    pub settings: bool,
}
