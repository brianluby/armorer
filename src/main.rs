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
    Plan {
        /// Include exact before/proposed text and complete diffs in a review envelope.
        #[arg(long)]
        preview: bool,
    },
    /// Show exact before/proposed bytes and diffs without writing consumer files.
    Preview {
        /// Optionally review this saved v1 plan against current repository state.
        #[arg(long, requires = "expect_plan_sha256")]
        plan: Option<PathBuf>,
        #[arg(long, requires = "plan")]
        expect_plan_sha256: Option<String>,
    },
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
    /// Authenticate the complete release set; Apple executables also require native macOS checks.
    VerifyRelease {
        #[arg(long)]
        directory: PathBuf,
        #[arg(long)]
        trusted_inputs: PathBuf,
        #[arg(long)]
        expect_context_sha256: String,
        #[arg(long, value_enum)]
        context_kind: ContextKind,
        #[arg(long)]
        gh: PathBuf,
        #[arg(long)]
        trusted_root: PathBuf,
        #[arg(long)]
        cyclonedx: PathBuf,
    },
    /// Explicitly compare independently approved historical bytes; establishes no provenance authentication.
    VerifyHistoricalBytes {
        #[arg(long)]
        directory: PathBuf,
        #[arg(long)]
        policy: PathBuf,
        #[arg(long)]
        expect_policy_sha256: String,
        #[arg(long)]
        source_repository: String,
        #[arg(long)]
        source_commit: String,
        #[arg(long)]
        source_tag: String,
    },
    /// Print a structural JSON schema. Semantic rules are also checked at runtime.
    Schema {
        #[arg(value_enum)]
        kind: SchemaKind,
    },
}

/// Explicitly select frozen catalog semantics; failures never retry another version.
#[derive(Clone, ValueEnum)]
enum ContextKind {
    LegacyV1,
    NativeV2,
    NativeV3,
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
    NativeCatalogV2,
    RuntimeDistributionV1,
    VerificationContextV2,
    VerificationContextV3,
    VerificationContext,
    CargoGraphV2,
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
/// Schema, catalog, plan, preview, apply, recovery, release verification and historical
/// comparison return 0; check returns 2 because release
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
        Operation::VerifyRelease {
            directory,
            trusted_inputs,
            expect_context_sha256,
            context_kind,
            gh,
            trusted_root,
            cyclonedx,
        } => {
            use armorer::verification::{
                apple::VerifiedAppleRelease, context::TrustedReleaseContext,
                cyclonedx::OfflineSbomValidator, release::AuthenticatedReleaseFiles,
                sigstore::OfflineVerifier,
            };
            // Authenticate independent intent before reading a release or opening executable adapters.
            let context = match context_kind {
                ContextKind::LegacyV1 => {
                    TrustedReleaseContext::open(&trusted_inputs, &expect_context_sha256)?
                }
                ContextKind::NativeV2 => {
                    TrustedReleaseContext::open_native_v2(&trusted_inputs, &expect_context_sha256)?
                }
                ContextKind::NativeV3 => {
                    TrustedReleaseContext::open_native_v3(&trusted_inputs, &expect_context_sha256)?
                }
            };
            let verifier = OfflineVerifier::open(
                &trusted_inputs.join("verification-policy.json"),
                &context.policy_identity().sha256,
                &gh,
                &trusted_root,
            )?;
            let sbom_validator = OfflineSbomValidator::open(&cyclonedx, context.sbom_validator())?;
            let verified = AuthenticatedReleaseFiles::verify(
                &directory,
                &context,
                &verifier,
                &sbom_validator,
            )?;
            let apple = VerifiedAppleRelease::verify(&verified, &context)?;
            Ok((
                json!({
                    "status": "authenticated-release-files",
                    "cryptographic_release_authenticated": true,
                    "provenance_verified": true,
                    "publication_authorized": false,
                    "slsa_build_level": null,
                    "context": verified.context_identity(),
                    "catalog": context.catalog_identity(),
                    "inputs": context.inputs(),
                    "inventory": verified.inventory_proof().subject(),
                    "inventory_bundle": verified.inventory_proof().bundle(),
                    "verification_policy": verified.inventory_proof().policy(),
                    "verifier": verified.inventory_proof().verifier(),
                    "trusted_root": verified.inventory_proof().trusted_root(),
                    "assets": verified.inventory().assets,
                    "attestation_bundles_verified": verified.attestations().len() + 1,
                    "sboms_verified": verified.sboms().len(),
                    "apple_verification": if apple.payloads().is_empty() { "not-required" } else { "native-verified" },
                    "apple_payloads": apple.payloads(),
                }),
                0,
            ))
        }
        Operation::VerifyHistoricalBytes {
            directory,
            policy,
            expect_policy_sha256,
            source_repository,
            source_commit,
            source_tag,
        } => {
            let source = armorer::trust::Source {
                repository: source_repository,
                commit: source_commit,
                git_ref: format!("refs/tags/{source_tag}"),
            };
            let result = armorer::verification::historical::HistoricalByteMatch::verify(
                &directory,
                &policy,
                &expect_policy_sha256,
                &source,
            )?;
            Ok((
                json!({
                    "status": "historical-byte-match",
                    "authenticity": "not-established",
                    "provenance_verified": false,
                    "slsa_build_level": null,
                    "requested_source": result.source(),
                    "approved_policy": result.policy_identity(),
                    "assets": result.assets(),
                    "limitations": result.limitations(),
                }),
                0,
            ))
        }
        Operation::Schema { kind } => {
            let schema = match kind {
                SchemaKind::NativeCatalogV2 => {
                    schemars::schema_for!(armorer::trust::native::NativeCatalogV2)
                }
                SchemaKind::RuntimeDistributionV1 => {
                    schemars::schema_for!(armorer::trust::native::RuntimeDistributionV1)
                }
                SchemaKind::VerificationContextV3 => {
                    schemars::schema_for!(armorer::verification::context::ReleaseExpectationsV3)
                }
                SchemaKind::VerificationContextV2 => {
                    schemars::schema_for!(armorer::verification::context::ReleaseExpectationsV2)
                }
                SchemaKind::VerificationContext => {
                    schemars::schema_for!(armorer::verification::context::ReleaseExpectations)
                }
                SchemaKind::CargoGraphV2 => {
                    schemars::schema_for!(armorer::verification::graph::CargoGraphV2)
                }
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
        Operation::Preview {
            plan,
            expect_plan_sha256,
        } => {
            let preview = match (plan, expect_plan_sha256) {
                (Some(path), Some(expected)) => {
                    let plan = armorer::apply::load_plan(&path, &expected)?;
                    armorer::preview::preview_plan(&cli.repository, &plan)?
                }
                (None, None) => armorer::preview::preview(&cli.repository)?,
                _ => {
                    return Err(Error::Transaction(
                        "saved preview requires a plan and digest",
                    ));
                }
            };
            Ok((serde_json::to_value(preview).map_err(|_| Error::Json)?, 0))
        }
        Operation::Plan { preview: true } => Ok((
            serde_json::to_value(armorer::preview::preview(&cli.repository)?)
                .map_err(|_| Error::Json)?,
            0,
        )),
        Operation::Plan { preview: false } => Ok((
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
