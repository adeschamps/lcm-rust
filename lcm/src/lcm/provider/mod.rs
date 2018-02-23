use std::io;
use std::time::Duration;
use regex::Regex;
use {Message, Provider, Subscription};

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
        unimplemented!();
    }

    /// Subscribe to a topic.
    pub fn subscribe<M, F>(&mut self, channel: Regex, buffer_size: usize, callback: F) -> Subscription
        where M: Message,
              F: FnMut(M) + 'a
    {
        unimplemented!();
    }

    /// Unsubscribes a message handler.
    pub fn unsubscribe(&mut self, subscription: Subscription) {
        unimplemented!();
    }

    /// Publishes a message on the specified channel.
    pub fn publish<M>(&mut self, channel: &str, message: &M)
        where M: Message
    {
        unimplemented!();
    }

    /// Waits for and dispatches messages.
    pub fn handle(&mut self) {
        unimplemented!();
    }

    /// Waits for and dispatches messages, with a timeout.
    pub fn handle_timeout(&mut self, timeout: Duration) {
        unimplemented!();
    }
}
