use std::{io, string};
use regex;

/// An error indicating that there was a failure to start the `Provider`.
#[derive(Debug, Fail)]
#[fail(display = "Failed to start LCM provider.")]
pub struct ProviderStartError {
    #[cause]
    io_error: io::Error,
}

impl From<io::Error> for ProviderStartError {
    fn from(io_error: io::Error) -> Self {
        ProviderStartError { io_error }
    }
}

/// An error indicating that an attempt to subscribe to a topic was
/// unsuccessful.
#[derive(Debug, Fail)]
pub enum SubscribeError {
    /// The provided string was an invalid regular expression.
    #[fail(display = "Invalid regular expression used to subscribe to channel: \"{}\".", channel)]
    InvalidRegex {
        channel: String,

        #[cause]
        regex_error: regex::Error,
    },

    /// The provider is no longer active.
    #[fail(display = "The provider is no longer active.")]
    MissingProvider,
}

/// Indicates that an error occured while trying to handle messages.
#[derive(Debug, Fail)]
pub enum HandleError {
    /// The provider returned an error.
    #[fail(display = "Provider returned an error.")]
    ProviderError {
        #[cause]
        io_error: io::Error,
    },

    /// The provider is no longer active.
    #[fail(display = "The provider is no longer active.")]
    MissingProvider,
}

/// An error that happens while trying to encode a message.
#[derive(Debug, Fail)]
pub enum EncodeError {
    #[fail(display = "The size of array does not match the size specified in {}. Expected {}, found {}.", size_var, expected, found)]
    SizeMismatch {
        /// The field specifying the size of the array.
        size_var: String,

        /// The size the array was expected to be.
        expected: i64,

        /// The size the array actually was.
        found: usize,
    },

    #[fail(display = "Error while reading from buffer.")]
    ReadErr {
        #[cause]
        io_error: io::Error,
    },
}

impl From<io::Error> for EncodeError {
    fn from(io_error: io::Error) -> Self {
        EncodeError::ReadErr { io_error }
    }
}

/// An error that happens while trying to decode a message.
#[derive(Debug, Fail)]
pub enum DecodeError {
    /// The size variable for an array had an invalid size.
    #[fail(display = "Invalid array size of {}", size)]
    InvalidSize {
        size: i64,
    },

    /// The expected message hash does not match the found hash.
    #[fail(display = "Invalid hash found. Expected 0x{:X}, found 0x{:X}.", expected, found)]
    HashMismatch {
        /// The expected hash value.
        expected: u64,

        /// The found hash value.
        found: u64,
    },

    /// A boolean value was not encoded as either `0` or `1`.
    #[fail(display = "Value of {} is invalid for Booleans. Booleans should be encoded as 0 or 1.", val)]
    InvalidBoolean {
        /// The found value.
        val: i8,
    },

    /// Error parsing a string into Unicode.
    #[fail(display = "Invalid Unicode value found.")]
    Utf8Error {
        #[cause]
        utf8_err: string::FromUtf8Error,
    },

    /// Missing terminating null character in a string.
    #[fail(display = "String is missing null terminator.")]
    MissingNullTerminator,

    /// An error while writing to the buffer.
    #[fail(display = "Error while writing to the buffer.")]
    WriteErr {
        #[cause]
        io_error: io::Error,
    }
}
impl DecodeError {
    /// Create a new `InvalidSize`.
    pub fn invalid_size(size: i64) -> Self {
        DecodeError::InvalidSize { size }
    }

    /// Create a new `HashMismatch`.
    pub fn hash_mismatch(expected: u64, found: u64) -> Self {
        DecodeError::HashMismatch { expected, found }
    }

    /// Create a new `InvalidBoolean`.
    pub fn invalid_bool(val: i8) -> Self {
        DecodeError::InvalidBoolean { val }
    }

    /// Create a new `Utf8Error`.
    pub fn invalid_utf8(utf8_err: string::FromUtf8Error) -> Self {
        DecodeError::Utf8Error { utf8_err }
    }
}
impl From<io::Error> for DecodeError {
    fn from(io_error: io::Error) -> Self {
        DecodeError::WriteErr { io_error }
    }
}
