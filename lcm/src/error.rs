//! Error types associated with LCM operations.
//!
//! In general, one will want to return an `Error` from a function as all of
//! the other errors can be converted into the `Error` using either the `?`
//! operator or `From`. The other error types exist in case one wants to
//! attempt to recover from an error.

use std::{io, string};
use regex;

// TODO:
// We should hide the `From<T>` implementations for all of these errors. Most
// of them only exist to make the code more readable in this crate and probably
// shouldn't be used by the end user. The best method for doing this (that I've
// heard) is to create separate "internal" and "external" errors and only do
// the `From` implementations for the inner. Until that is done, I have hidden
// the trait implementations from the docs.
//
// As they are hidden from the docs, I don't think I would consider making this
// change to *not* be a breaking change.

// TODO:
// There are a lot of `ProviderIssue` type errors in this module. I want to
// come up with some way to report the errors other than telling the user to
// look at the log but I'm not sure how to do it. I inintially attempted to use
// `Box<Fail>` but it didn't work. I think the only options might be:
// 1: Use `fail::Error`
//     * This is super expensive
//     * But it's also not on a happy path
// 2: Make this module aware of provider specific errors
//     * This could lead to a large number of error types
//     * But those could be filtered out via feature flags
//     * ...but that could make maintaining projects difficult

/// A generic LCM error.
///
/// If one does not intend to try and recover from errors, this is the best
/// error type to handle. All of the LCM errors can be converted to this type
/// using the `?` operator.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error happened while initializing the LCM instance.
    #[fail(display = "An error happened during initialization.")]
    Init(#[cause] InitError),

    /// An error happened while trying to subscribe to a channel.
    #[fail(display = "Failed to subscribe to the channel.")]
    Subscribe(#[cause] SubscribeError),

    /// An error happened while trying to publish a message.
    #[fail(display = "Failed to publish message.")]
    Publish(#[cause] PublishError),

    /// An error happened while trying to handle incoming messages.
    #[fail(display = "Unable to handle incoming messages.")]
    Handle(#[cause] HandleError),
}
impl From<InitError> for Error {
    fn from(err: InitError) -> Self {
        Error::Init(err)
    }
}
impl From<SubscribeError> for Error {
    fn from(err: SubscribeError) -> Self {
        Error::Subscribe(err)
    }
}
impl From<PublishError> for Error {
    fn from(err: PublishError) -> Self {
        Error::Publish(err)
    }
}
impl From<HandleError> for Error {
    fn from(err: HandleError) -> Self {
        Error::Handle(err)
    }
}


/// The LCM instance was unable to start.
#[derive(Debug, Fail)]
pub enum InitError {
    /// There was an IO issue that prevented the provider from starting.
    #[fail(display = "The LCM provider failed to start due to an IO error.")]
    IoError(#[cause] io::Error),

    /// The supplied LCM URL requested a provider that isn't known.
    ///
    /// If you get this error, check the feature flags on the crate. It is
    /// possible that the provider you are requesting is disabled.
    #[fail(display = "Unknown provider \"{}\".", _0)]
    UnknownProvider(String),

    /// The provided LCM URL was not valid.
    #[fail(display = "Invalid LCM URL.")]
    InvalidLcmUrl,
}

/// The attempt to subscribe to a channel was unsuccessful.
#[derive(Debug, Fail)]
pub enum SubscribeError {
    /// The provided string was an invalid regular expression.
    #[fail(display = "Invalid regular expression used.")]
    InvalidRegex(#[cause] regex::Error),

    /// The provider was unable to subscribe to the topic.
    ///
    /// Check the log for more information. Future releases should include more
    /// information in this error type.
    #[fail(display = "The provider failed to subscribe to the topic.")]
    ProviderIssue,
}

/// Publishing to a channel failed.
#[derive(Debug, Fail)]
pub enum PublishError {
    /// There was an error while trying to encode the message.
    #[fail(display = "Unable to encode the message.")]
    MessageEncoding(#[cause] EncodeError),

    /// There was an IO issue that prevented the provider from sending the
    /// message.
    #[fail(display = "Failed to send the message due to an IO error.")]
    IoError(#[cause] io::Error),

    /// The provider was unable to publish the message.
    ///
    /// Check the log for more information. Future releases should include more
    /// information in this error type.
    #[fail(display = "The provider was unable to publish the message.")]
    ProviderIssue,
}

/// Error occured while trying to handle incoming messages.
#[derive(Debug, Fail)]
pub enum HandleError {
    /// There was an IO error while trying to handle messages.
    #[fail(display = "Failed to handle messages due to an IO error.")]
    IoError(#[cause] io::Error),

    /// The provider was unable to handle the incoming messages.
    ///
    /// Check the log for more information. Future releases should include more
    /// information in this error type.
    #[fail(display = "The provider was unable to handle the incoming messages.")]
    ProviderIssue,
}

/// An error occurred while trying to decode a message.
#[derive(Debug, Fail)]
pub enum DecodeError {
    /// The size variable for an array was invalid.
    #[fail(display = "Invalid array size of {}.", _0)]
    InvalidSize(i64),

    /// The expected message hash does not match the found hash.
    #[fail(display = "Invalid hash found. Expected 0x{:X}, found 0x{:X}.", expected, found)]
    HashMismatch {
        /// The expected hash value.
        expected: u64,
        /// The found hash value.
        found: u64,
    },

    /// A boolean value was not encoded as either `0` or `1`.
    #[fail(display = "The value {} is invalid for booleans.", _0)]
    InvalidBoolean(i8),

    /// A string was not valid UTF-8.
    #[fail(display = "Invalid Unicode found.")]
    Utf8Error(#[cause] string::FromUtf8Error),

    /// A string was missing the null terminator.
    ///
    /// This doesn't stop us from parsing the string, but it does mean that the
    /// message is incorrectly encoded.
    #[fail(display = "String is missing the null terminator.")]
    MissingNullTerminator,

    /// An error occurred while trying to read from buffer.
    ///
    /// This error should never happen and should be removed in a future
    /// release. If it ever happens, please report a bug.
    #[fail(display = "An error happened while trying to read from the buffer.")]
    IoError(#[cause] io::Error),
}

/// An error occurred while trying to encode a message.
#[derive(Debug, Fail)]
pub enum EncodeError {
    /// There was a disagreement between the size variable and the size of the
    /// array.
    #[fail(display = "The size of the array does not match size specified in {}. Expected {}, found {}.", size_var, expected, found)]
    SizeMismatch {
        /// The field specifying the size of the array.
        size_var: &'static str,
        /// The size the array was expected to be.
        ///
        /// This is a signed variable since all of LCM's integer types are
        /// signed. This means that this error will happen any time the size
        /// variable is negative.
        expected: i64,
        /// The size the array actually was.
        found: usize,
    },

    /// An error occurred while trying to write to the buffer.
    ///
    /// This error should never happen and should be removed in a future
    /// release. If it ever happens, please report a bug.
    #[fail(display = "An error occurred while trying to write to the buffer.")]
    IoError(#[cause] io::Error),
}

#[doc(hidden)]
pub mod from {
    use std::sync::mpsc;
    use super::*;

    #[doc(hidden)]
    impl From<io::Error> for InitError {
        fn from(err: io::Error) -> Self {
            InitError::IoError(err)
        }
    }
    #[doc(hidden)]
    impl From<regex::Error> for SubscribeError {
        fn from(err: regex::Error) -> Self {
            SubscribeError::InvalidRegex(err)
        }
    }
    #[doc(hidden)]
    impl From<EncodeError> for PublishError {
        fn from(err: EncodeError) -> Self {
            PublishError::MessageEncoding(err)
        }
    }
    #[doc(hidden)]
    impl From<io::Error> for PublishError {
        fn from(err: io::Error) -> Self {
            PublishError::IoError(err)
        }
    }
    #[doc(hidden)]
    impl From<mpsc::RecvError> for HandleError {
        fn from(_: mpsc::RecvError) -> Self {
            HandleError::ProviderIssue
        }
    }
    #[doc(hidden)]
    impl From<io::Error> for DecodeError {
        fn from(err: io::Error) -> Self {
            DecodeError::IoError(err)
        }
    }
    #[doc(hidden)]
    impl From<io::Error> for EncodeError {
        fn from(err: io::Error) -> Self {
            EncodeError::IoError(err)
        }
    }
}
