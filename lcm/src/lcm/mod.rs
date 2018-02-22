use std::{io, thread};
use std::sync::mpsc;
use std::net::{Ipv4Addr, UdpSocket};
use regex::Regex;
use Message;

mod receiver;
use self::receiver::Receiver;

mod spsc;

/// An LCM instance that handles publishing and subscribing as well as encoding
/// and decoding messages.
pub struct Lcm<'a> {
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
        let socket = Lcm::setup_udp_socket(addr, port, ttl)?;
        let (notify_tx, notify_rx) = mpsc::sync_channel(1);
        let (subscribe_tx, subscribe_rx) = mpsc::channel();

        let receiver = Receiver::new(socket.try_clone()?, notify_tx, subscribe_rx);

        debug!("Starting read thread");
        thread::spawn(move || {
            let res = receiver.run();
            if let Err(e) = res {
                error!("Read thread failed with message: {}", e);
            }
        });

        Ok(Lcm {
            socket, notify_rx,
            next_subscription_id: 0,
            subscriptions: Vec::new(),
            sequence_number: 0,
        })
    }

    /// Subscribes a callback to a particular topic.
    pub fn subscribe<M, F>(&mut self, channel: &str, buffer_size: usize, callback: F) -> Subscription
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

    /// Set up the UDP socket.
    fn setup_udp_socket(addr: &Ipv4Addr, port: u16, ttl: u32) -> io::Result<UdpSocket> {
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
        socket.join_multicast_v4(addr, &Ipv4Addr::new(0, 0, 0, 0))?;

        debug!("Setting multicast packet TTL to {}", ttl);
        socket.set_multicast_ttl_v4(ttl)?;

        Ok(socket)
    }
}

/// A subscription to an LCM topic.
///
/// Used to unsubscribe from a channel.
#[derive(Debug)]
pub struct Subscription(u32);

/// An LCM datagram.
///
/// This can either be a complete datagram or a fragment of a message.
enum Datagram<'a> {
    Complete(&'a [u8]),
    Fragment,
}
impl<'a> Datagram<'a> {
    /// Parses raw bytes into a `Datagram`.
    fn parse(buf: &'a [u8]) -> Result<Self, &str> {
        unimplemented!();
    }
}

/// Message used to subscribe to a new channel.
type SubscribeMsg = (Regex, Box<Fn() -> io::Result<()> + Send + 'static>);

/// LCM's magic number for short messages.
const SHORT_HEADER_MAGIC: u32 = 0x3230434c;
/// LCM's magic number for message fragments.
const LONG_HEADER_MAGIC: u32 = 0x3330434c;
