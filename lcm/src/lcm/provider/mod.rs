use std::io;
use std::time::Duration;
use regex::Regex;

use {Message, Provider, Subscription};
use error::*;

mod udpm;
use self::udpm::UdpmProvider;

/// The backing providers for the `Lcm` type.
pub enum VTable<'a> {
    /// The UDP Multicast provider.
    Udpm(UdpmProvider<'a>),
}
impl<'a> VTable<'a> {
    /// Create a new VTable based on the given `Provider`.
    pub fn new(provider: Provider) -> io::Result<Self> {
        Ok(match provider {
            Provider::Udpm { addr, port, ttl } => VTable::Udpm(UdpmProvider::new(addr, port, ttl)?),
        })
    }

    /// Subscribe to a topic.
    pub fn subscribe<M, F>(&mut self, channel: Regex, buffer_size: usize, callback: F) -> Result<Subscription, SubscribeError>
        where M: Message,
              F: FnMut(M) + 'a
    {
        match *self {
            VTable::Udpm(ref mut p) => p.subscribe(channel, buffer_size, callback),
        }
    }

    /// Unsubscribes a message handler.
    pub fn unsubscribe(&mut self, subscription: Subscription) {
        match *self {
            VTable::Udpm(ref mut p) => p.unsubscribe(subscription),
        }
    }

    /// Publishes a message on the specified channel.
    pub fn publish<M>(&mut self, channel: &str, message: &M)
        where M: Message
    {
        match *self {
            VTable::Udpm(ref mut p) => p.publish(channel, message),
        }
    }

    /// Waits for and dispatches messages.
    pub fn handle(&mut self) {
        match *self {
            VTable::Udpm(ref mut p) => p.handle(),
        }
    }

    /// Waits for and dispatches messages, with a timeout.
    pub fn handle_timeout(&mut self, timeout: Duration) {
        match *self {
            VTable::Udpm(ref mut p) => p.handle_timeout(timeout),
        }
    }
}
