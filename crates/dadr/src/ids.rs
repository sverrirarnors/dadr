use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::{Uuid, Variant};

use crate::error::DadrError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecordId(Uuid);

impl RecordId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(input: &str) -> Result<Self, DadrError> {
        let uuid =
            Uuid::parse_str(input).map_err(|_| DadrError::InvalidRecordId(input.to_owned()))?;
        let record_id = Self(uuid);
        if !record_id.is_uuid_v7() {
            return Err(DadrError::InvalidRecordId(input.to_owned()));
        }
        Ok(record_id)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    pub fn is_uuid_v7(&self) -> bool {
        self.0.get_version_num() == 7 && self.0.get_variant() == Variant::RFC4122
    }

    /// The first 12 hex digits of the id: the record's short id.
    pub fn short_id(&self) -> String {
        self.0.simple().to_string().chars().take(12).collect()
    }

    pub fn timestamp_millis(&self) -> u64 {
        let bytes = self.0.as_bytes();
        ((bytes[0] as u64) << 40)
            | ((bytes[1] as u64) << 32)
            | ((bytes[2] as u64) << 24)
            | ((bytes[3] as u64) << 16)
            | ((bytes[4] as u64) << 8)
            | bytes[5] as u64
    }

    pub fn timestamp_utc(&self) -> Option<DateTime<Utc>> {
        let millis = i64::try_from(self.timestamp_millis()).ok()?;
        DateTime::<Utc>::from_timestamp_millis(millis)
    }
}

impl fmt::Display for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.hyphenated())
    }
}

impl FromStr for RecordId {
    type Err = DadrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecordIdPrefix(String);

impl RecordIdPrefix {
    pub fn parse_hex(input: &str) -> Result<Self, DadrError> {
        let normalized = input.trim().to_ascii_lowercase();
        if normalized.len() < 12
            || normalized.len() > 32
            || normalized.contains('-')
            || !normalized.chars().all(|ch| ch.is_ascii_hexdigit())
        {
            return Err(DadrError::InvalidRecordId(input.to_owned()));
        }
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RecordIdPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The target of a record link: a short id or a full hyphenated UUID.
///
/// Any other target is not a DADR record link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordRef {
    ShortId(String),
    Full(Uuid),
}

impl RecordRef {
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        if input.len() == SHORT_ID_LEN && input.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Some(Self::ShortId(input.to_ascii_lowercase()));
        }
        if input.len() == FULL_ID_LEN {
            return Uuid::try_parse(input).ok().map(Self::Full);
        }
        None
    }

    pub fn matches(&self, id: RecordId) -> bool {
        match self {
            Self::ShortId(short_id) => id.short_id() == *short_id,
            Self::Full(uuid) => id.as_uuid() == *uuid,
        }
    }
}

impl fmt::Display for RecordRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShortId(short_id) => f.write_str(short_id),
            Self::Full(uuid) => write!(f, "{}", uuid.hyphenated()),
        }
    }
}

const SHORT_ID_LEN: usize = 12;
const FULL_ID_LEN: usize = 36;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_uuidv7() {
        let id = RecordId::parse("01938b8e-4d70-7abc-8def-0123456789ab").unwrap();
        assert_eq!(id.short_id(), "01938b8e4d70");
        assert!(id.is_uuid_v7());
    }

    #[test]
    fn normalizes_uppercase_uuid() {
        let id = RecordId::parse("01938B8E-4D70-7ABC-8DEF-0123456789AB").unwrap();
        assert_eq!(id.to_string(), "01938b8e-4d70-7abc-8def-0123456789ab");
    }

    #[test]
    fn rejects_non_v7_uuid() {
        let err = RecordId::parse("01938b8e-4d70-4abc-8def-0123456789ab").unwrap_err();
        assert!(matches!(err, DadrError::InvalidRecordId(_)));
    }

    #[test]
    fn rejects_v7_with_invalid_variant() {
        let err = RecordId::parse("01938b8e-4d70-7abc-0def-0123456789ab").unwrap_err();
        assert!(matches!(err, DadrError::InvalidRecordId(_)));
    }

    #[test]
    fn parses_prefix() {
        let prefix = RecordIdPrefix::parse_hex("01938B8E4D70").unwrap();
        assert_eq!(prefix.as_str(), "01938b8e4d70");
    }

    #[test]
    fn rejects_short_or_hyphenated_prefix() {
        assert!(RecordIdPrefix::parse_hex("01938b8e4d7").is_err());
        assert!(RecordIdPrefix::parse_hex("01938b8e-4d70").is_err());
    }

    #[test]
    fn record_ref_accepts_only_short_or_full_ids() {
        let id = RecordId::parse("01938b8e-4d70-7abc-8def-0123456789ab").unwrap();
        assert!(RecordRef::parse("01938b8e4d70").unwrap().matches(id));
        assert!(RecordRef::parse("01938B8E4D70").unwrap().matches(id));
        assert!(RecordRef::parse("01938b8e-4d70-7abc-8def-0123456789ab")
            .unwrap()
            .matches(id));
        assert_eq!(RecordRef::parse("cafe"), None);
        assert_eq!(RecordRef::parse("01938b8e4d7"), None);
        assert_eq!(RecordRef::parse("01938b8e4d70a"), None);
        assert_eq!(RecordRef::parse("01938b8e4d707abc8def0123456789ab"), None);
        assert_eq!(RecordRef::parse("Some note"), None);
    }

    #[test]
    fn uuidv7_timestamp_conversion_is_non_panicking() {
        let id = RecordId::parse("ffffffff-ffff-7abc-8def-0123456789ab").unwrap();
        assert!(id.timestamp_utc().is_some());
    }
}
