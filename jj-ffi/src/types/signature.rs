//! Signature and timestamp types for FFI

use jj_lib::backend::{MillisSinceEpoch, Signature, Timestamp};

/// A timestamp for FFI
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiTimestamp {
    /// Milliseconds since Unix epoch
    pub millis_since_epoch: i64,
    /// Timezone offset in minutes from UTC
    pub tz_offset_minutes: i32,
}

impl From<&Timestamp> for FfiTimestamp {
    fn from(ts: &Timestamp) -> Self {
        Self {
            millis_since_epoch: ts.timestamp.0,
            tz_offset_minutes: ts.tz_offset,
        }
    }
}

impl From<Timestamp> for FfiTimestamp {
    fn from(ts: Timestamp) -> Self {
        Self::from(&ts)
    }
}

impl From<&FfiTimestamp> for Timestamp {
    fn from(ffi_ts: &FfiTimestamp) -> Self {
        Self {
            timestamp: MillisSinceEpoch(ffi_ts.millis_since_epoch),
            tz_offset: ffi_ts.tz_offset_minutes,
        }
    }
}

impl From<FfiTimestamp> for Timestamp {
    fn from(ffi_ts: FfiTimestamp) -> Self {
        Self::from(&ffi_ts)
    }
}

/// A commit signature (author or committer) for FFI
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiSignature {
    /// Name of the person
    pub name: String,
    /// Email address
    pub email: String,
    /// Timestamp of the signature
    pub timestamp: FfiTimestamp,
}

impl From<&Signature> for FfiSignature {
    fn from(sig: &Signature) -> Self {
        Self {
            name: sig.name.clone(),
            email: sig.email.clone(),
            timestamp: FfiTimestamp::from(&sig.timestamp),
        }
    }
}

impl From<Signature> for FfiSignature {
    fn from(sig: Signature) -> Self {
        Self::from(&sig)
    }
}

impl From<&FfiSignature> for Signature {
    fn from(ffi_sig: &FfiSignature) -> Self {
        Self {
            name: ffi_sig.name.clone(),
            email: ffi_sig.email.clone(),
            timestamp: Timestamp::from(&ffi_sig.timestamp),
        }
    }
}

impl From<FfiSignature> for Signature {
    fn from(ffi_sig: FfiSignature) -> Self {
        Self::from(&ffi_sig)
    }
}
