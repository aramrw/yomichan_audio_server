use std::fmt::Display;

use clap::builder::ValueParserFactory;
use clap::{Parser as ClapParser, ValueEnum as ClapValueEnum};
use serde::Serialize;
#[allow(unused_imports)]
use tracing::info;

#[derive(ClapParser, Debug, Clone)]
#[non_exhaustive]
pub struct Cli {
    /// Debug info level.
    #[arg(long, value_enum, default_value = "headless")]
    pub log: CliLog,
    #[arg(long, default_value_t = PortType::default())]
    pub port: PortType,
}

#[derive(ClapValueEnum, Clone, Debug, Default, PartialEq)]
pub enum CliLog {
    #[default]
    Headless,
    // used for checking if a headless instance needs to be spawned or it is the current headless instance
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

#[derive(thiserror::Error, Debug, Serialize)]
#[non_exhaustive]
pub enum PortTypeError {
    // #[error("Invalid port number: {port}")]
    // InvalidPort { port: String },
    // #[error("Failed to parse port: {port}")]
    // ParseNum { port: String },
}

impl PortType {
    // fn new(port: String) -> Self {
    //     PortType { inner: port }
    // }

    // pub fn parse_port(&self) -> Result<Self, PortTypeError> {
    //     let port_range = 0..=65536_u32;
    //
    //     let mut p = self.inner.as_str();
    //     //info!(name: "UNSTRIPPED BYTES", p);
    //     p = p.strip_prefix("localhost:").unwrap_or(p).trim();
    //     //info!(name: "STRIPPED BYTES", p);
    //
    //     let port = p.to_string();
    //     let port_u32 = p.parse::<u32>().map_err(|_| PortTypeError::ParseNum {
    //         port: p.to_string(),
    //     })?;
    //     if port_range.contains(&port_u32) {
    //         Ok(PortType::new(port))
    //     } else {
    //         Err(PortTypeError::InvalidPort { port })
    //     }
    // }
}
