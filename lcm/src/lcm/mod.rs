use std::env;
use std::collections::HashMap;
use std::time::Duration;
use regex::Regex;

mod providers;
#[cfg(feature = "udpm")]
use self::providers::udpm::UdpmProvider;

use Message;
use error::*;

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
    provider: Provider<'a>,
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
        let (provider_name, network, options) = parse_lcm_url(lcm_url)?;

        let provider = match provider_name {
            #[cfg(feature = "udpm")]
            "udpm" => Provider::Udpm(UdpmProvider::new(network, &options)?),

            #[cfg(feature = "file")]
            "file" => Provider::File(FileProvider::new(network, &options)?),

            _ => return Err(InitError::UnknownProvider(provider_name.into())),
        };

        Ok(Lcm { provider })
    }

    /// Subscribes a callback to a particular topic.
    ///
    /// The input is interpreted as a regular expression. Unlike the C
    /// implementation of LCM, the expression is *not* implicitly surrounded
    /// by '^' and '$'.
    pub fn subscribe<M, F>(
        &mut self,
        channel: &str,
        buffer_size: usize,
        callback: F,
    ) -> Result<Subscription, SubscribeError>
    where
        M: Message + Send + 'static,
        F: FnMut(M) + 'a,
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
    pub fn publish<M>(&mut self, channel: &str, message: &M) -> Result<(), PublishError>
    where
        M: Message,
    {
        provider!(self.publish(channel, message))
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
pub enum Provider<'a> {
    /// The UDP Multicast provider.
    #[cfg(feature = "udpm")]
    Udpm(UdpmProvider<'a>),

    /// The log file provider.
    #[cfg(feature = "file")]
    File(FileProvider<'a>),
}

/// Parses the string into its LCM URL components.
fn parse_lcm_url(lcm_url: &str) -> Result<(&str, &str, HashMap<&str, &str>), InitError> {
    // Start by parsing the provider string
    let (provider, remaining) = if let Some(p) = lcm_url.find("://") {
        let (p, r) = lcm_url.split_at(p);
        (p, &r[3..])
    } else {
        return Err(InitError::InvalidLcmUrl);
    };

    // Then split the network string from the options.
    let (network, options) = if let Some(p) = remaining.rfind('?') {
        let (n, o) = remaining.split_at(p);
        (n, &o[1..])
    } else {
        (remaining, "")
    };

    // Now we convert the options string into a map
    let options = match options {
        "" => HashMap::new(),
        _ => options
            .split('&')
            .map(|s| {
                if let Some(p) = s.find('=') {
                    let (a, v) = s.split_at(p);
                    Ok((a, &v[1..]))
                } else {
                    Err(InitError::InvalidLcmUrl)
                }
            })
            .collect::<Result<_, _>>()?,
    };

    // Then we can return it all
    Ok((provider, network, options))
}
