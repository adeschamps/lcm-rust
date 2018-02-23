use std::time::Duration;
use regex::Regex;

mod provider;

use {Message, Provider};
use error::*;
use self::provider::VTable;


/// An LCM instance that handles publishing and subscribing as well as encoding
/// and decoding messages.
pub struct Lcm<'a> {
    /// The backing provider.
    ///
    /// This provider basically does all of the work, with the `Lcm` struct
    /// being a unified frontend. The name comes from the C implementation.
    vtable: VTable<'a>,
}
impl<'a> Lcm<'a> {
    /// Creates a new `Lcm` instance using the specified provider.
    pub fn new(provider: Provider) -> Result<Self, ProviderStartError> {
        Ok(Lcm {
            vtable: VTable::new(provider)?,
        })
    }

    /// Subscribes a callback to a particular topic.
    ///
    /// The input is interpreted as a regular expression. Unlike the C
    /// implementation of LCM, the expression is *not* implicitly surrounded
    /// by '^' and '$'.
    pub fn subscribe<M, F>(&mut self, channel: &str, buffer_size: usize, callback: F) -> Result<Subscription, SubscribeError>
        where M: Message,
              F: FnMut(M) + 'a
    {
        // We can't just use the `?` operator since we need the channel name for context.
        let re = match Regex::new(channel) {
            Ok(re) => re,
            Err(e) => return Err(SubscribeError::InvalidRegex { channel: channel.into(), regex_error: e }),
        };

        self.vtable.subscribe(re, buffer_size, callback)
    }

    /// Unsubscribes a message handler.
    pub fn unsubscribe(&mut self, subscription: Subscription) {
        self.vtable.unsubscribe(subscription);
    }

    /// Publishes a message on the specified channel.
    pub fn publish<M>(&mut self, channel: &str, message: &M)
        where M: Message
    {
        self.vtable.publish(channel, message);
    }

    /// Waits for and dispatches messages.
    pub fn handle(&mut self) {
        self.vtable.handle();
    }

    /// Waits for and dispatches messages, with a timeout.
    pub fn handle_timeout(&mut self, timeout: Duration) {
        self.vtable.handle_timeout(timeout);
    }
}

/// A subscription to an LCM topic.
///
/// Used to unsubscribe from a channel.
#[derive(Debug)]
pub struct Subscription(u32);
