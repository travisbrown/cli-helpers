//! Opinionated helpers for building consistent command-line interfaces with [`clap`][clap] and [`simplelog`][simplelog].
//!
//! ## Example
//!
//! The [`prelude`] module exports a minimal subset of these two crates.
//!
//! ```rust,no_run
//! use cli_helpers::prelude::*;
//!
//! #[derive(Debug, Parser)]
//! #[clap(name = "demo", version, author)]
//! struct Opts {
//!     #[clap(flatten)]
//!     verbose: Verbosity,
//! }
//!
//! fn main() -> Result<(), cli_helpers::Error> {
//!     let opts: Opts = Opts::parse();
//!     opts.verbose.init_logging()?;
//!     Ok(())
//! }
//! ```
//!
//! [clap]: https://docs.rs/clap/latest/clap/
//! [simplelog]: https://docs.rs/simplelog/latest/simplelog/

use std::str::FromStr;

use chrono::{DateTime, TimeZone, Utc};
use simplelog::LevelFilter;

const TIMESTAMP_FMT_EN_US: &str = "%a %b %e %I:%M:%S %p %z %Y";
const S_TO_MS_CUTOFF: i64 = 1000000000000;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Logger initialization error")]
    Logger(#[from] log::SetLoggerError),
    #[error("Invalid timestamp format")]
    InvalidTimestamp(String),
}

fn select_log_level_filter(verbosity: u8) -> LevelFilter {
    match verbosity {
        0 => LevelFilter::Off,
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

#[derive(clap::Args, Debug, Clone, PartialEq, Eq)]
pub struct Verbosity {
    /// Level of verbosity
    #[clap(long, short = 'v', global = true, action = clap::ArgAction::Count)]
    verbose: u8,
}

impl Verbosity {
    pub fn new(verbose: u8) -> Self {
        Self { verbose }
    }

    /// Initialize a default terminal logger with the indicated log level.
    pub fn init_logging(&self) -> Result<(), Error> {
        Ok(simplelog::TermLogger::init(
            select_log_level_filter(self.verbose),
            simplelog::Config::default(),
            simplelog::TerminalMode::Stderr,
            simplelog::ColorChoice::Auto,
        )?)
    }
}

/// A timestamp represented as either an epoch second or the `en_US.UTF-8` default on Linux.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(DateTime<Utc>);

impl From<Timestamp> for DateTime<Utc> {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

impl FromStr for Timestamp {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<i64>()
            .ok()
            .and_then(|timestamp_n| {
                if timestamp_n < S_TO_MS_CUTOFF {
                    Utc.timestamp_opt(timestamp_n, 0).single()
                } else {
                    Utc.timestamp_millis_opt(timestamp_n).single()
                }
            })
            .map(Timestamp)
            .or_else(|| {
                DateTime::parse_from_str(&tz_name_to_offset(s), TIMESTAMP_FMT_EN_US)
                    .ok()
                    .map(|timestamp| Timestamp(timestamp.into()))
            })
            .ok_or_else(|| Error::InvalidTimestamp(s.to_string()))
    }
}

/// This is a very simple hack to support copy-paste from `date` for me without pulling in chrono-tz.
fn tz_name_to_offset(input: &str) -> String {
    input.replace("CET", "+0100").replace("CEST", "+0200")
}

pub mod prelude {
    pub use super::{Timestamp, Verbosity};
    pub use ::clap::Parser;
    pub use clap;
    pub mod log {
        pub use log::{error, info, warn, SetLoggerError};
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_prelude() {
        use super::prelude::*;
        use chrono::{TimeZone, Utc};

        #[derive(Debug, Parser, PartialEq, Eq)]
        #[clap(name = "test", version, author)]
        struct Opts {
            #[clap(flatten)]
            verbose: Verbosity,
            #[clap(long)]
            timestamp_a: Timestamp,
            #[clap(long)]
            timestamp_b: Timestamp,
            #[clap(long)]
            timestamp_c: Timestamp,
        }

        let parsed = Opts::try_parse_from([
            "test",
            "-vvvv",
            "--timestamp-a",
            "1692946034",
            "--timestamp-b",
            "Fri Aug 25 08:47:09 AM CEST 2023",
            "--timestamp-c",
            "1692946034632",
        ])
        .unwrap();

        let expected = Opts {
            verbose: Verbosity { verbose: 4 },
            timestamp_a: Timestamp(Utc.timestamp_opt(1692946034, 0).single().unwrap()),
            timestamp_b: Timestamp(Utc.timestamp_opt(1692946029, 0).single().unwrap()),
            timestamp_c: Timestamp(Utc.timestamp_opt(1692946034, 632000000).single().unwrap()),
        };

        assert_eq!(parsed, expected);
    }
}
