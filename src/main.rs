
mod codegen;
mod jit;

mod frontend;
mod compile;
mod errors;

use std::error::Error;

use std::fs::File;
use std::io::{BufReader, BufRead};

use clap::Parser;
use compile::compile_lines;
use errors::{LocalizedSourcedError, LocalizableError};
use frontend::tokenizer::Location;

/// LOL
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to the file to read
    #[arg(short, long)]
    path: std::path::PathBuf,
}

fn run(args: Args) -> Result<(), Box<dyn Error>> {
    
    let file = File::open(args.path.clone())
        .map_err(|err| err
            .with_location(Location::default())
            .with_source(args.path.clone()))?;

    let lines = BufReader::new(file)
        .lines()
        .map(Result::unwrap);

    compile_lines(lines)
        .map_err(|err| err.with_source(args.path))?;

    Ok(())
}


fn main() {
    let args = Args::parse();

    if let Err(e) = run(args) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
