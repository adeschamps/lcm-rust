use std::io;
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
    #[fail(display = "Invalid regular expression used to subscribe to channel: \"{}\"", channel)]
    InvalidRegex {
        channel: String,

        #[cause]
        regex_error: regex::Error,
    },

    /// The provider is no longer active.
    #[fail(display = "The provider is no longer active")]
    MissingProvider {
        #[cause]
        io_error: io::Error,
    },
}
