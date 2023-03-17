mod cli_args;

use tracing::{dispatcher::set_global_default, Level};
use tracing_subscriber::FmtSubscriber;
use cli_args::Args;
use clap::Parser;

fn main () {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish(); //build a console output formatter that only outputs if the level is >= INFO
    set_global_default(subscriber.into()).expect("setting default subscriber failed"); //set the global subscriber to be that subscriber

    let a = Args::parse();
    println!("{a:?}");
}