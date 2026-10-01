use armorer::{
    Error, Result,
    config::{Config, Lock},
    plan::{Plan, inspect},
};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;
use std::{io::Write, path::PathBuf};

#[derive(Parser)]
#[command(version, about = "Inspect Rust repositories and preview secure setup")]
struct Cli {
    #[arg(long, default_value = ".", global = true)]
    repository: PathBuf,
    #[command(subcommand)]
    command: Operation,
}

#[derive(Subcommand)]
enum Operation {
    /// Validate selections and report unmet setup requirements (JSON; exit 2 until ready).
    Check,
    /// Preview changes without writing repository files (JSON).
    Plan,
    /// Print a structural JSON schema. Semantic rules are also checked at runtime.
    Schema {
        #[arg(value_enum)]
        kind: SchemaKind,
    },
}

#[derive(Clone, ValueEnum)]
enum SchemaKind {
    Config,
    Lock,
    Plan,
}

/// Produce JSON and an exit status for the selected operation.
///
/// Schema and plan operations return status 0; check returns 2 because release
/// readiness remains blocked. Propagates inspection errors and maps JSON value
/// conversion failures to `Error::Json`; output is left to the caller.
fn run(cli: Cli) -> Result<(serde_json::Value, i32)> {
    match cli.command {
        Operation::Schema { kind } => {
            let schema = match kind {
                SchemaKind::Config => schemars::schema_for!(Config),
                SchemaKind::Lock => schemars::schema_for!(Lock),
                SchemaKind::Plan => schemars::schema_for!(Plan),
            };
            Ok((serde_json::to_value(schema).map_err(|_| Error::Json)?, 0))
        }
        Operation::Plan => Ok((
            serde_json::to_value(inspect(&cli.repository, "plan")?).map_err(|_| Error::Json)?,
            0,
        )),
        Operation::Check => Ok((
            serde_json::to_value(inspect(&cli.repository, "check")?).map_err(|_| Error::Json)?,
            2,
        )),
    }
}

/// Parse CLI arguments, write pretty JSON to stdout, and exit with the operation's status.
///
/// Operation errors become JSON errors with status 1. Serialization or stdout
/// write failures also exit with status 1; argument parsing is handled by clap.
fn main() {
    let (value, code) = match run(Cli::parse()) {
        Ok(result) => result,
        Err(error) => (
            json!({"error": {"code": error.code(), "message": error.to_string()}}),
            1,
        ),
    };
    let output = serde_json::to_vec_pretty(&value);
    let mut stdout = std::io::stdout().lock();
    match output {
        Ok(bytes)
            if stdout
                .write_all(&bytes)
                .and_then(|_| stdout.write_all(b"\n"))
                .is_ok() =>
        {
            std::process::exit(code)
        }
        _ => std::process::exit(1),
    }
}
