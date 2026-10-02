//! Bounded, regular-file snapshots. No downloaded file is executed or extracted.
use crate::{
    Error, Result,
    trust::{ByteIdentity, require},
};
use serde::de::{DeserializeOwned, DeserializeSeed, Visitor};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

pub(crate) const MAX_BUNDLE: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_PAYLOAD: usize = 17 * 1024 * 1024;
pub(crate) const MAX_ROOT: u64 = 4 * 1024 * 1024;
pub(crate) const MAX_VERIFIER: u64 = 128 * 1024 * 1024;

pub(crate) fn regular(path: &Path, max: u64) -> Result<File> {
    let before = fs::symlink_metadata(path)?;
    require(
        before.is_file() && before.len() <= max,
        "verification-input-type-or-size",
    )?;
    let file = File::open(path)?;
    let opened = file.metadata()?;
    require(
        opened.is_file() && opened.len() == before.len(),
        "verification-input-changed",
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        require(
            before.dev() == opened.dev() && before.ino() == opened.ino(),
            "verification-input-changed",
        )?;
    }
    Ok(file)
}

pub(crate) fn read_bounded(path: &Path, max: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    regular(path, max)?.take(max + 1).read_to_end(&mut bytes)?;
    require(bytes.len() as u64 <= max, "verification-input-size-limit")?;
    Ok(bytes)
}

// Byte bounds alone do not bound the allocation cost of tiny JSON values.
const MAX_NODES: usize = 262_144;
const MAX_DEPTH: usize = 64;
struct Seed<'a> {
    remaining: &'a mut usize,
    depth: usize,
}
impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = serde_json::Value;
    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        decoder: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        if *self.remaining == 0 || self.depth >= MAX_DEPTH {
            return Err(serde::de::Error::custom(
                "verification JSON structure limit",
            ));
        }
        *self.remaining -= 1;
        decoder.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Seed<'_> {
    type Value = serde_json::Value;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("bounded JSON without duplicate keys")
    }
    fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
        Ok(v.into())
    }
    fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
        Ok(v.into())
    }
    fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
        Ok(v.into())
    }
    fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
        serde_json::Number::from_f64(v)
            .map(Into::into)
            .ok_or_else(|| E::custom("invalid JSON number"))
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
        Ok(v.into())
    }
    fn visit_string<E: serde::de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
        Ok(v.into())
    }
    fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        mut access: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = access.next_element_seed(Seed {
            remaining: self.remaining,
            depth: self.depth + 1,
        })? {
            values.push(value);
        }
        Ok(values.into())
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        mut access: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        let mut values = serde_json::Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(serde::de::Error::custom("duplicate JSON key"));
            }
            let value = access.next_value_seed(Seed {
                remaining: self.remaining,
                depth: self.depth + 1,
            })?;
            values.insert(key, value);
        }
        Ok(values.into())
    }
}
pub(crate) fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let mut remaining = MAX_NODES;
    let value = Seed {
        remaining: &mut remaining,
        depth: 0,
    }
    .deserialize(&mut decoder)
    .map_err(|_| Error::Json)?;
    decoder.end().map_err(|_| Error::Json)?;
    serde_json::from_value(value).map_err(|_| Error::Json)
}

pub(crate) fn snapshot(source: &Path, destination: &Path, max: u64) -> Result<ByteIdentity> {
    let mut input = regular(source, max)?.take(max + 1);
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)?;
    let mut hash = Sha256::new();
    let mut size = 0_u64;
    let mut block = [0_u8; 65536];
    loop {
        let count = input.read(&mut block)?;
        if count == 0 {
            break;
        }
        size += count as u64;
        require(size <= max, "verification-input-size-limit")?;
        hash.update(&block[..count]);
        output.write_all(&block[..count])?;
    }
    output.sync_all()?;
    let identity = ByteIdentity {
        sha256: format!("{:x}", hash.finalize()),
        size,
    };
    identity.validate()?;
    Ok(identity)
}

pub(crate) fn identity(path: &Path, max: u64) -> Result<ByteIdentity> {
    let mut input = regular(path, max)?.take(max + 1);
    let mut hash = Sha256::new();
    let mut size = 0_u64;
    let mut block = [0_u8; 65536];
    loop {
        let count = input.read(&mut block)?;
        if count == 0 {
            break;
        }
        size += count as u64;
        require(size <= max, "verification-input-size-limit")?;
        hash.update(&block[..count]);
    }
    let value = ByteIdentity {
        sha256: format!("{:x}", hash.finalize()),
        size,
    };
    value.validate()?;
    Ok(value)
}

pub(crate) fn write_readonly(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    readonly(path, false)
}

pub(crate) fn readonly(path: &Path, executable: bool) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if executable { 0o500 } else { 0o400 }),
        )?;
    }
    #[cfg(not(unix))]
    {
        let _ = (path, executable);
        return Err(Error::Invalid("unsupported-verifier-platform".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn untrusted_json_has_node_depth_duplicate_and_trailing_data_bounds() {
        assert!(parse::<serde_json::Value>(b"{\"a\":{\"b\":1,\"b\":2}}").is_err());
        assert!(parse::<serde_json::Value>(b"{} {} ").is_err());
        let nested = format!("{}null{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
        assert!(parse::<serde_json::Value>(nested.as_bytes()).is_err());
        let nodes = format!("[{}]", vec!["null"; MAX_NODES].join(","));
        assert!(parse::<serde_json::Value>(nodes.as_bytes()).is_err());
        assert_eq!(
            parse::<serde_json::Value>(b"{\"a\":[true,1,1.5,null]}").unwrap(),
            serde_json::json!({"a":[true,1,1.5,null]})
        );
    }
}
