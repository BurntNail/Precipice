use clap::Parser;

use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    binary: PathBuf,
    #[arg(short, long)]
    cli_args: Vec<String>,
    #[arg(short, long, default_value_t = 1000)]
    runs: usize,
    #[arg(short, long, default_value_t = false)]
    show_output_in_console: bool,
}