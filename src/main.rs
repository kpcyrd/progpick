#[macro_use]
extern crate failure;

use atty::Stream;
use std::io::{self, Write};
use structopt::clap::AppSettings;
use structopt::StructOpt;
use indicatif::{ProgressBar, ProgressStyle, ProgressDrawTarget};

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
    /// Do not print progress bar
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,
    pattern: Pattern,
    // -z for null byte seperated
    // -n for dry run/explain(?)
    // -s for start offset(?)
    // ?? for end/range/step(?)

    // state file, write everything to that file, skip everything already in the file
    // this allows resumption
}

trait Feedback {
    fn new(total: usize) -> Self;

    #[inline]
    fn inc(&self) {
    }

    #[inline]
    fn finish(&self) {
    }
}

struct Silent;
impl Feedback for Silent {
    #[inline]
    fn new(_total: usize) -> Silent {
        Silent
    }
}

struct Verbose(ProgressBar);
impl Feedback for Verbose {
    #[inline]
    fn new(total: usize) -> Verbose {
        let pb = ProgressBar::new(total as u64);
        pb.set_draw_target(ProgressDrawTarget::stderr());
        pb.set_style(ProgressStyle::default_bar()
            .tick_chars(".oO°  °Oo.  ")
            .template(" {spinner:.bold.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .progress_chars("=>-"));
        pb.enable_steady_tick(100);
        pb.set_draw_delta(10_000);
        clicolors_control::set_colors_enabled(true);
        Verbose(pb)
    }

    #[inline]
    fn inc(&self) {
        self.0.inc(1);
    }

    #[inline]
    fn finish(&self) {
        self.0.finish();
    }
}

fn permutate<F: Feedback>(mut pattern: Pattern) {
    let f = F::new(pattern.count());

    let mut stdout = io::stdout();
    let mut out = String::new();
    while let Some(out) = pattern.next(&mut out) {
        out.push('\n');
        if stdout.write(out.as_bytes()).is_err() {
            break;
        }
        out.clear();
        f.inc();
    }

    f.finish();
}

fn run() -> Result<()> {
    let args = Args::from_args();

    if args.count {
        println!("{}", args.pattern.count());
    } else if args.quiet || atty::is(Stream::Stdout) {
        permutate::<Silent>(args.pattern);
    } else {
        permutate::<Verbose>(args.pattern);
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
