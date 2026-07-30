mod command;
mod exit_code;
mod output;
mod parser;
mod paths;

use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

use command::Command;
use parser::CommandParser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    command: Command,
}

impl Cli {
    pub fn parse() -> Self {
        Self::parse_from(env::args_os())
    }

    pub fn parse_from<I>(args: I) -> Self
    where
        I: IntoIterator<Item = OsString>,
    {
        Self {
            command: CommandParser::new(args).parse(),
        }
    }

    pub fn execute(self) -> ExitCode {
        self.command.execute().into()
    }
}
