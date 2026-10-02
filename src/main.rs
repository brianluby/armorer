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
    /// Authenticate every release file against independent inputs using pinned offline native verifiers.
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
}

#[derive(Clone, ValueEnum)]
enum SchemaKind {
    NativeCatalogV2,
    RuntimeDistributionV1,
    VerificationContextV2,
    VerificationContext,
    CargoGraphV2,
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
/// Successful schema, plan, apply, recovery, complete release verification and historical comparison return 0; check returns 2 because release
/// readiness remains blocked. Propagates inspection errors and maps JSON value
/// conversion failures to `Error::Json`; output is left to the caller.
fn run(cli: Cli) -> Result<(serde_json::Value, i32)> {
    match cli.command {
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
                context::TrustedReleaseContext, cyclonedx::OfflineSbomValidator,
                release::AuthenticatedReleaseFiles, sigstore::OfflineVerifier,
            };
            // Authenticate independent intent before reading a release or opening executable adapters.
            let context = match context_kind {
                ContextKind::LegacyV1 => {
                    TrustedReleaseContext::open(&trusted_inputs, &expect_context_sha256)?
                }
                ContextKind::NativeV2 => {
                    TrustedReleaseContext::open_native_v2(&trusted_inputs, &expect_context_sha256)?
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
                SchemaKind::VerificationContextV2 => {
                    schemars::schema_for!(armorer::verification::context::ReleaseExpectationsV2)
                }
                SchemaKind::VerificationContext => {
                    schemars::schema_for!(armorer::verification::context::ReleaseExpectations)
                }
                SchemaKind::CargoGraphV2 => {
                    schemars::schema_for!(armorer::verification::graph::CargoGraphV2)
                }
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
