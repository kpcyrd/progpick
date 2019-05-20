#[macro_use]
extern crate failure;

use std::io::{self, Write};
use structopt::clap::AppSettings;
use structopt::StructOpt;

mod errors;
use crate::errors::*;
mod pattern;
use crate::pattern::Pattern;
mod tokens;

#[derive(Debug, StructOpt)]
#[structopt(raw(global_settings = "&[AppSettings::ColoredHelp]"))]
pub struct Args {
    /// Count options instead of printing them
    #[structopt(short = "c", long = "count")]
    count: bool,
    pattern: Pattern,
    // -z for null byte seperated
    // -n for dry run/explain(?)
    // -s for start offset(?)
    // ?? for end/range/step(?)

    // state file, write everything to that file, skip everything already in the file
    // this allows resumption
}

fn run() -> Result<()> {
    let mut args = Args::from_args();

    if args.count {
        println!("{}", args.pattern.count());
    } else {
        let mut stdout = io::stdout();

        let mut out = String::new();
        while let Some(out) = args.pattern.next(&mut out) {
            out.push('\n');
            if stdout.write(out.as_bytes()).is_err() {
                break;
            }
            out.clear();
        }
    }

    Ok(())
}

fn main() {
    env_logger::init();

    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        for cause in err.iter_chain().skip(1) {
            eprintln!("Because: {}", cause);
        }
        std::process::exit(1);
    }
}
