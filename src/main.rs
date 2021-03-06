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
    /// Send permutations to a subprocess (arguments are supported but shell syntax is not)
    #[structopt(short = "e", long = "exec")]
    exec: Option<String>,
    pattern: Pattern,
}

pub enum Match<'a> {
    KnownMatch(&'a [u8]),
    UnknownMatch,
    None,
}

trait Feedback {
    fn new(total: usize) -> Self;

    #[inline(always)]
    fn found(&self, password: &[u8]);

    #[inline(always)]
    fn inc(&self) {
    }

    #[inline(always)]
    fn finish(&self) {
    }
}

fn display_pw(bytes: &[u8]) -> &str {
    std::str::from_utf8(&bytes[..bytes.len()-1]).unwrap()
}

struct Silent;
impl Feedback for Silent {
    #[inline(always)]
    fn new(_total: usize) -> Silent {
        Silent
    }

    #[inline(always)]
    fn found(&self, password: &[u8]) {
        println!("[+] found: {:?}", display_pw(password));
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
    fn found(&self, password: &[u8]) {
        self.0.println(format!("\x1b[1m[\x1b[32m+\x1b[0;1m]\x1b[0m found: {:?}", display_pw(password)));
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
    fn write<'a>(&mut self, b: &'a [u8]) -> Result<Match<'a>>;
}

struct Stdout(io::Stdout);
impl Stdout {
    fn new() -> Stdout {
        Stdout(io::stdout())
    }
}
impl Sink for Stdout {
    #[inline(always)]
    fn write<'a>(&mut self, b: &'a[u8]) -> Result<Match<'a>> {
        if self.0.write(b).is_err() {
            // we can't reliably tell which password worked
            Ok(Match::UnknownMatch)
        } else {
            Ok(Match::None)
        }
    }
}

struct Exec {
    bin: String,
    args: Vec<String>,
}

impl Exec {
    fn new(cmd: &str) -> Result<Exec> {
        let mut cmd = shellwords::split(cmd)
            .map_err(|_| format_err!("Mismatched quotes in cmd"))?;
        if cmd.len() < 1 {
            bail!("cmd argument can't be empty");
        }
        let bin = cmd.remove(0);
        Ok(Exec {
            bin,
            args: cmd,
        })
    }
}

impl Sink for Exec {
    #[inline(always)]
    fn write<'a>(&mut self, b: &'a [u8]) -> Result<Match<'a>> {
        let mut child = Command::new(&self.bin)
            .args(&self.args)
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

        if exit.success() {
            // we know the password
            Ok(Match::KnownMatch(b))
        } else {
            Ok(Match::None)
        }
    }
}

fn permutate<F: Feedback, S: Sink>(mut pattern: Pattern, mut sink: S) -> Result<()> {
    let f = F::new(pattern.count());

    let mut out = String::new();
    while let Some(out) = pattern.next(&mut out) {
        out.push('\n');
        match sink.write(out.as_bytes())? {
            Match::KnownMatch(hit) => {
                f.found(hit);
                break;
            },
            Match::UnknownMatch => break,
            Match::None => (),
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
            let exec = Exec::new(&exec)?;
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
