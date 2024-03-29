use atty::Stream;
use clap::Parser;
use env_logger::Env;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::io::{self, Write};
use std::process::{Command, Stdio};

mod errors;
use crate::errors::*;
mod pattern;
use crate::pattern::Pattern;
mod tokens;

#[derive(Debug, Parser)]
pub struct Args {
    /// Verbose logs (can be used multiple times, maximum: 4)
    #[clap(short, long, parse(from_occurrences))]
    verbose: u8,
    /// Count total number of permutations instead of printing them
    #[clap(short = 'c', long = "count")]
    count: bool,
    /// Do not print progress bar
    #[clap(short = 'q', long = "quiet")]
    quiet: bool,
    /// Send permutations to stdin of a subprocess
    #[clap(short = 'e', long = "exec")]
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

    fn found(&self, password: &[u8]);

    #[inline(always)]
    fn inc(&self) {}

    #[inline(always)]
    fn finish(&self) {}
}

fn display_pw(bytes: &[u8]) -> &str {
    std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap()
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
        console::set_colors_enabled(true);

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
        self.0.println(format!(
            "\x1b[1m[\x1b[32m+\x1b[0;1m]\x1b[0m found: {:?}",
            display_pw(password)
        ));
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
    fn write<'a>(&mut self, b: &'a [u8]) -> Result<Match<'a>> {
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
        let mut cmd = shellwords::split(cmd).map_err(|_| anyhow!("Mismatched quotes in cmd"))?;
        if cmd.is_empty() {
            bail!("cmd argument can't be empty");
        }
        let bin = cmd.remove(0);
        Ok(Exec { bin, args: cmd })
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
        stdin.write(b).context("Failed to write to child")?;
        drop(stdin);

        let exit = child.wait().context("Failed to wait for child")?;

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
            }
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

fn main() -> Result<()> {
    let args = Args::from_args();

    let log_level = match args.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    env_logger::init_from_env(Env::default().default_filter_or(log_level));

    if args.count {
        println!("{}", args.pattern.count());
    } else if let Some(exec) = args.exec {
        let exec = Exec::new(&exec)?;
        dispatch(args.pattern, exec, args.quiet)?;
    } else {
        let stdout = Stdout::new();
        dispatch(args.pattern, stdout, args.quiet || atty::is(Stream::Stdout))?;
    }

    Ok(())
}
