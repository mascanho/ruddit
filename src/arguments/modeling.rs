use clap::Parser;

//TODO: implement token input with clap

#[derive(Debug, Parser)]
#[command(name = "Ruddit")]
#[command(version = "0.1")]
#[command(about = "A simple sales CLI", long_about = None)]
#[command(version, about, long_about = None)]
pub struct Args {
    // API KEY
    #[arg(short, long)]
    pub apikey: Option<String>,

    #[arg(short, long)]
    pub gemini: Option<String>,

    #[arg(short, long)]
    pub subreddit: Option<String>,

    #[arg(short, long)]
    pub relevance: Option<String>,

    #[arg(short, long)]
    pub export: bool,

    #[arg(short, long)]
    pub clear: bool,

    // Needs to accept the relevance argument
    #[arg(short, long, requires = "relevance")]
    pub find: Option<String>,
}
