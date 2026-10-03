use armorer::{
    Error, Result,
    config::{Config, Lock},
    plan::{Plan, inspect},
};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;
use std::{io::Write, path::PathBuf};

#[derive(Parser)]
#[command(
    version,
    about = "Inspect Rust repositories and apply reviewed local setup"
)]
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
    /// Print the authenticated embedded bootstrap tool/workflow catalog (no downloads).
    Catalog,

    /// Explicit multi-file provisioning with a separately versioned approval contract.
    Bootstrap {
        #[command(subcommand)]
        operation: BootstrapOperation,
    },

    /// Review and apply upgrades/imports/rollback using separate approval contracts.
    Upgrade {
        #[command(subcommand)]
        operation: UpgradeOperation,
    },

    /// Preview changes without writing repository files (JSON).
    Plan,
    /// Apply a reviewed plan; the digest must be supplied independently.
    Apply {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        expect_plan_sha256: String,
    },
    /// Explicitly roll back or finalize a matching interrupted transaction.
    Recover {
        #[arg(long)]
        expect_plan_sha256: String,
    },
    /// Print a structural JSON schema. Semantic rules are also checked at runtime.
    Schema {
        #[arg(value_enum)]
        kind: SchemaKind,
    },
}

#[derive(Subcommand)]
enum BootstrapOperation {
    /// Preview exact bootstrap files using explicit owner-reviewed policy (read only).
    Plan {
        #[arg(long)]
        policy: PathBuf,
    },
    /// Inspect the same configuration without writing (exit 2; release gates remain open).
    Check {
        #[arg(long)]
        policy: PathBuf,
    },
    /// Apply a reviewed version-two plan and independently supplied digest.
    Apply {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        expect_plan_sha256: String,
    },
    /// Recover only the exact matching version-two journal.
    Recover {
        #[arg(long)]
        expect_plan_sha256: String,
    },
}

#[derive(Subcommand)]
enum UpgradeOperation {
    /// Show exact generated base, current/candidate bytes, pin and compatibility changes.
    Plan {
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        catalog: String,
        #[arg(long)]
        import_file: Vec<String>,
        #[arg(long)]
        allow_downgrade: bool,
    },
    /// Apply only a separately reviewed upgrade plan and independent digest.
    Apply {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        expect_plan_sha256: String,
    },
    /// Explicitly recover the exact upgrade or reviewed rollback journal.
    Recover {
        #[arg(long)]
        expect_plan_sha256: String,
    },
    /// Preview exact prior-byte restoration; requires explicit historical downgrade intent.
    RollbackPlan {
        #[arg(long)]
        upgrade_plan: PathBuf,
        #[arg(long)]
        expect_upgrade_plan_sha256: String,
        #[arg(long)]
        allow_downgrade: bool,
    },
    /// Restore a separately reviewed reverse packet; release readiness remains unverified.
    RollbackApply {
        #[arg(long)]
        plan: PathBuf,
        #[arg(long)]
        expect_plan_sha256: String,
    },
}

#[derive(Clone, ValueEnum)]
enum SchemaKind {
    UpgradePlan,
    UpgradeRollback,
    BootstrapPlan,
    CiPolicy,
    Config,
    Lock,
    Plan,
    ReleaseInventory,
    VerificationPolicy,
    ArtifactEvidence,
    EvidenceRequirements,
    CapabilityConfig,
    Catalog,
    CapabilityObservation,
    LifecycleRecord,
    GithubReceipt,
    PublishSet,
    RegistryReceipt,
}

/// Produce JSON and an exit status for the selected operation.
///
/// Schema, plan, apply and recovery return status 0; check returns 2 because release
/// readiness remains blocked. Propagates inspection errors and maps JSON value
/// conversion failures to `Error::Json`; output is left to the caller.
fn run(cli: Cli) -> Result<(serde_json::Value, i32)> {
    match cli.command {
        Operation::Upgrade { operation } => {
            let value = match operation {
                UpgradeOperation::Plan {
                    policy,
                    catalog,
                    import_file,
                    allow_downgrade,
                } => serde_json::to_value(armorer::upgrade::inspect(
                    &cli.repository,
                    &policy,
                    &catalog,
                    &import_file,
                    allow_downgrade,
                )?),
                UpgradeOperation::Apply {
                    plan,
                    expect_plan_sha256,
                } => {
                    let plan = armorer::upgrade::load_plan(&plan, &expect_plan_sha256)?;
                    serde_json::to_value(armorer::upgrade::apply(
                        &cli.repository,
                        &plan,
                        &expect_plan_sha256,
                    )?)
                }
                UpgradeOperation::Recover { expect_plan_sha256 } => serde_json::to_value(
                    armorer::upgrade::recover(&cli.repository, &expect_plan_sha256)?,
                ),
                UpgradeOperation::RollbackPlan {
                    upgrade_plan,
                    expect_upgrade_plan_sha256,
                    allow_downgrade,
                } => {
                    let plan =
                        armorer::upgrade::load_plan(&upgrade_plan, &expect_upgrade_plan_sha256)?;
                    serde_json::to_value(armorer::upgrade::inspect_rollback(
                        &cli.repository,
                        &plan,
                        &expect_upgrade_plan_sha256,
                        allow_downgrade,
                    )?)
                }
                UpgradeOperation::RollbackApply {
                    plan,
                    expect_plan_sha256,
                } => {
                    let plan = armorer::upgrade::load_rollback_plan(&plan, &expect_plan_sha256)?;
                    serde_json::to_value(armorer::upgrade::apply_rollback(
                        &cli.repository,
                        &plan,
                        &expect_plan_sha256,
                    )?)
                }
            };
            Ok((value.map_err(|_| Error::Json)?, 0))
        }
        Operation::Bootstrap { operation } => {
            let (value, status) = match operation {
                BootstrapOperation::Plan { policy } => (
                    serde_json::to_value(armorer::bootstrap::inspect(&cli.repository, &policy)?),
                    0,
                ),
                BootstrapOperation::Check { policy } => (
                    serde_json::to_value(armorer::bootstrap::inspect(&cli.repository, &policy)?),
                    2,
                ),
                BootstrapOperation::Apply {
                    plan,
                    expect_plan_sha256,
                } => {
                    let plan = armorer::bootstrap::load_plan(&plan, &expect_plan_sha256)?;
                    (
                        serde_json::to_value(armorer::bootstrap::apply(
                            &cli.repository,
                            &plan,
                            &expect_plan_sha256,
                        )?),
                        0,
                    )
                }
                BootstrapOperation::Recover { expect_plan_sha256 } => (
                    serde_json::to_value(armorer::bootstrap::recover(
                        &cli.repository,
                        &expect_plan_sha256,
                    )?),
                    0,
                ),
            };
            Ok((value.map_err(|_| Error::Json)?, status))
        }
        Operation::Catalog => Ok((
            serde_json::to_value(armorer::catalog::reviewed()?).map_err(|_| Error::Json)?,
            0,
        )),
        Operation::Schema { kind } => {
            let schema = match kind {
                SchemaKind::UpgradeRollback => {
                    schemars::schema_for!(armorer::upgrade::RollbackPlan)
                }
                SchemaKind::UpgradePlan => schemars::schema_for!(armorer::upgrade::Plan),
                SchemaKind::BootstrapPlan => schemars::schema_for!(armorer::bootstrap::Plan),
                SchemaKind::CiPolicy => schemars::schema_for!(armorer::ci_policy::CiPolicy),
                SchemaKind::Config => schemars::schema_for!(Config),
                SchemaKind::Lock => schemars::schema_for!(Lock),
                SchemaKind::Plan => schemars::schema_for!(Plan),
                SchemaKind::ReleaseInventory => {
                    schemars::schema_for!(armorer::trust::inventory::ReleaseInventory)
                }
                SchemaKind::VerificationPolicy => {
                    schemars::schema_for!(armorer::trust::policy::VerificationPolicy)
                }
                SchemaKind::ArtifactEvidence => {
                    schemars::schema_for!(armorer::trust::evidence::ArtifactEvidence)
                }
                SchemaKind::EvidenceRequirements => {
                    schemars::schema_for!(armorer::trust::evidence::EvidenceRequirements)
                }
                SchemaKind::CapabilityConfig => {
                    schemars::schema_for!(armorer::trust::capability::CapabilityConfig)
                }
                SchemaKind::Catalog => schemars::schema_for!(armorer::trust::capability::Catalog),
                SchemaKind::CapabilityObservation => {
                    schemars::schema_for!(armorer::trust::capability::CapabilityObservation)
                }
                SchemaKind::LifecycleRecord => {
                    schemars::schema_for!(armorer::trust::publication::LifecycleRecord)
                }
                SchemaKind::GithubReceipt => {
                    schemars::schema_for!(armorer::trust::publication::GithubReceipt)
                }
                SchemaKind::PublishSet => {
                    schemars::schema_for!(armorer::trust::publication::PublishSet)
                }
                SchemaKind::RegistryReceipt => {
                    schemars::schema_for!(armorer::trust::publication::RegistryReceipt)
                }
            };
            Ok((serde_json::to_value(schema).map_err(|_| Error::Json)?, 0))
        }
        Operation::Apply {
            plan,
            expect_plan_sha256,
        } => {
            let plan = armorer::apply::load_plan(&plan, &expect_plan_sha256)?;
            Ok((
                serde_json::to_value(armorer::apply::apply(
                    &cli.repository,
                    &plan,
                    &expect_plan_sha256,
                )?)
                .map_err(|_| Error::Json)?,
                0,
            ))
        }
        Operation::Recover { expect_plan_sha256 } => Ok((
            serde_json::to_value(armorer::apply::recover(
                &cli.repository,
                &expect_plan_sha256,
            )?)
            .map_err(|_| Error::Json)?,
            0,
        )),
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
