use std::fmt::Display;
use std::path::PathBuf;

use clap::builder::ValueParserFactory;
use clap::{Parser as ClapParser, ValueEnum as ClapValueEnum};
use serde::Serialize;
#[allow(unused_imports)]
use tracing::info;

#[derive(ClapParser, Debug, Clone)]
#[non_exhaustive]
pub struct Cli {
    /// Level to display debug information
    #[arg(long, value_enum, default_value = "headless")]
    pub log: CliLog,
    /// The port to run the server on. Ex: --port 8080
    #[arg(long, default_value_t = PortType::default())]
    pub port: PortType,
    /// The path to the audio folder. defaults to the yas executable path
    #[arg(short, long, default_value = "./audio")]
    pub audio: PathBuf,
    /// Prints the available sources. Can be used to sort with "./sort.txt" file
    #[arg(long)]
    pub sources: bool,
}

#[derive(ClapValueEnum, Clone, Debug, Default, PartialEq)]
pub enum CliLog {
    #[default]
    Headless,
    // used for checking if a headless instance needs to be spawned
    // or it is the current headless instance
    HeadlessInstance,
    Dev,
    Full,
}

impl ValueParserFactory for PortType {
    type Parser = PortTypeValueParser;
    fn value_parser() -> Self::Parser {
        PortTypeValueParser
    }
}

#[derive(Clone, Debug)]
pub struct PortTypeValueParser;
impl clap::builder::TypedValueParser for PortTypeValueParser {
    type Value = PortType;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value_str = value.to_string_lossy();

        let strip = value_str
            .strip_prefix("localhost:")
            .unwrap_or(&value_str)
            .trim();
        let port_u32 = strip.parse::<u32>().map_err(|_| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                format!("Invalid port value: {}", value_str),
            )
        })?;

        if (0..=65535).contains(&port_u32) {
            Ok(PortType {
                inner: format!("localhost:{strip}"),
            })
        } else {
            Err(clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                format!("Port out of range: {}", value_str),
            ))
        }
    }
}

#[derive(Clone, Debug)]
pub struct PortType {
    pub inner: String,
}

impl Default for PortType {
    fn default() -> Self {
        Self {
            inner: "localhost:8080".to_string(),
        }
    }
}

impl Display for PortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let port = &self.inner;
        write!(f, "{port}")
    }
}
