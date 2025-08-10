use clap::Parser;

//TODO: implement token input with clap

#[derive(Parser, Debug)]
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
}
