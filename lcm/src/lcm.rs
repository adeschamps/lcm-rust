use std::marker::PhantomData;
use Message;

/// An LCM instance that handles publishing and subscribing as well as encoding
/// and decoding messages.
pub struct Lcm<'a> {
    // TODO
    _pd: PhantomData<&'a ()>,
}
impl<'a> Lcm<'a> {
    /// Creates a new `Lcm` instance.
    pub fn new() -> Self
    {
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
