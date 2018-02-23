use std::{io, thread};
use std::time::Duration;
use std::sync::mpsc;
use std::net::{Ipv4Addr, UdpSocket};
use regex::Regex;

use {Message, Subscription};
use utils::spsc;

mod datagram;
mod backend;
use self::backend::Backend;

/// Message used to subscribe to a new channel.
type SubscribeMsg = (Regex, Box<Fn() -> io::Result<()> + Send + 'static>);

/// The UDP Multicast provider.
///
/// It starts a new thread to handle the incoming messages. Those messages are
/// converted from raw bytes to an LCM datagram and then checked against all
/// the subscriptions in the background thread. The user thread only sees the
/// message ones it has been sent through the SPSC queue.
pub struct UdpmProvider<'a> {
    /// The socket used to send datagrams.
    socket: UdpSocket,

    /// The channel used to notify the `Lcm` object that messages have been
    /// queued.
    notify_rx: mpsc::Receiver<()>,

    /// The next available subscription ID
    next_subscription_id: u32,
    /// The subscriptions.
    subscriptions: Vec<(Subscription, Box<FnMut() + 'a>)>,

    /// The sequence number for the outgoing messages.
    sequence_number: u32,
}
impl<'a> UdpmProvider<'a> {
    /// Creates a new UDPM provider using the given settings.
    pub fn new(addr: Ipv4Addr, port: u16, ttl: u32) -> io::Result<Self>
    {
        debug!("Creating LCM provider with lcm_url=\"udpm://{}:{}?ttl={}\"", addr, port, ttl);
        let socket = UdpmProvider::setup_udp_socket(addr, port, ttl)?;
        let (notify_tx, notify_rx) = mpsc::sync_channel(1);
        let (subscribe_tx, subscribe_rx) = mpsc::channel();

        let receiver = Backend::new(socket.try_clone()?, notify_tx, subscribe_rx);

        debug!("Starting read thread");
        thread::spawn(move || {
            let res = receiver.run();
            if let Err(e) = res {
                error!("Read thread failed with message: {}", e);
            }
        });

        Ok(UdpmProvider {
            socket, notify_rx,
            next_subscription_id: 0,
            subscriptions: Vec::new(),
            sequence_number: 0,
        })
    }

    /// Subscribes a callback to a particular topic.
    ///
    /// This involves sending the `channel` and a closure to the currently
    /// running `Backend`. The closure will be used to convert the LCM datagram
    /// into an actual message type which will then be passed to the client.
    pub fn subscribe<M, F>(&mut self, channel: Regex, buffer_size: usize, callback: F) -> Subscription
        where M: Message,
              F: FnMut(M) + 'a
    {
        unimplemented!();
    }

    /// Unsubscribes a message handler.
    ///
    /// All this will do is delete the subscription from the `Vec`. The backend
    /// will determine that the topic has been unsubscribed since the SPSC
    /// channel used to send messages will be closed.
    pub fn unsubscribe(&mut self, subscription: Subscription) {
        unimplemented!();
    }

    /// Publishes a message on the specified channel.
    ///
    /// This message will be sent directly by the `UdpmProvider` without being
    /// sent to the backend.
    pub fn publish<M>(&mut self, channel: &str, message: &M)
        where M: Message
    {
        unimplemented!();
    }

    /// Waits for and dispatches messages.
    ///
    /// Blocks on the `notify_rx` channel until a message comes through and
    /// then runs the callback on all available messages.
    pub fn handle(&mut self) {
        unimplemented!();
    }

    /// Waits for and dispatches messages, with a timeout.
    ///
    /// Does the same thing as `UdpmProvider::handle` but with a timeout.
    pub fn handle_timeout(&mut self, timeout: Duration) {
        unimplemented!();
    }

    /// Set up the UDP socket.
    fn setup_udp_socket(addr: Ipv4Addr, port: u16, ttl: u32) -> io::Result<UdpSocket> {
        use std::net::{SocketAddr, IpAddr};

        debug!("Binding UDP socket");
        let socket = {
            let inaddr_any = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
            UdpSocket::bind(&SocketAddr::new(inaddr_any, port))?
        };

        // FIXME:
        // Rust only supportes SO_REUSEADDR through crates. Come back to this
        // later, after deciding which crate to use (probably net2 or plain
        // libc). See lcm_udpm.c:936-956
        warn!("Skipping SO_REUSEADDR and SO_REUSEPORT");

        // FIXME:
        // The C version of LCM increases the receive buffer size on Win32. Do
        // we need to do this and how?
        warn!("Not checking receive buffer size");

        debug!("Joining multicast group");
        socket.join_multicast_v4(&addr, &Ipv4Addr::new(0, 0, 0, 0))?;

        debug!("Setting multicast packet TTL to {}", ttl);
        socket.set_multicast_ttl_v4(ttl)?;

        Ok(socket)
    }
}
