//! Native Apple checks over already authenticated release bytes; no executable is run.
use super::{context::TrustedReleaseContext, io, release::AuthenticatedReleaseFiles, sigstore};
use crate::{
    Error, Result,
    config::Profile,
    trust::{
        ByteIdentity,
        evidence::{AppleTicketMode, ArtifactEvidence, StepKind},
        require,
    },
};
use flate2::bufread::GzDecoder;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    io::Read,
    path::Path,
    time::{Duration, Instant},
};

const MAX_BINARY: usize = 1024 * 1024 * 1024;
const MAX_METADATA: usize = 17 * 1024 * 1024;
#[cfg(any(target_os = "macos", test))]
const MAX_SIGNATURE: usize = 16 * 1024 * 1024;

/// Audit information derived from a native check; serialized data cannot construct the proof.
#[derive(Debug, Clone, Serialize)]
pub struct NativeApplePayload {
    archive: ByteIdentity,
    binary: ByteIdentity,
    certificate_sha256: String,
    secure_timestamp: u64,
    notarization: &'static str,
    online_notarization_check_requested: bool,
}

/// Whole required Apple set, bound to the same private approved context and authenticated files.
/// ```compile_fail
/// let proof: armorer::verification::apple::VerifiedAppleRelease = serde_json::from_str("{}").unwrap();
/// ```
pub struct VerifiedAppleRelease {
    context: ByteIdentity,
    payloads: BTreeMap<String, NativeApplePayload>,
}

impl VerifiedAppleRelease {
    /// Authenticate the entire release first; a required Apple failure yields no partial proof.
    /// Linux cannot verify Apple executables. Library-only Apple selections need no native signing.
    pub fn verify(
        files: &AuthenticatedReleaseFiles,
        context: &TrustedReleaseContext,
    ) -> Result<Self> {
        context.validate_at(sigstore::wall_time()?)?;
        require(
            files.context_identity() == context.identity(),
            "apple-release-context-mismatch",
        )?;
        let started = Instant::now();
        let mut payloads = BTreeMap::new();
        for selected in context.selections() {
            let selection = &selected.selection;
            if selection.profile == Profile::Library || selection.target != "aarch64-apple-darwin" {
                continue;
            }
            require(
                cfg!(target_os = "macos"),
                "native-apple-verification-unsupported-platform",
            )?;
            let team = context
                .policy()
                .apple_team
                .as_deref()
                .ok_or_else(|| invalid("apple-team-required"))?;
            require(valid_team(team), "apple-team-format-unsupported")?;
            let key = selection.key();
            let name = format!("{key}.tar.gz");
            let archive = files.asset_bytes(&name)?;
            let archive_identity = identity(&archive);
            let evidence_bytes = files.asset_bytes(&format!("{key}.apple.json"))?;
            require(
                evidence_bytes.len() <= MAX_METADATA,
                "apple-evidence-size-limit",
            )?;
            let evidence: ArtifactEvidence = io::parse(&evidence_bytes)?;
            evidence.validate_against_requirements(
                context.config(),
                files.inventory(),
                &selected.evidence_requirements,
                sigstore::wall_time()?,
            )?;
            let assertions = evidence
                .apple_assertions
                .as_ref()
                .ok_or_else(|| invalid("apple-assertions-missing"))?;
            require(
                assertions.ticket_mode == AppleTicketMode::OnlineStandaloneMachO,
                "apple-stapled-package-verification-unsupported",
            )?;
            let binary_name = selection
                .binary
                .as_deref()
                .ok_or_else(|| invalid("apple-binary-required"))?;
            let binary = unpack(&archive, binary_name)?;
            let binary_identity = identity(&binary);
            let sign = evidence
                .steps
                .iter()
                .find(|step| step.kind == StepKind::Sign)
                .ok_or_else(|| invalid("apple-sign-step-missing"))?;
            let notarize = evidence
                .steps
                .iter()
                .find(|step| step.kind == StepKind::Notarize)
                .ok_or_else(|| invalid("apple-notarize-step-missing"))?;
            let package = evidence
                .steps
                .iter()
                .find(|step| step.kind == StepKind::Package)
                .ok_or_else(|| invalid("apple-package-step-missing"))?;
            require(
                sign.output == binary_identity
                    && notarize.output == binary_identity
                    && package.output == archive_identity,
                "apple-native-byte-chain-mismatch",
            )?;
            let workspace = crate::filesystem::private_tempdir("armorer-apple-native-", None)?;
            let path = workspace.path().join("payload.macho");
            io::write_readonly(&path, &binary)?;
            let timestamp = native(&path, &binary, team, &assertions.certificate_sha256)?;
            require(
                sign.started_at <= timestamp
                    && timestamp <= sign.finished_at
                    && timestamp <= sigstore::wall_time()?,
                "apple-secure-timestamp-step-mismatch",
            )?;
            binary_identity.matches(&io::read_bounded(&path, MAX_BINARY as u64)?)?;
            require(
                started.elapsed() <= Duration::from_secs(20 * 60),
                "apple-native-stage-expired",
            )?;
            context.validate_at(sigstore::wall_time()?)?;
            payloads.insert(
                key,
                NativeApplePayload {
                    archive: archive_identity,
                    binary: binary_identity,
                    certificate_sha256: assertions.certificate_sha256.clone(),
                    secure_timestamp: timestamp,
                    notarization: "system-ticket-verified-cache-or-network",
                    online_notarization_check_requested: true,
                },
            );
        }
        context.validate_at(sigstore::wall_time()?)?;
        Ok(Self {
            context: context.identity().clone(),
            payloads,
        })
    }

    /// Return checked subjects; an empty set means native Apple signing was not required.
    pub fn payloads(&self) -> &BTreeMap<String, NativeApplePayload> {
        &self.payloads
    }

    /// Identify the independently approved context to which every checked Apple payload belongs.
    pub fn context_identity(&self) -> &ByteIdentity {
        &self.context
    }
}

/// Use static constant errors so offered metadata never reaches diagnostic messages.
fn invalid(code: &str) -> Error {
    Error::Invalid(code.into())
}

/// Measure actual bytes; a digest is an identity, not authentication by itself.
fn identity(bytes: &[u8]) -> ByteIdentity {
    ByteIdentity {
        sha256: crate::digest(bytes),
        size: bytes.len() as u64,
    }
}

/// Restrict the independent team identifier before constructing native requirement syntax.
fn valid_team(team: &str) -> bool {
    team.len() == 10
        && team
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
}

/// Read bounded canonical octal without sign, base-256 extensions or arbitrary whitespace.
fn octal(bytes: &[u8]) -> Result<usize> {
    let digits = bytes
        .strip_suffix(&[0])
        .or_else(|| bytes.strip_suffix(b" "))
        .ok_or_else(|| invalid("apple-tar-number-format"))?;
    require(
        !digits.is_empty() && digits.iter().all(|byte| (b'0'..=b'7').contains(byte)),
        "apple-tar-number-format",
    )?;
    digits.iter().try_fold(0usize, |value, byte| {
        value
            .checked_mul(8)
            .and_then(|value| value.checked_add(usize::from(byte - b'0')))
            .ok_or_else(|| invalid("apple-tar-number-overflow"))
    })
}

/// Decode one bounded gzip member and one inert regular USTAR leaf, never extracting paths.
fn unpack(archive: &[u8], expected_name: &str) -> Result<Vec<u8>> {
    require(
        !archive.is_empty()
            && archive.len() <= MAX_BINARY
            && !expected_name.is_empty()
            && expected_name.len() < 100
            && crate::config::identifier(expected_name),
        "apple-package-bounds-or-name",
    )?;
    let mut decoder = GzDecoder::new(archive);
    let mut tar = Vec::new();
    (&mut decoder)
        .take((MAX_BINARY + 16_384) as u64)
        .read_to_end(&mut tar)?;
    require(
        decoder.get_ref().is_empty(),
        "apple-gzip-extra-member-or-trailing-bytes",
    )?;
    require(
        tar.len() >= 1536 && tar.len() <= MAX_BINARY + 16_383 && tar.len().is_multiple_of(512),
        "apple-tar-size-limit",
    )?;
    let header = &tar[..512];
    require(
        header[..expected_name.len()] == *expected_name.as_bytes()
            && header[expected_name.len()..100]
                .iter()
                .all(|byte| *byte == 0)
            && &header[257..265] == b"ustar\0\x30\x30"
            && header[156] == b'0'
            && header[157..257].iter().all(|byte| *byte == 0)
            && header[265..329].iter().all(|byte| *byte == 0)
            && header[345..512].iter().all(|byte| *byte == 0),
        "apple-tar-unsafe-or-unexpected-leaf",
    )?;
    require(
        octal(&header[100..108])? == 0o755
            && octal(&header[108..116])? == 0
            && octal(&header[116..124])? == 0
            && octal(&header[136..148])? == 0,
        "apple-tar-noncanonical-ownership-or-mode",
    )?;
    require(
        header[329..345].iter().all(|byte| *byte == 0),
        "apple-tar-device-fields",
    )?;
    let checksum = header
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            if (148..156).contains(&index) {
                32
            } else {
                usize::from(*byte)
            }
        })
        .sum::<usize>();
    require(
        header[154..156] == [0, b' '] && octal(&header[148..155])? == checksum,
        "apple-tar-checksum-mismatch",
    )?;
    let size = octal(&header[124..136])?;
    require(
        size > 0 && size <= MAX_BINARY && size <= tar.len() - 512,
        "apple-tar-subject-size",
    )?;
    let end = 512 + size;
    let padded_end = end.next_multiple_of(512);
    require(
        tar.len() >= padded_end + 1024 && tar[end..].iter().all(|byte| *byte == 0),
        "apple-tar-extra-or-nonzero-leaf",
    )?;
    Ok(tar[512..end].to_vec())
}

/// Decode a bounded endian-specific word without indexing unchecked offered offsets.
#[cfg(any(target_os = "macos", test))]
fn word(bytes: &[u8], offset: usize, little: bool) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| invalid("apple-signature-offset-overflow"))?;
    let array: [u8; 4] = bytes
        .get(offset..end)
        .ok_or_else(|| invalid("apple-signature-truncated"))?
        .try_into()
        .map_err(|_| invalid("apple-signature-truncated"))?;
    Ok(if little {
        u32::from_le_bytes(array)
    } else {
        u32::from_be_bytes(array)
    })
}

/// Parsed slices remain unauthenticated until Security.framework validates the same private file.
#[cfg(any(target_os = "macos", test))]
struct Signature<'a> {
    directory: &'a [u8],
    cms: &'a [u8],
}

/// Bound the thin ARM64 executable and unique, nonoverlapping signing blobs; no image is loaded.
#[cfg(any(target_os = "macos", test))]
fn signature<'a>(binary: &'a [u8], expected_team: &str) -> Result<Signature<'a>> {
    require(
        binary.len() >= 32
            && binary.len() <= MAX_BINARY
            && word(binary, 0, true)? == 0xfeedfacf
            && word(binary, 4, true)? == 0x0100000c
            && word(binary, 12, true)? == 2,
        "apple-macho-target-or-kind",
    )?;
    let count = word(binary, 16, true)? as usize;
    let size = word(binary, 20, true)? as usize;
    require(
        count > 0 && count <= 4096 && size <= 1024 * 1024 && 32 + size <= binary.len(),
        "apple-macho-command-bounds",
    )?;
    let mut cursor = 32;
    let mut location = None;
    for _ in 0..count {
        let kind = word(binary, cursor, true)?;
        let length = word(binary, cursor + 4, true)? as usize;
        require(
            length >= 8 && length.is_multiple_of(8) && cursor + length <= 32 + size,
            "apple-macho-command-layout",
        )?;
        if kind == 0x1d {
            require(
                length == 16 && location.is_none(),
                "apple-macho-signature-command-ambiguous",
            )?;
            location = Some((
                word(binary, cursor + 8, true)? as usize,
                word(binary, cursor + 12, true)? as usize,
            ));
        }
        cursor += length;
    }
    require(cursor == 32 + size, "apple-macho-command-total")?;
    let (offset, length) = location.ok_or_else(|| invalid("apple-macho-signature-missing"))?;
    require(
        offset >= cursor
            && (12..=MAX_SIGNATURE).contains(&length)
            && offset.checked_add(length) == Some(binary.len()),
        "apple-macho-signature-bounds",
    )?;
    let blob = &binary[offset..];
    require(
        word(blob, 0, false)? == 0xfade0cc0,
        "apple-signature-superblob-required",
    )?;
    let used = word(blob, 4, false)? as usize;
    let count = word(blob, 8, false)? as usize;
    require(
        (1..=64).contains(&count)
            && 12 + count * 8 <= used
            && used <= blob.len()
            && blob[used..].iter().all(|byte| *byte == 0),
        "apple-signature-table-bounds",
    )?;
    let mut regions = Vec::new();
    let mut kinds = std::collections::BTreeSet::new();
    let (mut directory, mut cms) = (None, None);
    for index in 0..count {
        let kind = word(blob, 12 + index * 8, false)?;
        let start = word(blob, 16 + index * 8, false)? as usize;
        require(
            kinds.insert(kind)
                && matches!(kind, 0 | 2 | 5 | 7 | 0x10000)
                && start >= 12 + count * 8
                && start <= used.saturating_sub(8),
            "apple-signature-slot-unsupported-or-duplicate",
        )?;
        let length = word(blob, start + 4, false)? as usize;
        let end = start
            .checked_add(length)
            .ok_or_else(|| invalid("apple-signature-offset-overflow"))?;
        require(length >= 8 && end <= used, "apple-signature-slot-bounds")?;
        regions.push((start, end));
        if kind == 0 {
            directory = Some(&blob[start..end]);
        }
        if kind == 0x10000 {
            require(
                word(blob, start, false)? == 0xfade0b01 && length > 8,
                "apple-cms-blob-required",
            )?;
            cms = Some(&blob[start + 8..end]);
        }
    }
    regions.sort_unstable();
    require(
        regions.windows(2).all(|pair| pair[0].1 <= pair[1].0),
        "apple-signature-slots-overlap",
    )?;
    let directory = directory.ok_or_else(|| invalid("apple-code-directory-required"))?;
    let cms = cms.ok_or_else(|| invalid("apple-cms-signature-required"))?;
    require(
        directory.len() >= 88
            && word(directory, 0, false)? == 0xfade0c02
            && (0x20400..=0x20600).contains(&word(directory, 8, false)?)
            && word(directory, 12, false)? & 0x10002 == 0x10000
            && directory[36] == 32
            && directory[37] == 2,
        "apple-code-directory-runtime-or-hash",
    )?;
    let team_offset = word(directory, 48, false)? as usize;
    require(
        team_offset >= 88
            && team_offset
                .checked_add(expected_team.len() + 1)
                .is_some_and(|end| end <= directory.len())
            && &directory[team_offset..team_offset + expected_team.len()]
                == expected_team.as_bytes()
            && directory[team_offset + expected_team.len()] == 0,
        "apple-code-directory-team-mismatch",
    )?;
    Ok(Signature { directory, cms })
}

#[cfg(not(target_os = "macos"))]
/// A required native check cannot be simulated on Linux or replaced by producer assertions.
fn native(_path: &Path, _binary: &[u8], _team: &str, _certificate: &str) -> Result<u64> {
    Err(invalid("native-apple-verification-unsupported-platform"))
}

#[cfg(target_os = "macos")]
/// Request the documented online ticket check with fixed system codesign, bounded I/O and no payload execution.
fn check_online_notarization(path: &Path, requirement: &str) -> Result<()> {
    use std::process::{Command, Stdio};
    require(path.is_absolute(), "apple-native-private-path-required")?;
    let directory = path
        .parent()
        .ok_or_else(|| invalid("apple-native-private-directory-required"))?;
    let mut command = Command::new("/usr/bin/codesign");
    command
        .args([
            "--verify",
            "--strict",
            "--check-notarization",
            "--test-requirement",
        ])
        .arg(format!("={requirement}"))
        .arg(path)
        .current_dir(directory)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("HOME", directory)
        .env("TMPDIR", directory)
        .env("LANG", "C")
        .stdin(Stdio::null());
    sigstore::run_process(
        command,
        sigstore::Limits {
            timeout: Duration::from_secs(60),
            stdout: 4096,
            stderr: 16384,
        },
    )
    .map_err(|_| invalid("apple-online-notarization-check-failed"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
/// Validate the frozen file through Apple APIs, then bind authenticated CMS timestamp and leaf identity.
fn native(path: &Path, binary: &[u8], team: &str, certificate: &str) -> Result<u64> {
    use core_foundation::url::CFURL;
    use security_framework::{
        cms::CMSDecoder,
        os::macos::code_signing::{Flags, SecRequirement, SecStaticCode},
        policy::SecPolicy,
    };
    use security_framework_sys::cms::CMSSignerStatus;
    use std::str::FromStr;
    require(
        valid_team(team) && crate::config::hex_digest(certificate, 64),
        "apple-native-expectation-format",
    )?;
    identity(binary).matches(&io::read_bounded(path, MAX_BINARY as u64)?)?;
    let url = CFURL::from_path(path, false).ok_or_else(|| invalid("apple-native-file-url"))?;
    let code = SecStaticCode::from_path(&url, Flags::NONE)
        .map_err(|_| invalid("apple-native-code-object"))?;
    let expected = format!(
        "anchor apple generic and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"{team}\""
    );
    let requirement = SecRequirement::from_str(&expected)
        .map_err(|_| invalid("apple-native-requirement-unsupported"))?;
    // kSecCSAllowNetworkAccess is the documented SDK bit 16; the pinned binding omits its name.
    let flags =
        Flags::STRICT_VALIDATE | Flags::CHECK_ALL_ARCHITECTURES | Flags::from_bits_retain(1 << 16);
    code.check_validity(flags, &requirement)
        .map_err(|_| invalid("apple-developer-id-signature-invalid"))?;
    let signature = signature(binary, team)?;
    let decoder = CMSDecoder::create().map_err(|_| invalid("apple-cms-decoder"))?;
    decoder
        .update_message(signature.cms)
        .map_err(|_| invalid("apple-cms-message"))?;
    decoder
        .set_detached_content(signature.directory)
        .map_err(|_| invalid("apple-cms-detached-content"))?;
    decoder
        .finalize_message()
        .map_err(|_| invalid("apple-cms-finalize"))?;
    require(
        decoder
            .get_num_signers()
            .map_err(|_| invalid("apple-cms-signers"))?
            == 1,
        "apple-cms-ambiguous-signers",
    )?;
    let status = decoder
        .get_signer_status(0, &[SecPolicy::create_x509()])
        .map_err(|_| invalid("apple-cms-signer-status"))?;
    require(
        status.signer_status == CMSSignerStatus::kCMSSignerValid
            && status.cert_verify_result.is_ok(),
        "apple-cms-signature-or-current-certificate-invalid",
    )?;
    let chain = status.sec_trust.chain();
    let leaf = chain
        .first()
        .ok_or_else(|| invalid("apple-cms-leaf-certificate-missing"))?;
    require(
        crate::digest(&leaf.to_der()) == certificate,
        "apple-cms-leaf-certificate-mismatch",
    )?;
    // This authenticated TSA timestamp differs from the signer-supplied signing-time attribute.
    let timestamp = decoder
        .get_signer_timestamp(0)
        .map_err(|_| invalid("apple-authenticated-timestamp-missing-or-invalid"))?
        + 978_307_200.0;
    require(
        timestamp.is_finite() && timestamp >= 1.0 && timestamp <= sigstore::wall_time()? as f64,
        "apple-authenticated-timestamp-future-or-invalid",
    )?;
    let notarized_requirement = expected + " and notarized";
    // The requirement interpreter consults the local ticket store. The documented codesign
    // option first requests an online lookup; its success alone never grants this proof.
    identity(binary).matches(&io::read_bounded(path, MAX_BINARY as u64)?)?;
    check_online_notarization(path, &notarized_requirement)?;
    identity(binary).matches(&io::read_bounded(path, MAX_BINARY as u64)?)?;
    let notarized = SecRequirement::from_str(&notarized_requirement)
        .map_err(|_| invalid("apple-notarization-requirement-unsupported"))?;
    code.check_validity(flags, &notarized)
        .map_err(|_| invalid("apple-notarization-ticket-missing-or-invalid"))?;
    identity(binary).matches(&io::read_bounded(path, MAX_BINARY as u64)?)?;
    Ok(timestamp.floor() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    #[cfg(target_os = "macos")]
    use std::fs;
    use std::io::Write;

    /// Construct a synthetic canonical archive; its successful parse is never a native proof.
    fn archive(tar: &[u8]) -> Vec<u8> {
        let mut writer = GzEncoder::new(Vec::new(), Compression::default());
        writer.write_all(tar).unwrap();
        writer.finish().unwrap()
    }

    /// Write checksum-consistent headers so unsafe-structure tests reach their semantic gate.
    fn checksum(tar: &mut [u8]) {
        tar[148..156].fill(b' ');
        let checksum = tar[..512]
            .iter()
            .map(|byte| usize::from(*byte))
            .sum::<usize>();
        tar[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());
    }

    /// Build one regular named leaf with zero ownership, fixed mode and inert fixture data.
    fn tar() -> Vec<u8> {
        let mut tar = vec![0; 10240];
        tar[..7].copy_from_slice(b"fixture");
        tar[100..108].copy_from_slice(b"0000755\0");
        tar[108..116].copy_from_slice(b"0000000\0");
        tar[116..124].copy_from_slice(b"0000000\0");
        tar[124..136].copy_from_slice(b"00000000005\0");
        tar[136..148].copy_from_slice(b"00000000000\0");
        tar[156] = b'0';
        tar[257..265].copy_from_slice(b"ustar\0\x30\x30");
        tar[512..517].copy_from_slice(b"inert");
        checksum(&mut tar);
        tar
    }

    #[test]
    /// Reject unsafe tar member forms even with correct checksums and gzip envelopes.
    fn package_parser_rejects_links_paths_extra_members_and_metadata() {
        let valid = tar();
        assert_eq!(unpack(&archive(&valid), "fixture").unwrap(), b"inert");
        assert!(unpack(&archive(&valid), "other").is_err());
        assert!(unpack(&archive(&valid), "../fixture").is_err());
        for offset in [0, 99, 108, 116, 136, 157, 265, 329, 345, 500, 1024] {
            let mut changed = valid.clone();
            changed[offset] = b'1';
            checksum(&mut changed);
            assert!(
                unpack(&archive(&changed), "fixture").is_err(),
                "offset {offset}"
            );
        }
        for kind in [
            b'1', b'2', b'3', b'4', b'5', b'6', b'x', b'g', b'L', b'K', 0,
        ] {
            let mut changed = valid.clone();
            changed[156] = kind;
            checksum(&mut changed);
            assert!(
                unpack(&archive(&changed), "fixture").is_err(),
                "type {kind}"
            );
        }
        let mut changed = valid.clone();
        changed[100..108].copy_from_slice(b"0000777\0");
        checksum(&mut changed);
        assert!(unpack(&archive(&changed), "fixture").is_err());
        let mut changed = valid.clone();
        changed[124..136].copy_from_slice(b"00000000000\0");
        checksum(&mut changed);
        assert!(unpack(&archive(&changed), "fixture").is_err());
        let mut changed = valid.clone();
        changed[124] = 128;
        checksum(&mut changed);
        assert!(unpack(&archive(&changed), "fixture").is_err());
        let mut changed = valid.clone();
        changed[148] ^= 1;
        assert!(unpack(&archive(&changed), "fixture").is_err());
    }

    #[test]
    /// Reject appended gzip streams, trailer junk, corrupt CRC and truncated packages.
    fn gzip_parser_requires_one_complete_member_and_checked_trailer() {
        let valid = archive(&tar());
        assert!(unpack(&valid, "fixture").is_ok());
        for suffix in [vec![0], vec![1, 2, 3], valid.clone()] {
            let mut changed = valid.clone();
            changed.extend(suffix);
            assert!(unpack(&changed, "fixture").is_err());
        }
        for length in [0, 1, 10, valid.len() / 2, valid.len() - 1] {
            assert!(unpack(&valid[..length], "fixture").is_err());
        }
        let mut changed = valid.clone();
        let end = changed.len();
        changed[end - 8] ^= 1;
        assert!(unpack(&changed, "fixture").is_err());
        assert!(unpack(&archive(&tar()[..1024]), "fixture").is_err());
    }

    /// Write one synthetic little-endian Mach-O word without reading fixture code.
    fn little(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// Write one synthetic big-endian signing-blob word without native signing authority.
    fn big(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    /// Construct a parser-only Mach-O fixture whose fake CMS can never pass Security.framework.
    fn macho() -> Vec<u8> {
        let mut bytes = vec![0; 512];
        little(&mut bytes, 0, 0xfeedfacf);
        little(&mut bytes, 4, 0x0100000c);
        little(&mut bytes, 12, 2);
        little(&mut bytes, 16, 1);
        little(&mut bytes, 20, 16);
        little(&mut bytes, 32, 0x1d);
        little(&mut bytes, 36, 16);
        little(&mut bytes, 40, 256);
        little(&mut bytes, 44, 256);
        big(&mut bytes, 256, 0xfade0cc0);
        big(&mut bytes, 260, 209);
        big(&mut bytes, 264, 2);
        big(&mut bytes, 268, 0);
        big(&mut bytes, 272, 32);
        big(&mut bytes, 276, 0x10000);
        big(&mut bytes, 280, 200);
        big(&mut bytes, 288, 0xfade0c02);
        big(&mut bytes, 292, 128);
        big(&mut bytes, 296, 0x20400);
        big(&mut bytes, 300, 0x10000);
        bytes[324] = 32;
        bytes[325] = 2;
        big(&mut bytes, 336, 88);
        bytes[376..386].copy_from_slice(b"FIXTURE123");
        big(&mut bytes, 456, 0xfade0b01);
        big(&mut bytes, 460, 9);
        bytes[464] = 0x30;
        bytes
    }

    #[test]
    /// Reject malformed target, bounds, duplicate slots, missing runtime, missing CMS and wrong team.
    fn signature_parser_rejects_ambiguous_or_unbound_metadata() {
        let valid = macho();
        let parsed = signature(&valid, "FIXTURE123").unwrap();
        assert_eq!(parsed.directory.len(), 128);
        assert_eq!(parsed.cms, &[0x30]);
        assert!(signature(&valid, "OTHER12345").is_err());
        for (offset, value) in [
            (0, 0),
            (4, 0x01000007),
            (12, 6),
            (16, 0),
            (16, 4097),
            (20, u32::MAX),
            (36, 8),
            (40, 20),
            (44, u32::MAX),
        ] {
            let mut changed = valid.clone();
            little(&mut changed, offset, value);
            assert!(
                signature(&changed, "FIXTURE123").is_err(),
                "little {offset} / {value}"
            );
        }
        for (offset, value) in [
            (256, 0),
            (260, u32::MAX),
            (264, 0),
            (264, 65),
            (272, 0),
            (276, 0),
            (276, 0x1000),
            (280, 32),
            (292, u32::MAX),
            (296, 0x20100),
            (300, 0),
            (300, 0x10002),
            (336, u32::MAX),
            (456, 0),
        ] {
            let mut changed = valid.clone();
            big(&mut changed, offset, value);
            assert!(
                signature(&changed, "FIXTURE123").is_err(),
                "big {offset} / {value}"
            );
        }
        for (offset, value) in [(324, 20), (325, 1), (376, b'O'), (386, b'1'), (511, 1)] {
            let mut changed = valid.clone();
            changed[offset] = value;
            assert!(signature(&changed, "FIXTURE123").is_err(), "byte {offset}");
        }
        for length in [0, 8, 31, 47, 257, 464, 511] {
            assert!(signature(&valid[..length], "FIXTURE123").is_err());
        }
    }

    #[test]
    /// Reject team strings capable of extending the native requirement expression.
    fn team_identifiers_cannot_inject_requirements() {
        assert!(valid_team("FIXTURE123"));
        for team in [
            "",
            "fixture123",
            "ABCDEFGHIJK",
            "ABCDEFGHI\"",
            "ABCDE/1234",
            "ABCDE\n1234",
            "ÉBCDE1234",
        ] {
            assert!(!valid_team(team));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    /// Confirm the actual platform can compile the full Developer ID, team and notarization requirement.
    fn platform_supports_the_required_notarization_expression() {
        use security_framework::os::macos::code_signing::SecRequirement;
        use std::str::FromStr;
        assert!(SecRequirement::from_str("anchor apple generic and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = \"FIXTURE123\" and notarized").is_ok());
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires the installed Apple compiler and system codesign; run explicitly in native qualification"]
    /// A genuine hardened ad-hoc signature passes ordinary validation but fails the Developer ID native gate.
    fn real_hardened_adhoc_signature_cannot_satisfy_developer_id() {
        use std::process::{Command, Stdio};
        let directory = tempfile::Builder::new()
            .prefix("armorer-apple-negative-")
            .tempdir()
            .unwrap();
        let source = directory.path().join("owned.c");
        let path = directory.path().join("owned-fixture");
        fs::write(&source, b"int main(void) { return 0; }\n").unwrap();
        let run = |tool: &str, arguments: &[&std::ffi::OsStr]| {
            Command::new(tool)
                .args(arguments)
                .env_clear()
                .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
                .env("HOME", directory.path())
                .env("TMPDIR", directory.path())
                .current_dir(directory.path())
                .stdin(Stdio::null())
                .output()
                .unwrap()
        };
        let compiled = run(
            "/usr/bin/clang",
            &[source.as_os_str(), "-o".as_ref(), path.as_os_str()],
        );
        assert!(compiled.status.success());
        let signed = run(
            "/usr/bin/codesign",
            &[
                "--force".as_ref(),
                "--sign".as_ref(),
                "-".as_ref(),
                "--options".as_ref(),
                "runtime".as_ref(),
                "--timestamp=none".as_ref(),
                path.as_os_str(),
            ],
        );
        assert!(signed.status.success());
        assert!(
            run(
                "/usr/bin/codesign",
                &["--verify".as_ref(), "--strict".as_ref(), path.as_os_str()]
            )
            .status
            .success()
        );
        let metadata = run(
            "/usr/bin/codesign",
            &[
                "--display".as_ref(),
                "--verbose=4".as_ref(),
                path.as_os_str(),
            ],
        );
        assert!(metadata.status.success());
        assert!(String::from_utf8_lossy(&metadata.stderr).contains("adhoc,runtime"));
        assert!(check_online_notarization(&path, "notarized").is_err());
        let bytes = io::read_bounded(&path, MAX_BINARY as u64).unwrap();
        let error = native(&path, &bytes, "FIXTURE123", &"0".repeat(64)).unwrap_err();
        assert_eq!(
            error.to_string(),
            invalid("apple-developer-id-signature-invalid").to_string()
        );
        assert_eq!(io::read_bounded(&path, MAX_BINARY as u64).unwrap(), bytes);
        // The compiled fixture is never executed; '-' uses no Developer ID certificate or keychain secret.
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires the pin-qualified public Apple reference and native ticket lookup; run explicitly in native qualification"]
    /// Accept a genuinely notarized reference and reject wrong team/certificate and tampering without executing it.
    fn real_public_developer_id_reference_and_native_adversaries() {
        let offered = std::env::var_os("ARMORER_TEST_APPLE_REFERENCE")
            .expect("qualified inert Apple reference required");
        let bytes = io::read_bounded(Path::new(&offered), 32 * 1024 * 1024).unwrap();
        assert_eq!(bytes.len(), 11_571_456);
        assert_eq!(
            crate::digest(&bytes),
            "5e7b3040ed2e772715b6fa19f6269b3b45f6705c769e29a6aceeb9ad7bd62caa"
        );
        let directory = tempfile::Builder::new()
            .prefix("armorer-apple-reference-")
            .tempdir()
            .unwrap();
        let path = directory.path().join("payload.macho");
        io::write_readonly(&path, &bytes).unwrap();
        let certificate = "912d20cb0e38fc959b701cbbbfc4aced91d04c4d78c67dd4ac1ef1fcc80f7952";
        assert_eq!(
            native(&path, &bytes, "DVH6X33J83", certificate).unwrap(),
            1_790_809_791
        );
        assert_eq!(
            native(&path, &bytes, "FIXTURE123", certificate)
                .unwrap_err()
                .to_string(),
            invalid("apple-developer-id-signature-invalid").to_string()
        );
        assert_eq!(
            native(&path, &bytes, "DVH6X33J83", &"0".repeat(64))
                .unwrap_err()
                .to_string(),
            invalid("apple-cms-leaf-certificate-mismatch").to_string()
        );
        let mut tampered = bytes.clone();
        tampered[4096] ^= 1;
        let changed = directory.path().join("tampered.macho");
        io::write_readonly(&changed, &tampered).unwrap();
        assert_eq!(
            native(&changed, &tampered, "DVH6X33J83", certificate)
                .unwrap_err()
                .to_string(),
            invalid("apple-developer-id-signature-invalid").to_string()
        );
        assert_eq!(io::read_bounded(&path, MAX_BINARY as u64).unwrap(), bytes);
        assert_eq!(
            io::read_bounded(&changed, MAX_BINARY as u64).unwrap(),
            tampered
        );
    }
}
