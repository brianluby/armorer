//! Decode only an independently approved canonical runtime distribution, never release payloads.
use super::{context::TrustedReleaseContext, io, sigstore};
use crate::{
    Result,
    trust::{
        ByteIdentity,
        native::{MAX_NATIVE_MEMBER, NativeTarget, RuntimeDistributionV1},
        require,
    },
};
use sha2::{Digest, Sha256};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tempfile::TempDir;

const BLOCK: u64 = 512;
const RECORD: u64 = 10240;
const MAX_DISTRIBUTION: u64 = 3 * MAX_NATIVE_MEMBER + 2 * RECORD;

/// A private snapshot borrows the approved authority and cannot be manufactured from JSON.
/// ```compile_fail
/// let runtime: armorer::verification::runtime::ApprovedRuntimeFiles = serde_json::from_str("{}").unwrap();
/// ```
pub struct ApprovedRuntimeFiles<'context> {
    context: &'context TrustedReleaseContext,
    _directory: TempDir,
    executable: PathBuf,
    executable_identity: ByteIdentity,
    distribution_identity: ByteIdentity,
}
impl<'context> ApprovedRuntimeFiles<'context> {
    /// Rehash approved archive bytes before parsing; validate every member before exposing one native executable.
    /// No command, network access, consuming build script or release archive is executed or extracted.
    pub fn open(archive: &Path, context: &'context TrustedReleaseContext) -> Result<Self> {
        context.validate_at(sigstore::wall_time()?)?;
        let spec = context.runtime_distribution()?;
        let native = NativeTarget::current()?;
        require(
            spec.distribution.size <= MAX_DISTRIBUTION,
            "runtime-distribution-size-limit",
        )?;
        let started = Instant::now();
        let directory = tempfile::tempdir()?;
        let snapshot = directory.path().join("runtime.tar");
        let distribution_identity = io::snapshot(archive, &snapshot, MAX_DISTRIBUTION)?;
        require(
            distribution_identity == spec.distribution,
            "runtime-distribution-byte-mismatch",
        )?;
        io::readonly(&snapshot, false)?;
        let executable = directory.path().join(native.runtime_member());
        decode(&snapshot, spec, native, &executable, started)?;
        context.validate_at(sigstore::wall_time()?)?;
        let executable_identity = spec
            .members
            .get(&native)
            .ok_or_else(|| crate::Error::Invalid("runtime-member-missing".into()))?
            .bytes
            .clone();
        require(
            io::identity(&executable, MAX_NATIVE_MEMBER)? == executable_identity,
            "runtime-executable-byte-mismatch",
        )?;
        io::readonly(&executable, true)?;
        Ok(Self {
            context,
            _directory: directory,
            executable,
            executable_identity,
            distribution_identity,
        })
    }
    /// Recheck current approval and retained bytes; callers choose fixed reviewed invocation separately.
    pub fn native_executable(&self) -> Result<&Path> {
        self.context.validate_at(sigstore::wall_time()?)?;
        require(
            io::identity(&self.executable, MAX_NATIVE_MEMBER)? == self.executable_identity,
            "runtime-executable-byte-mismatch",
        )?;
        Ok(&self.executable)
    }
    /// Return the actual approved archive identity common to all release selections.
    pub fn distribution_identity(&self) -> &ByteIdentity {
        &self.distribution_identity
    }
}

/// Construct the one accepted POSIX USTAR header, avoiding permissive numeric/path interpretations.
fn canonical_header(name: &str, size: u64) -> [u8; 512] {
    let mut header = [0_u8; 512];
    header[..name.len()].copy_from_slice(name.as_bytes());
    header[100..108].copy_from_slice(b"0000500\0");
    header[108..116].copy_from_slice(b"0000000\0");
    header[116..124].copy_from_slice(b"0000000\0");
    header[124..136].copy_from_slice(format!("{size:011o}\0").as_bytes());
    header[136..148].copy_from_slice(b"00000000000\0");
    header[148..156].fill(b' ');
    header[156] = b'0';
    header[257..265].copy_from_slice(b"ustar\x0000");
    let sum: u64 = header.iter().map(|byte| u64::from(*byte)).sum();
    header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    header
}

/// Reject foreign architectures and scripts; digest approval remains the executable trust authority.
fn native_header(prefix: &[u8; 64], target: NativeTarget) -> Result<()> {
    let valid = match target {
        NativeTarget::LinuxX86 | NativeTarget::LinuxArm => {
            let machine = if target == NativeTarget::LinuxX86 {
                62_u16
            } else {
                183
            };
            prefix[..7] == *b"\x7fELF\x02\x01\x01"
                && matches!(u16::from_le_bytes([prefix[16], prefix[17]]), 2 | 3)
                && u16::from_le_bytes([prefix[18], prefix[19]]) == machine
                && prefix[20..24] == [1, 0, 0, 0]
        }
        NativeTarget::MacArm => {
            prefix[..4] == [0xcf, 0xfa, 0xed, 0xfe]
                && prefix[4..8] == [12, 0, 0, 1]
                && prefix[12..16] == [2, 0, 0, 0]
        }
    };
    require(valid, "runtime-native-header-mismatch")
}

/// Check stage duration without returning partial or executable output on timeout.
fn deadline(started: Instant) -> Result<()> {
    require(
        started.elapsed() <= Duration::from_secs(20 * 60),
        "runtime-stage-timeout",
    )
}

/// Hash all three fixed members and retain only the current host member in a private directory.
fn decode(
    archive: &Path,
    spec: &RuntimeDistributionV1,
    native: NativeTarget,
    output: &Path,
    started: Instant,
) -> Result<()> {
    let mut reader = io::regular(archive, MAX_DISTRIBUTION)?;
    let mut written = 0_u64;
    for target in [
        NativeTarget::LinuxX86,
        NativeTarget::LinuxArm,
        NativeTarget::MacArm,
    ] {
        deadline(started)?;
        let member = spec
            .members
            .get(&target)
            .ok_or_else(|| crate::Error::Invalid("runtime-member-missing".into()))?;
        require(
            member.name == target.runtime_member()
                && (64..=MAX_NATIVE_MEMBER).contains(&member.bytes.size),
            "invalid-runtime-member",
        )?;
        let mut header = [0_u8; 512];
        reader.read_exact(&mut header)?;
        require(
            header == canonical_header(&member.name, member.bytes.size),
            "runtime-ustar-header-mismatch",
        )?;
        written += BLOCK;
        let mut destination = if target == native {
            Some(
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(output)?,
            )
        } else {
            None
        };
        let mut hash = Sha256::new();
        let mut remaining = member.bytes.size;
        let mut buffer = [0_u8; 65536];
        let mut prefix = [0_u8; 64];
        while remaining > 0 {
            deadline(started)?;
            let count = remaining.min(buffer.len() as u64) as usize;
            reader.read_exact(&mut buffer[..count])?;
            if remaining == member.bytes.size {
                prefix.copy_from_slice(&buffer[..64]);
            }
            hash.update(&buffer[..count]);
            if let Some(file) = &mut destination {
                file.write_all(&buffer[..count])?;
            }
            remaining -= count as u64;
        }
        require(
            format!("{:x}", hash.finalize()) == member.bytes.sha256,
            "runtime-member-byte-mismatch",
        )?;
        native_header(&prefix, target)?;
        if let Some(file) = destination {
            file.sync_all()?;
            io::readonly(output, false)?;
        }
        let padding = (BLOCK - member.bytes.size % BLOCK) % BLOCK;
        let mut zeros = [0_u8; 512];
        reader.read_exact(&mut zeros[..padding as usize])?;
        require(
            zeros[..padding as usize].iter().all(|byte| *byte == 0),
            "runtime-ustar-padding-mismatch",
        )?;
        written += member.bytes.size + padding;
    }
    let total = (written + 2 * BLOCK).div_ceil(RECORD) * RECORD;
    require(
        total == spec.distribution.size,
        "runtime-ustar-size-mismatch",
    )?;
    let mut tail = vec![0_u8; (total - written) as usize];
    reader.read_exact(&mut tail)?;
    require(
        tail.iter().all(|byte| *byte == 0),
        "runtime-ustar-end-mismatch",
    )?;
    require(
        reader.read(&mut [0_u8; 1])? == 0,
        "runtime-ustar-trailing-data",
    )?;
    deadline(started)
}
