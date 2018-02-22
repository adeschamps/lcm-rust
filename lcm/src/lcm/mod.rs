use std::io;
use std::net::Ipv4Addr;
use Message;

mod receiver;
use self::receiver::Receiver;

use std::marker::PhantomData;

/// An LCM instance that handles publishing and subscribing as well as encoding
/// and decoding messages.
pub struct Lcm<'a> {
    // TODO
    _pd: PhantomData<&'a ()>,
}
impl<'a> Lcm<'a> {
    /// Creates a new `Lcm` instance with the default settings.
    ///
    /// The default address is "239.255.76.67:7667" with a TTL of 0.
    pub fn new() -> io::Result<Self> {
        let ip_addr = Ipv4Addr::new(239, 255, 76, 67);
        let port = 7667;
        let ttl = 0;

        Lcm::with_settings(&ip_addr, port, ttl)
    }

    /// Creates a new `Lcm` instance with the specified settings.
    pub fn with_settings(addr: &Ipv4Addr, port: u16, ttl: u32) -> io::Result<Self>
    {
        debug!("Creating LCM instance with lcm_url=\"udpm://{}:{}?ttl={}\"", addr, port, ttl);

        let receiver = Receiver::new(addr, port, ttl)?;
        unimplemented!();
    }

    /// Subscribes a callback to a particular topic.
    pub fn subscribe<M, F>(&mut self, channel: &str, callback: F) -> Subscription
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

    /// Waits for and dispatches the next incoming message.
    pub fn handle(&mut self) {
        unimplemented!();
    }
}

/// A subscription to an LCM topic.
///
/// Used to unsubscribe from a channel.
pub struct Subscription {
    // TODO
}
