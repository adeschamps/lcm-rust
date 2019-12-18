use std::env;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::Duration;
use regex::Regex;
use url::Url;

mod providers;
#[cfg(feature = "file")]
use self::providers::file::FileProvider;
#[cfg(feature = "udpm")]
use self::providers::udpm::UdpmProvider;

use {Marshall, Message};
use error::*;
use utils::spsc;

/// Message used to subscribe to a new channel.
type SubscribeMsg = (
    Regex,
    Box<dyn Fn(&str, &[u8]) -> Result<(), TrampolineError> + Send + 'static>,
);

/// This is the maximum allowed message size.
///
/// The C version of LCM discards any message greater than this size.
pub const MAX_MESSAGE_SIZE: usize = 1 << 28;

/// The maximum allow number of bytes in a channel name.
pub const MAX_CHANNEL_NAME_LENGTH: usize = 63;

/// Convenience macro for dispatching functions among providers.
macro_rules! provider
{
    ($self:ident.$func:ident($($args:expr),*)) => {
        match $self.provider {
            #[cfg(feature = "udpm")]
            Provider::Udpm(ref mut p) => p.$func($($args),*),

            #[cfg(feature = "file")]
            Provider::File(ref mut p) => p.$func($($args),*),
        }
    }
}

/// Default LCM URL to be used when the `LCM_DEFAULT_URL` environment variable
/// is not available.
const LCM_DEFAULT_URL: &str = "udpm://239.255.76.67:7667?ttl=0";

/// An LCM instance that handles publishing and subscribing as well as encoding
/// and decoding messages.
pub struct Lcm<'a> {
    /// The backing provider.
    ///
    /// This provider basically does all of the work, with the `Lcm` struct
    /// being a unified frontend.
    provider: Provider,

    /// The next available subscription ID
    next_subscription_id: u32,
    /// The subscriptions.
    subscriptions: Vec<(Subscription, Box<dyn FnMut() + 'a>)>,
    /// The channel used to notify the backend of new subscriptions.
    subscribe_tx: mpsc::Sender<SubscribeMsg>,
}
impl<'a> Lcm<'a> {
    /// Creates a new `Lcm` instance.
    ///
    /// This uses the `LCM_DEFAULT_URL` environment variable to construct a
    /// provider. If the variable does not exist or is empty, it will use the
    /// LCM default of "udpm://239.255.76.67:7667?ttl=0".
    pub fn new() -> Result<Self, InitError> {
        let lcm_default_url = env::var("LCM_DEFAULT_URL");
        let lcm_url = match lcm_default_url {
            Ok(ref s) if s.is_empty() => {
                debug!("LCM_DEFAULT_URL available but empty. Using default settings.");
                LCM_DEFAULT_URL
            }
            Ok(ref s) => {
                debug!("LCM_DEFAULT_URL=\"{}\"", s);
                s
            }
            Err(_) => {
                debug!("LCM_DEFAULT_URL not present or unavailable. Using default settings.");
                LCM_DEFAULT_URL
            }
        };

        Lcm::with_lcm_url(lcm_url)
    }

    /// Create a new `Lcm` instance with the provider constructed from the
    /// supplied LCM URL.
    pub fn with_lcm_url(lcm_url: &str) -> Result<Self, InitError> {
        debug!("Creating LCM instance using \"{}\"", lcm_url);
        let url = Url::parse(lcm_url)?;

        let (subscribe_tx, subscribe_rx) = mpsc::channel();

        let provider = match url.scheme() {
            #[cfg(feature = "udpm")]
            "udpm" => Provider::Udpm(UdpmProvider::new(&url, subscribe_rx)?),

            #[cfg(feature = "file")]
            "file" => Provider::File(FileProvider::new(&url)?),

            scheme => return Err(InitError::UnknownProvider(scheme.into())),
        };

        Ok(Lcm {
            provider,
            next_subscription_id: 0,
            subscriptions: Vec::new(),
            subscribe_tx,
        })
    }

    /// Subscribes a callback to a particular channel.
    ///
    /// The input is interpreted as a regular expression. Unlike the C
    /// implementation of LCM, the expression is *not* implicitly surrounded
    /// by `^` and `$`.
    pub fn subscribe<M, F>(
        &mut self,
        channel: &str,
        buffer_size: usize,
        mut callback: F,
    ) -> Result<Subscription, SubscribeError>
    where
        M: Message + Send + 'static,
        F: FnMut(&str, M) + 'a,
    {
        let channel = Regex::new(channel)?;

        // Create the channel used to send the message back from the backend
        let (tx, rx) = spsc::channel::<(String, M)>(buffer_size);

        // Then create the function that will convert the bytes into a message
        // and send it and the function that will pass things on to the callback.
        let conversion_func = move |chan: &str, mut bytes: &[u8]| -> Result<(), TrampolineError> {
            // First try to decode the message
            let message = M::decode_with_hash(&mut bytes)?;

            // Then double check that the channel isn't closed
            if tx.is_closed() {
                return Err(TrampolineError::MessageChannelClosed);
            }

            // Otherwise, put it in the queue and call it a day.
            tx.send((chan.into(), message));
            Ok(())
        };

        let callback_fn = move || {
            // We can't loop forever because they might be filling up faster
            // than we can process them. So we're only going to read a number
            // equal to the size of the queue. This seems like it would be the
            // least surprising behavior for the user.
            for _ in 0..rx.capacity() {
                if let Some((chan, m)) = rx.recv() {
                    callback(&chan, m);
                } else {
                    break;
                }
            }
        };

        // Finally, create the new subscription ID
        let sub_id = self.next_subscription_id;
        self.next_subscription_id += 1;

        // Send it across the way and then store our callback.
        match self.subscribe_tx.send((channel, Box::new(conversion_func))) {
            Ok(_) => {}
            Err(_) => {
                warn!("UDPM provider has died. Unable to send subscribe message.");
                return Err(SubscribeError::ProviderIssue);
            }
        }
        self.subscriptions
            .push((Subscription(sub_id), Box::new(callback_fn)));

        Ok(Subscription(sub_id))
    }

    /// Subscribes a raw callback to a particular channel.
    ///
    /// The normal `Lcm::subscribe` function should be preferred over this one.
    pub fn subscribe_raw<F>(
        &mut self,
        channel: &str,
        buffer_size: usize,
        mut callback: F,
    ) -> Result<Subscription, SubscribeError>
    where
        F: FnMut(&str, &[u8]) + 'a,
    {
        self.subscribe(channel, buffer_size, move |chan: &str, m: RawBytes| {
            callback(chan, &m.0);
        })
    }

    /// Unsubscribes a message handler.
    pub fn unsubscribe(&mut self, subscription: Subscription) {
        self.subscriptions
            .retain(|&(ref sub, _)| *sub != subscription);

        // Explicitly drop the subscription, since it is no longer
        // valid.  Without this, clippy suggests passing the
        // subscription by reference, which does not capture the
        // semantics of what this function does.
        drop(subscription);
    }

    /// Publishes a message on the specified channel.
    pub fn publish<M>(&mut self, channel: &str, message: &M) -> Result<(), PublishError>
    where
        M: Message,
    {
        let message_buf = message.encode_with_hash()?;

        if channel.len() > MAX_CHANNEL_NAME_LENGTH {
            warn!("The channel name was too long. Unable to publish message.");
            return Err(PublishError::ProviderIssue);
        }

        if message_buf.len() > MAX_MESSAGE_SIZE {
            warn!("The message was too large to publish.");
            return Err(PublishError::ProviderIssue);
        }

        provider!(self.publish(channel, &message_buf))
    }

    /// Publishes a raw message on the specified channel.
    ///
    /// The normal `Lcm::publish` function should be preferred over this one.
    pub fn publish_raw(&mut self, channel: &str, buffer: &[u8]) -> Result<(), PublishError> {
        // TODO:
        // This is a fairly inefficient implementation. At some point, it
        // should be replaced with something better.
        self.publish(channel, &RawBytes(buffer.to_owned()))
    }

    /// Waits for and dispatches messages.
    pub fn handle(&mut self) -> Result<(), HandleError> {
        provider!(self.handle())?;
        self.subscriptions
            .iter_mut()
            .for_each(|&mut (_, ref mut f)| (*f)());
        Ok(())
    }

    /// Waits for and dispatches messages, with a timeout.
    pub fn handle_timeout(&mut self, timeout: Duration) -> Result<(), HandleError> {
        provider!(self.handle_timeout(timeout))?;
        self.subscriptions
            .iter_mut()
            .for_each(|&mut (_, ref mut f)| (*f)());
        Ok(())
    }
} // impl Lcm

/// Errors that can happen during the trampoline closure.
#[derive(Debug, Fail)]
pub enum TrampolineError {
    /// The channel was closed.
    ///
    /// This generally signifies that the user unsubscribed from the channel.
    #[fail(display = "Unsubscribed from the channel")]
    MessageChannelClosed,

    /// There was a decoding error.
    #[fail(display = "Unable to decode message: {}", _0)]
    Decode(#[cause] DecodeError),
}
impl From<DecodeError> for TrampolineError {
    fn from(err: DecodeError) -> Self {
        TrampolineError::Decode(err)
    }
}

/// A subscription to an LCM topic.
///
/// Used to unsubscribe from a channel.
#[derive(Debug, PartialEq, Eq)]
pub struct Subscription(u32);

/// The backing providers for the `Lcm` type.
enum Provider {
    /// The UDP Multicast provider.
    #[cfg(feature = "udpm")]
    Udpm(UdpmProvider),

    /// The log file provider.
    #[cfg(feature = "file")]
    File(FileProvider),
}

/// A type used to allow users to subscribe to raw bytes.
struct RawBytes(Vec<u8>);
impl Marshall for RawBytes {
    fn encode(&self, _: &mut dyn Write) -> Result<(), EncodeError> {
        unimplemented!();
    }

    fn decode(_: &mut dyn Read) -> Result<Self, DecodeError> {
        unimplemented!();
    }

    fn size(&self) -> usize {
        unimplemented!();
    }
}
impl Message for RawBytes {
    const HASH: u64 = 0;

    fn encode_with_hash(&self) -> Result<Vec<u8>, EncodeError> {
        Ok(self.0.clone())
    }

    fn decode_with_hash(buffer: &mut dyn Read) -> Result<Self, DecodeError> {
        let mut bytes = Vec::new();
        buffer.read_to_end(&mut bytes)?;
        Ok(RawBytes(bytes))
    }
}
