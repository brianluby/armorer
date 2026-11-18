use armorer::{
    Error, Result,
    config::{Config, Lock},
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::json;
use std::{io::Write, path::PathBuf};

mod render;

#[derive(Parser)]
#[command(
    version,
    about = "Inspect Rust repositories and apply reviewed local setup"
)]
struct Cli {
    #[arg(long, default_value = ".", global = true)]
    repository: PathBuf,
    /// Output format: json (default, the machine contract) or text (reviewer summary).
    #[arg(long, value_enum, default_value_t = OutputFormat::Json, global = true)]
    format: OutputFormat,
    #[command(subcommand)]
    command: Operation,
}

/// Selects how stdout is rendered; exit statuses and stderr never depend on it.
#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Text,
}

#[derive(Subcommand)]
enum Operation {
    /// Report bootstrap drift over .armorer/ci-policy.toml (JSON; exit 0 in sync, 2 on drift).
    Check,
    /// Propose armorer.toml and a policy draft from read-only discovery (JSON); --write creates them.
    Init {
        /// GitHub owner/name recorded in the proposed armorer.toml.
        #[arg(long)]
        repo_name: String,
        /// Exact stable x.y.z toolchain; defaults only to an exact rust-toolchain.toml channel.
        #[arg(long)]
        toolchain: Option<String>,
        /// Create the reviewed files; never overwrites an existing file.
        #[arg(long, requires = "expect_proposal_sha256")]
        write: bool,
        /// Independently reviewed proposal_sha256 that authorizes --write.
        #[arg(long, requires = "write")]
        expect_proposal_sha256: Option<String>,
    },
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
    /// Review and control a separately approved exact owned release, with native capability gates.
    Publication {
        #[command(flatten)]
        inputs: PublicationInputs,
        #[command(subcommand)]
        operation: PublicationOperation,
    },
    /// Print a structural JSON schema. Semantic rules are also checked at runtime.
    Schema {
        #[arg(value_enum)]
        kind: SchemaKind,
    },
}

/// Independently approved inputs are authenticated before credentials or a native publication adapter are opened.
#[derive(Args)]
struct PublicationInputs {
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
    #[arg(long)]
    publication_policy: PathBuf,
    #[arg(long)]
    expect_publication_policy_sha256: String,
    #[arg(long)]
    release_attestation_root: PathBuf,
}

#[derive(Subcommand)]
enum PublicationOperation {
    /// Authenticate and freeze exact assets for independent review; opens no credential-bearing client.
    Plan,
    /// Observe native prerequisites without any remote writes; settings alone grant no authority.
    Inspect {
        #[arg(long)]
        owner_home: Option<PathBuf>,
        #[arg(long)]
        state_directory: PathBuf,
    },
    /// Stage only a separately approved new or identical owned draft; unsupported native gates block writes.
    Stage {
        #[arg(long)]
        approval: PathBuf,
        #[arg(long)]
        expect_approval_sha256: String,
        #[arg(long)]
        state_directory: PathBuf,
    },
    /// Publish only the separately approved exact draft after fresh served-byte and mutable checks.
    Publish {
        #[arg(long)]
        approval: PathBuf,
        #[arg(long)]
        expect_approval_sha256: String,
        #[arg(long)]
        state_directory: PathBuf,
    },
    /// Resolve an ambiguous publication through read-only verification; never repeat the provider write.
    RecoverPublished {
        #[arg(long)]
        owner_home: Option<PathBuf>,
        #[arg(long)]
        state_directory: PathBuf,
    },
}

/// Explicitly select frozen catalog semantics; failures never retry another version.
#[derive(Clone, Copy, ValueEnum)]
enum ContextKind {
    NativeV3,
}

#[derive(Subcommand)]
enum BootstrapOperation {
    /// Preview exact bootstrap files using explicit owner-reviewed policy (read only).
    Plan {
        #[arg(long)]
        policy: PathBuf,
    },
    /// Report provisioning drift without writing (exit 0 in sync, exit 2 on drift).
    Check {
        #[arg(long)]
        policy: PathBuf,
        /// Also require secure release readiness; exits 2 while any release gate is open.
        #[arg(long)]
        release_ready: bool,
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
    PublicationPolicy,
    PublicationPlan,
    PublicationApproval,
    NativeCatalogV2,
    RuntimeDistributionV1,
    VerificationContextV3,
    CargoGraphV2,
    UpgradePlan,
    UpgradeRollback,
    BootstrapPlan,
    InitProposal,
    CiPolicy,
    Config,
    Lock,
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
/// comparison return 0; check returns 0 in sync and 2 on drift. Propagates
/// inspection errors and maps JSON value conversion failures to `Error::Json`;
/// output is left to the caller.
/// Authenticate complete inert release bytes and native Apple evidence before creating publication intent.
fn freeze_publication_inputs(
    inputs: &PublicationInputs,
) -> Result<(
    armorer::verification::context::TrustedReleaseContext,
    armorer::verification::publication::TrustedPublicationPolicy,
    armorer::verification::publication::FrozenRelease,
)> {
    use armorer::verification::publication::{FrozenRelease, TrustedPublicationPolicy};
    let context = open_trusted_context(
        &inputs.trusted_inputs,
        &inputs.expect_context_sha256,
        inputs.context_kind,
    )?;
    // Publication intent is authenticated after the context and before executable adapters.
    let policy = TrustedPublicationPolicy::open(
        &inputs.publication_policy,
        &inputs.expect_publication_policy_sha256,
        &context,
    )?;
    let (files, apple) = authenticate_release_files(
        &inputs.directory,
        &inputs.trusted_inputs,
        &context,
        &inputs.gh,
        &inputs.trusted_root,
        &inputs.cyclonedx,
    )?;
    let frozen = FrozenRelease::freeze(&files, &apple, &context, &policy)?;
    Ok((context, policy, frozen))
}

/// Open the explicitly selected trusted context version; errors never fall back.
fn open_trusted_context(
    trusted_inputs: &std::path::Path,
    expect_context_sha256: &str,
    context_kind: ContextKind,
) -> Result<armorer::verification::context::TrustedReleaseContext> {
    use armorer::verification::context::TrustedReleaseContext;
    match context_kind {
        ContextKind::NativeV3 => {
            TrustedReleaseContext::open_native_v3(trusted_inputs, expect_context_sha256)
        }
    }
}

/// Shared security sequence for VerifyRelease and publication freezing: verifier,
/// SBOM validator, authenticated release files and native Apple evidence, in order.
/// A fix or reordering here applies to both callers.
fn authenticate_release_files(
    directory: &std::path::Path,
    trusted_inputs: &std::path::Path,
    context: &armorer::verification::context::TrustedReleaseContext,
    gh: &std::path::Path,
    trusted_root: &std::path::Path,
    cyclonedx: &std::path::Path,
) -> Result<(
    armorer::verification::release::AuthenticatedReleaseFiles,
    armorer::verification::apple::VerifiedAppleRelease,
)> {
    use armorer::verification::{
        apple::VerifiedAppleRelease, cyclonedx::OfflineSbomValidator,
        release::AuthenticatedReleaseFiles, sigstore::OfflineVerifier,
    };
    let verifier = OfflineVerifier::open(
        &trusted_inputs.join("verification-policy.json"),
        &context.policy_identity().sha256,
        gh,
        trusted_root,
    )?;
    let validator = OfflineSbomValidator::open(cyclonedx, context.sbom_validator())?;
    let files = AuthenticatedReleaseFiles::verify(directory, context, &verifier, &validator)?;
    let apple = VerifiedAppleRelease::verify(&files, context)?;
    Ok((files, apple))
}

/// Route explicit actions through independent intent and preserve existing read-only discovery behavior.
fn run(cli: Cli) -> Result<(serde_json::Value, i32)> {
    match cli.command {
        Operation::Publication { inputs, operation } => {
            use armorer::verification::publication::{
                ApprovedPublication, NativeGithub, OwnedDraftController,
            };
            let (context, policy, frozen) = freeze_publication_inputs(&inputs)?;
            // One exhaustive match: Plan returns before any credential access, and a new
            // variant must be handled here at compile time instead of reaching a panic.
            // Authenticate an independent action before reading credentials; inspect and
            // recovery are GET-only.
            let (state_directory, owner_home, read_only, approval) = match &operation {
                PublicationOperation::Plan => {
                    return Ok((
                        json!({"plan":frozen.plan(), "plan_identity":frozen.identity(),
                        "remote_mutations":false, "publication_authorized":false}),
                        0,
                    ));
                }
                PublicationOperation::Inspect {
                    state_directory,
                    owner_home,
                }
                | PublicationOperation::RecoverPublished {
                    state_directory,
                    owner_home,
                } => (state_directory, owner_home.as_deref(), true, None),
                PublicationOperation::Stage {
                    state_directory,
                    approval,
                    expect_approval_sha256,
                    ..
                }
                | PublicationOperation::Publish {
                    state_directory,
                    approval,
                    expect_approval_sha256,
                    ..
                } => (
                    state_directory,
                    None,
                    false,
                    Some(ApprovedPublication::open(
                        approval,
                        expect_approval_sha256,
                        &frozen,
                        &policy,
                    )?),
                ),
            };
            let client = if read_only {
                let home = owner_home
                    .map(PathBuf::from)
                    .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
                    .ok_or_else(|| {
                        Error::Invalid("owner home not provided and HOME is unset".into())
                    })?;
                NativeGithub::open_owner_read_only(
                    &inputs.gh,
                    &inputs.release_attestation_root,
                    &policy,
                    &home,
                )?
            } else {
                NativeGithub::open(&inputs.gh, &inputs.release_attestation_root, &policy)?
            }
            .bind_repository(&context, &policy)?;
            let mut controller =
                OwnedDraftController::open(client, frozen, policy, state_directory, context)?;
            let value = match operation {
                PublicationOperation::Inspect { .. } => serde_json::to_value(controller.inspect()?),
                PublicationOperation::Stage { .. } => {
                    serde_json::to_value(controller.stage(approval.as_ref().ok_or_else(
                        || Error::Invalid("stage requires an authenticated approval".into()),
                    )?)?)
                }
                PublicationOperation::Publish { .. } => {
                    serde_json::to_value(controller.publish(approval.as_ref().ok_or_else(
                        || Error::Invalid("publish requires an authenticated approval".into()),
                    )?)?)
                }
                PublicationOperation::RecoverPublished { .. } => {
                    serde_json::to_value(controller.recover_published()?)
                }
                // Dead by construction: Plan returned in the exhaustive match above.
                PublicationOperation::Plan => {
                    return Err(Error::Invalid(
                        "publication plan is served before authentication".into(),
                    ));
                }
            };
            Ok((value.map_err(|_| Error::Json)?, 0))
        }
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
                BootstrapOperation::Check {
                    policy,
                    release_ready,
                } => {
                    let plan = armorer::bootstrap::inspect(&cli.repository, &policy)?;
                    // Reasons go to stderr; stdout stays the unchanged plan document.
                    let drift = plan.drift();
                    // The strict gate fails only while a blocker exists.
                    let blockers = if release_ready {
                        plan.release_blockers()
                    } else {
                        Vec::new()
                    };
                    let status = if !drift.is_empty() {
                        eprintln!("armorer: provisioning drift: {}", drift.join(", "));
                        2
                    } else if !blockers.is_empty() {
                        eprintln!(
                            "armorer: release readiness not established: {}",
                            blockers.join(", ")
                        );
                        2
                    } else {
                        0
                    };
                    (serde_json::to_value(plan), status)
                }
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
            serde_json::to_value(armorer::catalog::reviewed_v2()?).map_err(|_| Error::Json)?,
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
            // Authenticate independent intent before reading a release or opening
            // executable adapters; the sequence is shared with publication freezing.
            let context =
                open_trusted_context(&trusted_inputs, &expect_context_sha256, context_kind)?;
            let (verified, apple) = authenticate_release_files(
                &directory,
                &trusted_inputs,
                &context,
                &gh,
                &trusted_root,
                &cyclonedx,
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
                    "attestation_bundles_verified": verified.verified_bundle_count(),
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
        Operation::Init {
            repo_name,
            toolchain,
            write,
            expect_proposal_sha256,
        } => {
            let expected = expect_proposal_sha256.as_deref().filter(|_| write);
            let value =
                armorer::init::run(&cli.repository, &repo_name, toolchain.as_deref(), expected)?;
            if expected.is_none() {
                eprintln!(
                    "armorer: nothing was written; review files and findings, then rerun with --write --expect-proposal-sha256 <reviewed proposal_sha256>"
                );
            }
            Ok((value, 0))
        }
        Operation::Schema { kind } => {
            let schema = match kind {
                SchemaKind::PublicationPolicy => {
                    schemars::schema_for!(armorer::verification::publication::PublicationPolicy)
                }
                SchemaKind::PublicationPlan => {
                    schemars::schema_for!(armorer::verification::publication::PublicationPlan)
                }
                SchemaKind::PublicationApproval => {
                    schemars::schema_for!(armorer::verification::publication::PublicationApproval)
                }
                SchemaKind::NativeCatalogV2 => {
                    schemars::schema_for!(armorer::trust::native::NativeCatalogV2)
                }
                SchemaKind::RuntimeDistributionV1 => {
                    schemars::schema_for!(armorer::trust::native::RuntimeDistributionV1)
                }
                SchemaKind::VerificationContextV3 => {
                    schemars::schema_for!(armorer::verification::context::ReleaseExpectationsV3)
                }
                SchemaKind::CargoGraphV2 => {
                    schemars::schema_for!(armorer::verification::graph::CargoGraphV2)
                }
                SchemaKind::UpgradeRollback => {
                    schemars::schema_for!(armorer::upgrade::RollbackPlan)
                }
                SchemaKind::UpgradePlan => schemars::schema_for!(armorer::upgrade::Plan),
                SchemaKind::BootstrapPlan => schemars::schema_for!(armorer::bootstrap::Plan),
                SchemaKind::InitProposal => schemars::schema_for!(armorer::init::Proposal),
                SchemaKind::CiPolicy => schemars::schema_for!(armorer::ci_policy::CiPolicy),
                SchemaKind::Config => schemars::schema_for!(Config),
                SchemaKind::Lock => schemars::schema_for!(Lock),
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
        Operation::Check => {
            // The surviving contract: bootstrap drift over the managed policy.
            let plan = armorer::bootstrap::inspect(
                &cli.repository,
                &armorer::safe_path(&cli.repository, ".armorer/ci-policy.toml")?,
            )?;
            let drift = plan.drift();
            let status = if drift.is_empty() {
                0
            } else {
                eprintln!("armorer: provisioning drift: {}", drift.join(", "));
                2
            };
            Ok((serde_json::to_value(plan).map_err(|_| Error::Json)?, status))
        }
    }
}

/// Parse CLI arguments, write pretty JSON to stdout, and exit with the operation's status.
///
/// Operation errors become JSON errors with status 1. Serialization or stdout
/// write failures also exit with status 1; argument parsing is handled by clap.
fn main() {
    let cli = Cli::parse();
    // `cfg!` keeps this block type-checked on every host (and free of
    // unreachable-code warnings) while refusing up front on non-Unix hosts.
    if !cfg!(unix) {
        let refusal = json!({"error": {
            "code": "unsupported-host",
            "message": "Armorer supports Linux and macOS hosts only"
        }});
        let written = serde_json::to_vec_pretty(&refusal).is_ok_and(|bytes| {
            let mut stdout = std::io::stdout().lock();
            stdout
                .write_all(&bytes)
                .and_then(|()| stdout.write_all(b"\n"))
                .is_ok()
        });
        if !written {
            eprintln!("armorer: failed to write JSON output to stdout");
        }
        std::process::exit(1);
    }
    let format = cli.format;
    let (value, code) = match run(cli) {
        Ok(result) => result,
        Err(error) => (
            json!({"error": {"code": error.code(), "message": error.to_string()}}),
            1,
        ),
    };
    let output = match format {
        OutputFormat::Json => serde_json::to_vec_pretty(&value),
        OutputFormat::Text => match render::render(&value) {
            Some(text) => Ok(text.into_bytes()),
            None => {
                eprintln!("armorer: no text view for this output; showing JSON");
                serde_json::to_vec_pretty(&value)
            }
        },
    };
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
        _ => {
            eprintln!("armorer: failed to write JSON output to stdout");
            std::process::exit(1)
        }
    }
}
