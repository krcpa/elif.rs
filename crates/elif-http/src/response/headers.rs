//! HTTP header utilities and wrappers

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Framework-native header name wrapper that hides Axum internals
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ElifHeaderName(axum::http::HeaderName);

impl ElifHeaderName {
    /// Create a new header name from a string
    pub fn from_str(name: &str) -> Result<Self, axum::http::header::InvalidHeaderName> {
        axum::http::HeaderName::from_str(name).map(Self)
    }

    /// Get header name as string
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Internal method to convert to axum HeaderName (for framework internals only)
    pub(crate) fn to_axum(&self) -> &axum::http::HeaderName {
        &self.0
    }

    /// Internal method to create from axum HeaderName (for framework internals only)
    pub(crate) fn from_axum(name: axum::http::HeaderName) -> Self {
        Self(name)
    }
}

impl FromStr for ElifHeaderName {
    type Err = axum::http::header::InvalidHeaderName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s)
    }
}

impl fmt::Display for ElifHeaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Framework-native header value wrapper that hides Axum internals
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElifHeaderValue(axum::http::HeaderValue);

impl ElifHeaderValue {
    /// Create a new header value from a string
    pub fn from_str(value: &str) -> Result<Self, axum::http::header::InvalidHeaderValue> {
        axum::http::HeaderValue::from_str(value).map(Self)
    }

    /// Create a new header value from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, axum::http::header::InvalidHeaderValue> {
        axum::http::HeaderValue::from_bytes(bytes).map(Self)
    }

    /// Get header value as string
    pub fn to_str(&self) -> Result<&str, axum::http::header::ToStrError> {
        self.0.to_str()
    }

    /// Get header value as bytes
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Internal method to convert to axum HeaderValue (for framework internals only)
    pub(crate) fn to_axum(&self) -> &axum::http::HeaderValue {
        &self.0
    }

    /// Internal method to create from axum HeaderValue (for framework internals only)
    pub(crate) fn from_axum(value: axum::http::HeaderValue) -> Self {
        Self(value)
    }
}

impl FromStr for ElifHeaderValue {
    type Err = axum::http::header::InvalidHeaderValue;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s)
    }
}

impl fmt::Display for ElifHeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_str() {
            Ok(s) => write!(f, "{}", s),
            Err(_) => write!(f, "<invalid UTF-8>"),
        }
    }
}

/// Framework-native header map wrapper that hides Axum internals
#[derive(Debug, Clone)]
pub struct ElifHeaderMap(axum::http::HeaderMap);

impl ElifHeaderMap {
    /// Create a new empty header map
    pub fn new() -> Self {
        Self(axum::http::HeaderMap::new())
    }

    /// Insert a header into the map
    pub fn insert(&mut self, name: ElifHeaderName, value: ElifHeaderValue) -> Option<ElifHeaderValue> {
        self.0.insert(name.0, value.0).map(ElifHeaderValue)
    }

    /// Get a header value by name
    pub fn get(&self, name: &ElifHeaderName) -> Option<&ElifHeaderValue> {
        // SAFETY: This is safe because ElifHeaderValue is a transparent wrapper around HeaderValue
        unsafe { std::mem::transmute(self.0.get(&name.0)) }
    }

    /// Get a header value by string name
    pub fn get_str(&self, name: &str) -> Option<&ElifHeaderValue> {
        if let Ok(header_name) = ElifHeaderName::from_str(name) {
            self.get(&header_name)
        } else {
            None
        }
    }

    /// Remove a header from the map
    pub fn remove(&mut self, name: &ElifHeaderName) -> Option<ElifHeaderValue> {
        self.0.remove(&name.0).map(ElifHeaderValue)
    }

    /// Check if the map contains a header
    pub fn contains_key(&self, name: &ElifHeaderName) -> bool {
        self.0.contains_key(&name.0)
    }

    /// Check if the map contains a header by string name
    pub fn contains_key_str(&self, name: &str) -> bool {
        if let Ok(header_name) = ElifHeaderName::from_str(name) {
            self.contains_key(&header_name)
        } else {
            false
        }
    }

    /// Get the number of headers
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the header map is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Clear all headers
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Iterate over all headers
    pub fn iter(&self) -> impl Iterator<Item = (&ElifHeaderName, &ElifHeaderValue)> {
        self.0.iter().map(|(k, v)| {
            // SAFETY: This is safe because our wrapper types are transparent
            unsafe { 
                (std::mem::transmute(k), std::mem::transmute(v))
            }
        })
    }

    /// Convert to a HashMap for easier manipulation
    pub fn to_hash_map(&self) -> HashMap<String, String> {
        self.0
            .iter()
            .filter_map(|(k, v)| {
                v.to_str().ok().map(|v| (k.as_str().to_string(), v.to_string()))
            })
            .collect()
    }

    /// Internal method to convert to axum HeaderMap (for framework internals only)
    pub(crate) fn to_axum(&self) -> &axum::http::HeaderMap {
        &self.0
    }

    /// Internal method to create from axum HeaderMap (for framework internals only)
    pub(crate) fn from_axum(headers: axum::http::HeaderMap) -> Self {
        Self(headers)
    }
}

impl Default for ElifHeaderMap {
    fn default() -> Self {
        Self::new()
    }
}

impl From<axum::http::HeaderMap> for ElifHeaderMap {
    fn from(headers: axum::http::HeaderMap) -> Self {
        Self::from_axum(headers)
    }
}

// Common header name constants
pub mod header_names {
    use super::ElifHeaderName;
    use axum::http::header;

    // Define headers using string constants rather than axum header constants
    // to avoid type mismatch issues
    pub const AUTHORIZATION: ElifHeaderName = ElifHeaderName(header::AUTHORIZATION);
    pub const CONTENT_TYPE: ElifHeaderName = ElifHeaderName(header::CONTENT_TYPE);
    pub const CONTENT_LENGTH: ElifHeaderName = ElifHeaderName(header::CONTENT_LENGTH);
    pub const ACCEPT: ElifHeaderName = ElifHeaderName(header::ACCEPT);
    pub const CACHE_CONTROL: ElifHeaderName = ElifHeaderName(header::CACHE_CONTROL);
    pub const ETAG: ElifHeaderName = ElifHeaderName(header::ETAG);
    pub const IF_NONE_MATCH: ElifHeaderName = ElifHeaderName(header::IF_NONE_MATCH);
    pub const LOCATION: ElifHeaderName = ElifHeaderName(header::LOCATION);
    pub const SET_COOKIE: ElifHeaderName = ElifHeaderName(header::SET_COOKIE);
    pub const COOKIE: ElifHeaderName = ElifHeaderName(header::COOKIE);
    pub const USER_AGENT: ElifHeaderName = ElifHeaderName(header::USER_AGENT);
    pub const REFERER: ElifHeaderName = ElifHeaderName(header::REFERER);
    pub const ORIGIN: ElifHeaderName = ElifHeaderName(header::ORIGIN);
    pub const ACCESS_CONTROL_ALLOW_ORIGIN: ElifHeaderName = ElifHeaderName(header::ACCESS_CONTROL_ALLOW_ORIGIN);
    pub const ACCESS_CONTROL_ALLOW_METHODS: ElifHeaderName = ElifHeaderName(header::ACCESS_CONTROL_ALLOW_METHODS);
    pub const ACCESS_CONTROL_ALLOW_HEADERS: ElifHeaderName = ElifHeaderName(header::ACCESS_CONTROL_ALLOW_HEADERS);
    pub const ACCESS_CONTROL_EXPOSE_HEADERS: ElifHeaderName = ElifHeaderName(header::ACCESS_CONTROL_EXPOSE_HEADERS);
    pub const ACCESS_CONTROL_ALLOW_CREDENTIALS: ElifHeaderName = ElifHeaderName(header::ACCESS_CONTROL_ALLOW_CREDENTIALS);
    pub const ACCESS_CONTROL_MAX_AGE: ElifHeaderName = ElifHeaderName(header::ACCESS_CONTROL_MAX_AGE);
    pub const CONTENT_SECURITY_POLICY: ElifHeaderName = ElifHeaderName(header::CONTENT_SECURITY_POLICY);
    pub const STRICT_TRANSPORT_SECURITY: ElifHeaderName = ElifHeaderName(header::STRICT_TRANSPORT_SECURITY);
    pub const X_FRAME_OPTIONS: ElifHeaderName = ElifHeaderName(header::X_FRAME_OPTIONS);
    pub const X_CONTENT_TYPE_OPTIONS: ElifHeaderName = ElifHeaderName(header::X_CONTENT_TYPE_OPTIONS);
    pub const X_XSS_PROTECTION: ElifHeaderName = ElifHeaderName(header::X_XSS_PROTECTION);
    pub const REFERRER_POLICY: ElifHeaderName = ElifHeaderName(header::REFERRER_POLICY);
}