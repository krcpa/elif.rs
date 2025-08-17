//! Storage backend implementations

pub mod local;

#[cfg(feature = "aws-s3")]
pub mod s3;

pub use local::*;

#[cfg(feature = "aws-s3")]
pub use s3::*;