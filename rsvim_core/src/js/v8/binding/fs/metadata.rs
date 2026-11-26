//! File metadata.
//!
//! The type and its conversion live in `crate::js::fs`; this module only keeps
//! the path `crate::js::binding::fs::metadata::*` working for existing callers.

pub use crate::js::fs::FsMetadata;
pub use crate::js::fs::FsMetadataBuilder;
pub use crate::js::fs::convert_metadata as convert;
