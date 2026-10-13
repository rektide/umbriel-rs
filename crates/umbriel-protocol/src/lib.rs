//! Sans-I/O building blocks for the Umbriel compositor's wire protocol.
#![forbid(unsafe_code)]

/// Maximum outgoing request size, including the terminating LF.
pub const REQUEST_LIMIT: usize = 65_536;
