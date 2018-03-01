use std::env;
use std::io::{Write, Read};
use std::collections::HashMap;
use std::time::Duration;
use regex::Regex;

mod providers;
use self::providers::udpm::UdpmProvider;

use {Marshall, Message};
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

/// Default LCM URL to be used when the "LCM_DEFAULT_URL" environment variable
/// is not available.
const LCM_DEFAULT_URL: &'static str = "udpm://239.255.76.67:7667?ttl=0";


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
        let lcm_default_url = env::var("LCM_DEFAULT_URL");
        let lcm_url = match lcm_default_url {
            Ok(ref s) if s.is_empty() => {
                debug!("LCM_DEFAULT_URL available but empty. Using default settings.");
                LCM_DEFAULT_URL
            },
            Ok(ref s) => {
                debug!("LCM_DEFAULT_URL=\"{}\"", s);
                s
            },
            Err(_) => {
                debug!("LCM_DEFAULT_URL not present or unavailable. Using default settings.");
                LCM_DEFAULT_URL
            }
        };

        Lcm::with_lcm_url(lcm_url)
    }

    /// Create a new `Lcm` instance with the provider constructed from the
    /// supplied LCM URL.
    pub fn with_lcm_url(lcm_url: &str) -> Result<Self, LcmInitError> {
        debug!("Creating LCM instance using \"{}\"", lcm_url);
        let (provider_name, network, options) = parse_lcm_url(lcm_url)?;

        let provider = match provider_name {
            "udpm" => Provider::Udpm(UdpmProvider::new(network, options)?),

            #[cfg(feature = "file")]
            "file" => Provider::File(FileProvider::new(network, options)?),

            _ => return Err(LcmInitError::UnknownProvider(provider_name.into())),
        };

        Ok(Lcm {
            provider
        })
    }

    /// Subscribes a callback to a particular channel.
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

    /// Subscribes a raw callback to a particular channel.
    ///
    /// The normal `Lcm::subscribe` function should be preferred over this one.
    pub fn subscribe_raw<F>(&mut self, channel: &str, buffer_size: usize, mut callback: F) -> Result<Subscription, SubscribeError>
        where F: FnMut(&[u8]) + 'a
    {
        self.subscribe(channel, buffer_size, move |m: RawBytes| {
            callback(&m.0);
        })
    }

    /// Unsubscribes a message handler.
    pub fn unsubscribe(&mut self, subscription: Subscription) {
        provider!(self.unsubscribe(subscription))
    }

    /// Publishes a message on the specified channel.
    pub fn publish<M>(&mut self, channel: &str, message: &M) -> Result<(), PublishError>
        where M: Message
    {
        provider!(self.publish(channel, message))
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
        provider!(self.handle())
    }

    /// Waits for and dispatches messages, with a timeout.
    pub fn handle_timeout(&mut self, timeout: Duration) -> Result<(), HandleError> {
        provider!(self.handle_timeout(timeout))
    }
}

/// A subscription to an LCM topic.
///
/// Used to unsubscribe from a channel.
#[derive(Debug, PartialEq, Eq)]
pub struct Subscription(u32);

/// The backing providers for the `Lcm` type.
enum Provider<'a> {
    /// The UDP Multicast provider.
    Udpm(UdpmProvider<'a>),

    /// The log file provider.
    #[cfg(feature = "file")]
    File(FileProvider<'a>),
}

/// A type used to allow users to subscribe to raw bytes.
struct RawBytes(Vec<u8>);
impl Marshall for RawBytes {
    fn encode(&self, _: &mut Write) -> Result<(), EncodeError> {
        unimplemented!();
    }

    fn decode(_: &mut Read) -> Result<Self, DecodeError> {
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

    fn decode_with_hash(buffer: &mut Read) -> Result<Self, DecodeError> {
        let mut bytes = Vec::new();
        buffer.read_to_end(&mut bytes)?;
        Ok(RawBytes(bytes))
    }
}

/// Parses the string into its LCM URL components.
fn parse_lcm_url(lcm_url: &str) -> Result<(&str, &str, HashMap<&str, &str>), LcmInitError> {
    // Start by parsing the provider string
    let (provider, remaining) = if let Some(p) = lcm_url.find("://") {
        let (p, r) = lcm_url.split_at(p);
        (p, &r[3..])
    } else { return Err(LcmInitError::InvalidLcmUrl) };

    // Then split the network string from the options.
    let (network, options) = if let Some(p) = remaining.rfind('?') {
        let (n, o) = remaining.split_at(p);
        (n, &o[1..])
    } else { (remaining, "") };

    // Now we convert the options string into a map
    let options = match options {
        "" => HashMap::new(),
        _ => {
            options.split('&').map(|s| {
                if let Some(p) = s.find('=') {
                    let (a, v) = s.split_at(p);
                    Ok((a, &v[1..]))
                } else { Err(LcmInitError::InvalidLcmUrl) }
            }).collect::<Result<_, _>>()?
        }
    };

    // Then we can return it all
    Ok((provider, network, options))
}
