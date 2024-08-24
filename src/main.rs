mod errors;
mod pattern;
mod tokens;

use crate::errors::*;
use crate::pattern::Pattern;
use clap::{ArgAction, Parser};
use env_logger::Env;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::io::{self, IsTerminal, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Debug, Parser)]
pub struct Args {
    /// Verbose logs (can be used multiple times, maximum: 4)
    #[arg(short, long, action(ArgAction::Count))]
    verbose: u8,
    /// Count total number of permutations instead of printing them
    #[arg(short = 'c', long = "count")]
    count: bool,
    /// Do not print progress bar
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,
    /// Send permutations to stdin of a subprocess
    #[arg(short = 'e', long = "exec")]
    exec: Option<String>,
    pattern: Pattern,
}

pub enum SolveStatus<'a> {
    KnownSolution(&'a [u8]),
    UnknownSolution,
    Unsolved,
}

trait Feedback {
    fn found(&self, password: &[u8]);

    #[inline(always)]
    fn inc(&self) {}

    #[inline(always)]
    fn finish(&self) {}
}

fn display_pw(bytes: &[u8]) -> &str {
    std::str::from_utf8(&bytes[..bytes.len() - 1]).unwrap()
}

fn colored_found_msg(password: &[u8]) -> String {
    format!(
        "\x1b[1m[\x1b[32m+\x1b[0;1m]\x1b[0m found: {:?}",
        display_pw(password)
    )
}

struct Silent {
    colors: bool,
}

impl Feedback for Silent {
    #[inline(always)]
    fn found(&self, password: &[u8]) {
        if self.colors {
            println!("{}", colored_found_msg(password));
        } else {
            println!("[+] found: {:?}", display_pw(password));
        }
    }
}

struct Verbose(ProgressBar);

impl Verbose {
    fn new(total: usize) -> Verbose {
        console::set_colors_enabled(true);

        let pb = ProgressBar::new(total as u64);
        pb.set_draw_target(ProgressDrawTarget::stderr_with_hz(4));
        pb.set_style(ProgressStyle::default_bar()
            .tick_chars(".oO°  °Oo.  ")
            .template(" {spinner:.bold.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("=>-"));
        pb.enable_steady_tick(Duration::from_millis(100));
        Verbose(pb)
    }
}

impl Feedback for Verbose {
    #[inline(always)]
    fn found(&self, password: &[u8]) {
        self.0.println(colored_found_msg(password));
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
    fn write<'a>(&mut self, b: &'a [u8]) -> Result<SolveStatus<'a>>;
}

struct Stdout(io::Stdout);
impl Stdout {
    fn new() -> Stdout {
        Stdout(io::stdout())
    }
}

impl Sink for Stdout {
    #[inline(always)]
    fn write<'a>(&mut self, b: &'a [u8]) -> Result<SolveStatus<'a>> {
        if self.0.write(b).is_err() {
            // we can't reliably tell which password worked based on stdout close
            Ok(SolveStatus::UnknownSolution)
        } else {
            Ok(SolveStatus::Unsolved)
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
    fn write<'a>(&mut self, b: &'a [u8]) -> Result<SolveStatus<'a>> {
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
            Ok(SolveStatus::KnownSolution(b))
        } else {
            Ok(SolveStatus::Unsolved)
        }
    }
}

fn permutate(mut pattern: Pattern, sink: &mut dyn Sink, f: &dyn Feedback) -> Result<()> {
    let mut out = String::new();
    while let Some(out) = pattern.next(&mut out) {
        out.push('\n');
        match sink.write(out.as_bytes())? {
            SolveStatus::KnownSolution(hit) => {
                f.found(hit);
                break;
            }
            SolveStatus::UnknownSolution => break,
            SolveStatus::Unsolved => (),
        }
        out.clear();
        f.inc();
    }

    f.finish();
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let log_level = match args.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    env_logger::init_from_env(Env::default().default_filter_or(log_level));

    if args.count {
        println!("{}", args.pattern.count());
    } else {
        let mut sink: Box<dyn Sink> = if let Some(exec) = args.exec {
            Box::new(Exec::new(&exec)?)
        } else {
            Box::new(Stdout::new())
        };

        let colors = io::stdout().is_terminal();
        let feedback: Box<dyn Feedback> = if args.quiet || !colors {
            Box::new(Silent { colors })
        } else {
            let count = args.pattern.count();
            Box::new(Verbose::new(count))
        };

        permutate(args.pattern, &mut *sink, &*feedback)?;
    }

    Ok(())
}
