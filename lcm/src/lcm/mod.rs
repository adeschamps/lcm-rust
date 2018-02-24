use std::env;
use std::time::Duration;
use regex::Regex;

mod providers;
use self::providers::udpm::UdpmProvider;

use Message;
use error::*;

/// Convenience macro for dispatching functions among providers.
macro_rules! provider
{
    ($self:ident.$func:ident($($args:expr),*)) => {
        match $self.provider {
            Provider::Udpm(ref mut p) => p.$func($($args),*),

            #[cfg(feature = "file")]
            Provider::File(ref mut p) => p.$func($($args),*),
        }
    }
}


/// An LCM instance that handles publishing and subscribing as well as encoding
/// and decoding messages.
pub struct Lcm<'a> {
    /// The backing provider.
    ///
    /// This provider basically does all of the work, with the `Lcm` struct
    /// being a unified frontend.
    provider: Provider<'a>,
}
impl<'a> Lcm<'a> {
    /// Creates a new `Lcm` instance.
    ///
    /// This uses the `LCM_DEFAULT_URL` environment variable to construct a
    /// provider. If the variable does not exist or is empty, it will use the
    /// LCM default of "udpm://239.255.76.67:7667?ttl=0".
    pub fn new() -> Result<Self, LcmInitError> {
        unimplemented!();
    }

    /// Create a new `Lcm` instance with the provider constructed from the
    /// supplied LCM URL.
    pub fn with_lcm_url(lcm_url: &str) -> Result<Self, LcmInitError> {
        unimplemented!();
    }

    /// Subscribes a callback to a particular topic.
    ///
    /// The input is interpreted as a regular expression. Unlike the C
    /// implementation of LCM, the expression is *not* implicitly surrounded
    /// by '^' and '$'.
    pub fn subscribe<M, F>(&mut self, channel: &str, buffer_size: usize, callback: F) -> Result<Subscription, SubscribeError>
        where M: Message + Send + 'static,
              F: FnMut(M) + 'a
    {
        let re = Regex::new(channel)?;

        // Dispatch the subscription request
        provider!(self.subscribe(re, buffer_size, callback))
    }

    /// Unsubscribes a message handler.
    pub fn unsubscribe(&mut self, subscription: Subscription) {
        provider!(self.unsubscribe(subscription))
    }

    /// Publishes a message on the specified channel.
    pub fn publish<M>(&mut self, channel: &str, message: &M)
        where M: Message
    {
        provider!(self.publish(channel, message))
    }

    /// Waits for and dispatches messages.
    pub fn handle(&mut self) {
        provider!(self.handle())
    }

    /// Waits for and dispatches messages, with a timeout.
    pub fn handle_timeout(&mut self, timeout: Duration) {
        provider!(self.handle_timeout(timeout))
    }
}

/// A subscription to an LCM topic.
///
/// Used to unsubscribe from a channel.
#[derive(Debug)]
pub struct Subscription(u32);

/// The backing providers for the `Lcm` type.
pub enum Provider<'a> {
    /// The UDP Multicast provider.
    Udpm(UdpmProvider<'a>),

    /// The log file provider.
    #[cfg(feature = "file")]
    File(FileProvider<'a>),
}
