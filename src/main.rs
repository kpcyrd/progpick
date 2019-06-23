#[macro_use]
extern crate failure;

use atty::Stream;
use std::io::{self, Write};
use std::process::{Command, Stdio};
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
    /// Send permutations to subprocess
    #[structopt(short = "e", long = "exec")]
    exec: Option<String>,
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

    #[inline(always)]
    fn inc(&self) {
    }

    #[inline(always)]
    fn finish(&self) {
    }
}

struct Silent;
impl Feedback for Silent {
    #[inline(always)]
    fn new(_total: usize) -> Silent {
        Silent
    }
}

struct Verbose(ProgressBar);
impl Feedback for Verbose {
    #[inline]
    fn new(total: usize) -> Verbose {
        clicolors_control::set_colors_enabled(true);

        let pb = ProgressBar::new(total as u64);
        pb.set_draw_target(ProgressDrawTarget::stderr());
        pb.set_style(ProgressStyle::default_bar()
            .tick_chars(".oO°  °Oo.  ")
            .template(" {spinner:.bold.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .progress_chars("=>-"));
        pb.enable_steady_tick(100);
        pb.set_draw_delta(10_000);
        Verbose(pb)
    }

    #[inline(always)]
    fn inc(&self) {
        self.0.inc(1);
    }

    #[inline(always)]
    fn finish(&self) {
        self.0.finish();
    }
}

trait Sink {
    #[inline(always)]
    fn write(&mut self, b: &[u8]) -> Result<bool>;
}

struct Stdout(io::Stdout);
impl Stdout {
    fn new() -> Stdout {
        Stdout(io::stdout())
    }
}
impl Sink for Stdout {
    #[inline(always)]
    fn write(&mut self, b: &[u8]) -> Result<bool> {
        Ok(self.0.write(b).is_err())
    }
}

struct Exec<'a>(&'a str);
impl<'a> Exec<'a> {
    fn new(exec: &str) -> Exec {
        Exec(exec)
    }
}
impl<'a> Sink for Exec<'a> {
    #[inline(always)]
    fn write(&mut self, b: &[u8]) -> Result<bool> {
        let mut child = Command::new(self.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn child")?;
        let mut stdin = child.stdin.take().unwrap();
        stdin.write(b)
            .context("Failed to write to child")?;
        drop(stdin);

        let exit = child
            .wait()
            .context("Failed to wait for child")?;

        Ok(exit.success())
    }
}

fn permutate<F: Feedback, S: Sink>(mut pattern: Pattern, mut sink: S) -> Result<()> {
    let f = F::new(pattern.count());

    let mut out = String::new();
    while let Some(out) = pattern.next(&mut out) {
        out.push('\n');
        if sink.write(out.as_bytes())? {
            break;
        }
        out.clear();
        f.inc();
    }

    f.finish();
    Ok(())
}

#[inline]
fn dispatch<S: Sink>(pattern: Pattern, sink: S, quiet: bool) -> Result<()> {
    if quiet {
        permutate::<Silent, _>(pattern, sink)
    } else {
        permutate::<Verbose, _>(pattern, sink)
    }
}

fn run() -> Result<()> {
    let args = Args::from_args();

    if args.count {
        println!("{}", args.pattern.count());
    } else {
        if let Some(exec) = args.exec {
            let exec = Exec::new(&exec);
            dispatch(args.pattern, exec, args.quiet)?;
        } else {
            let stdout = Stdout::new();
            dispatch(args.pattern, stdout, args.quiet || atty::is(Stream::Stdout))?;
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
