use std::{io, string};
use std::sync::mpsc;
use regex;

// TODO:
// The errors need significant work. I just sort of added errors as I needed
// them without any real thought or design.

/// An error indicating that there was a failure to start the `Provider`.
#[derive(Debug, Fail)]
#[fail(display = "Failed to start LCM provider.")]
pub enum LcmInitError {
    /// The provider failed to start.
    #[fail(display = "Failed to start the LCM provider.")]
    ProviderStart(#[cause] io::Error),

    /// The provider is not known.
    #[fail(display = "Unknown provider \"{}\"", _0)]
    UnknownProvider(String),

    /// The LCM URL was not valid.
    #[fail(display = "The LCM URL was not valid")]
    InvalidLcmUrl,
}

impl From<io::Error> for LcmInitError {
    fn from(io_error: io::Error) -> Self {
        LcmInitError::ProviderStart(io_error)
    }
}

/// An error indicating that an attempt to subscribe to a topic was
/// unsuccessful.
#[derive(Debug, Fail)]
pub enum SubscribeError {
    /// The provided string was an invalid regular expression.
    #[fail(display = "Invalid regular expression used to subscribe to channel.")]
    InvalidRegex(#[cause] regex::Error),

    /// The provider is no longer active.
    #[fail(display = "The provider is no longer active.")]
    MissingProvider,
}

impl From<regex::Error> for SubscribeError {
    fn from(regex_error: regex::Error) -> Self {
        SubscribeError::InvalidRegex(regex_error)
    }
}

/// An error during publishing.
#[derive(Debug, Fail)]
pub enum PublishError {
    /// An error happened while trying to encode the message.
    #[fail(display = "Error encoding message.")]
    MessageEncoding(#[cause] EncodeError),

    /// An IO issue with the provider.
    #[fail(display = "Error with the provider.")]
    ProviderIssue(#[cause] io::Error),

    /// The full message was not sent.
    #[fail(display = "Unable to send the full message.")]
    MessageNotSent,

    /// The message was too large to be sent.
    #[fail(display = "Message too large to send.")]
    MessageTooLarge,
}

impl From<EncodeError> for PublishError {
    fn from(err: EncodeError) -> Self {
        PublishError::MessageEncoding(err)
    }
}

impl From<io::Error> for PublishError {
    fn from(io_err: io::Error) -> Self {
        PublishError::ProviderIssue(io_err)
    }
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

impl From<mpsc::RecvError> for HandleError {
    fn from(_: mpsc::RecvError) -> Self {
        HandleError::MissingProvider
    }
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

    /// The message channel was closed.
    ///
    /// Probably represents an unsubscription.
    #[fail(display = "Message channel was closed.")]
    MessageChannelClosed,

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
