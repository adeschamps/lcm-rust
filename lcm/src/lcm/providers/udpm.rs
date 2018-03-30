use std::thread;
use std::io::{self, Write};
use std::collections::HashMap;
use std::time::Duration;
use std::sync::mpsc;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::borrow::Borrow;
use url::Url;
use byteorder::{ByteOrder, NetworkEndian, WriteBytesExt};

use lcm::{MAX_MESSAGE_SIZE, TrampolineError, SubscribeMsg};
use error::*;

/// LCM's magic number for short messages.
const SHORT_HEADER_MAGIC: u32 = 0x4C43_3032;
/// LCM's magic number for message fragments.
const LONG_HEADER_MAGIC: u32 = 0x4C43_3033;

/// The maximum size for datagrams.
///
/// We want this to stay below the Ethernet MTU.
pub const MAX_DATAGRAM_SIZE: usize = 1400;

/// The header size for small datagrams.
pub const SMALL_HEADER_SIZE: usize = 8;

/// The header size for fragmented datagrams.
pub const FRAG_HEADER_SIZE: usize = 20;

/// The UDP Multicast provider.
///
/// It starts a new thread to handle the incoming messages. Those messages are
/// converted from raw bytes to an LCM datagram and then checked against all
/// the subscriptions in the background thread. The user thread only sees the
/// message ones it has been sent through the SPSC queue.
pub struct UdpmProvider {
    /// The socket used to send datagrams.
    socket: UdpSocket,

    /// The multicast address.
    addr: SocketAddr,

    /// The channel used to notify the `Lcm` object that messages have been
    /// queued.
    notify_rx: mpsc::Receiver<()>,

    /// The sequence number for the outgoing messages.
    sequence_number: u32,
}
impl UdpmProvider {
    /// Creates a new UDPM provider using the given settings.
    pub fn new(url: &Url, subscribe_rx: mpsc::Receiver<SubscribeMsg>) -> Result<Self, InitError> {
        // Parse the network string into the address and port
        let addr = url.to_socket_addrs()?
            .next()
            .expect("The URL should contain an address");

        // Parse additional options
        let mut ttl = 0;
        for (key, value) in url.query_pairs() {
            match key.borrow() {
                "ttl" => ttl = value.parse().map_err(InitError::InvalidTtl)?,
                "recv_buf_size" => { /* TODO: support this option */ }
                _ => {}
            }
        }

        debug!(
            "Starting UDPM provider with multicast (ip = {}, port = {}, ttl = {})",
            addr.ip(),
            addr.port(),
            ttl
        );
        let socket = UdpmProvider::setup_udp_socket(addr, ttl)?;
        let (notify_tx, notify_rx) = mpsc::sync_channel(1);

        let receiver = Backend::new(socket.try_clone()?, notify_tx, subscribe_rx);

        debug!("Starting read thread");
        thread::spawn(move || {
            let res = receiver.run();
            if let Err(e) = res {
                error!("Read thread failed with message: {}", e);
            }
        });

        Ok(UdpmProvider {
            socket,
            addr,
            notify_rx,
            sequence_number: 0,
        })
    }

    /// Publishes a message on the specified channel.
    ///
    /// This message will be sent directly by the `UdpmProvider` without being
    /// sent to the backend.
    pub fn publish(&mut self, channel: &str, message_buf: &[u8]) -> Result<(), PublishError> {
        // Determine if we need to split this message up into fragments
        let available = MAX_DATAGRAM_SIZE - SMALL_HEADER_SIZE - (channel.len() + 1);
        if message_buf.len() > available {
            // We need to break this into fragments
            self.send_frag_datagram(channel, &message_buf)?;
        } else {
            // This message can go out in a single datagram
            self.send_small_datagram(channel, &message_buf)?;
        }

        self.sequence_number += 1;
        Ok(())
    }

    /// Waits for and dispatches messages.
    ///
    /// Blocks on the `notify_rx` channel until a message comes through and
    /// then runs the callback on all available messages.
    pub fn handle(&mut self) -> Result<(), HandleError> {
        debug!("Waiting on notify channel");
        self.notify_rx.recv()?;
        Ok(())
    }

    /// Waits for and dispatches messages, with a timeout.
    ///
    /// Does the same thing as `UdpmProvider::handle` but with a timeout.
    pub fn handle_timeout(&mut self, timeout: Duration) -> Result<(), HandleError> {
        debug!("Waiting on notify channel");
        if let Err(mpsc::RecvTimeoutError::Disconnected) = self.notify_rx.recv_timeout(timeout) {
            warn!("The provider has been shut down or otherwise killed.");
            return Err(HandleError::ProviderIssue);
        }
        Ok(())
    }

    /// Set up the UDP socket.
    fn setup_udp_socket(addr: SocketAddr, ttl: u32) -> io::Result<UdpSocket> {
        use net2::UdpBuilder;

        let builder = UdpBuilder::new_v4()?;

        debug!("Setting SO_REUSEADDR");
        builder.reuse_address(true)?;

        // The UDPM source for the C version of LCM says that the SO_REUSEPORT
        // only needs to be set on MacOS and FreeBSD.
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        {
            use net2::unix::UnixUdpBuilderExt;
            debug!("Setting SO_REUSEPORT");
            builder.reuse_port(true)?;
        }

        // FIXME:
        // The C version of LCM increases the receive buffer size on Win32. Do
        // we need to do this and how?
        warn!("Not checking receive buffer size");

        debug!("Binding UDP socket");
        let socket = {
            let inaddr_any = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
            builder.bind(SocketAddr::new(inaddr_any, addr.port()))?
        };

        debug!("Joining multicast group");
        match addr.ip() {
            IpAddr::V4(ref addr) => socket.join_multicast_v4(addr, &Ipv4Addr::new(0, 0, 0, 0))?,
            IpAddr::V6(ref _addr) => unimplemented!("IPv6 is not supported."),
        }

        debug!("Setting multicast packet TTL to {}", ttl);
        socket.set_multicast_ttl_v4(ttl)?;

        Ok(socket)
    }

    /// Sends the message using the "fragmented message" datagram.
    fn send_frag_datagram(&mut self, channel: &str, message: &[u8]) -> Result<(), PublishError> {
        let mut buf = [0u8; MAX_DATAGRAM_SIZE];

        let n_fragments = {
            let available = MAX_DATAGRAM_SIZE - FRAG_HEADER_SIZE;
            let first_available = available - channel.len() - 1;

            1 + (message.len() + available - first_available) / available
        };

        if n_fragments > ::std::u16::MAX as usize {
            // Probably a redundant check
            warn!("The message was broken into too many fragments. Unable to send.");
            return Err(PublishError::ProviderIssue);
        }

        trace!(
            "Sending {} fragment datagrams on channel \"{}\"",
            n_fragments,
            channel
        );
        let mut remaining_message = message;
        let mut fragment_offset = 0;
        for fragment_number in 0..n_fragments {
            let (datagram_size, amount_written) = {
                let mut buf = &mut buf[..];

                // We're writing to a slice, so these can never fail.
                buf.write_u32::<NetworkEndian>(LONG_HEADER_MAGIC).unwrap();
                buf.write_u32::<NetworkEndian>(self.sequence_number)
                    .unwrap();
                buf.write_u32::<NetworkEndian>(message.len() as u32)
                    .unwrap();
                buf.write_u32::<NetworkEndian>(fragment_offset).unwrap();
                buf.write_u16::<NetworkEndian>(fragment_number as u16)
                    .unwrap();
                buf.write_u16::<NetworkEndian>(n_fragments as u16).unwrap();

                if fragment_number == 0 {
                    // We need to write the channel name in the very first fragment
                    for &b in channel.as_bytes() {
                        buf.write_u8(b).unwrap();
                    }
                    buf.write_u8(0).unwrap();
                }

                let amount_written = buf.write(remaining_message).unwrap();
                let message_end = FRAG_HEADER_SIZE + if fragment_number == 0 {
                    channel.len() + 1
                } else {
                    0
                };

                (message_end + amount_written, amount_written)
            };

            let sent = self.socket.send_to(&buf[0..datagram_size], self.addr)?;

            if sent != datagram_size {
                warn!(
                    "The number of bytes sent ({}) did not equal the size of the datagram ({}).",
                    sent, datagram_size
                );
                return Err(PublishError::ProviderIssue);
            }

            remaining_message = &remaining_message[amount_written..];
            fragment_offset += amount_written as u32;
        }

        Ok(())
    }

    /// Sends the message using a "small message" datagram.
    ///
    /// This function will panic if the message does not actually fit within a
    /// small datagram.
    fn send_small_datagram(&mut self, channel: &str, message: &[u8]) -> Result<(), PublishError> {
        trace!("Sending small datagram on channel \"{}\"", channel);
        let mut buf = [0u8; MAX_DATAGRAM_SIZE];

        let datagram_size = {
            let mut buf = &mut buf[..];
            let payload_start = SMALL_HEADER_SIZE + channel.len() + 1;
            let payload_end = payload_start + message.len();

            assert!(payload_end <= MAX_DATAGRAM_SIZE);

            // We're writing to a slice, so these can never fail. Literally,
            // the code for writing to a slice does not have a way to return an
            // `Err`.
            buf.write_u32::<NetworkEndian>(SHORT_HEADER_MAGIC).unwrap();
            buf.write_u32::<NetworkEndian>(self.sequence_number)
                .unwrap();
            for &b in channel.as_bytes() {
                buf.write_u8(b).unwrap();
            }
            buf.write_u8(0).unwrap();

            buf.write_all(message).unwrap();

            payload_end
        };

        let sent = self.socket.send_to(&buf[0..datagram_size], self.addr)?;

        if sent != datagram_size {
            warn!(
                "The number of bytes sent ({}) did not equal the size of the datagram ({}).",
                sent, datagram_size
            );
            Err(PublishError::ProviderIssue)
        } else {
            Ok(())
        }
    }
}

/// The LCM backend used for receiving UDPM messages without blocking the main
/// thread.
pub struct Backend {
    /// The multicast socket used for receiving datagrams.
    socket: UdpSocket,

    /// The channel used to notify the provider object that messages have been
    /// queued.
    notify_tx: mpsc::SyncSender<()>,

    /// The channel used to subscribe to a new topic.
    subscribe_rx: mpsc::Receiver<SubscribeMsg>,

    /// The list of subscribed channels and the closure used to send the
    /// messages back to the provider object.
    subscriptions: Vec<SubscribeMsg>,

    /// Partially complete messages.
    fragments: HashMap<SocketAddr, FragmentBuffer>,
}
impl Backend {
    /// Create a `Backend` with the specified channels.
    fn new(
        socket: UdpSocket,
        notify_tx: mpsc::SyncSender<()>,
        subscribe_rx: mpsc::Receiver<SubscribeMsg>,
    ) -> Self {
        Backend {
            socket,
            notify_tx,
            subscribe_rx,
            subscriptions: Vec::new(),
            fragments: HashMap::new(),
        }
    }

    /// Enter the `Backend` execution loop.
    ///
    /// This function will wait for events on the UDP socket and forward them
    /// through the appropriate channels based on subscriptions. It will only
    /// exit if the notification channel closes (which signifies that the
    /// client provider object has been deleted).
    fn run(mut self) -> io::Result<()> {
        let mut buf = [0u8; 0xFFFF];
        loop {
            // Wait for an incoming datagram
            trace!("Waiting on socket");
            let (count, from) = self.socket.recv_from(&mut buf)?;
            trace!("Datagram on socket");

            // If the message used the whole buffer then there is a good chance
            // that some bytes were discarded. We should warn the user.
            if count == buf.len() {
                debug!("Read buffer fully utilized. Bytes may have been dropped.");
            }

            // Make sure the subscription list is fully up-to-date
            self.check_for_subscriptions();

            // If it's too short, it absolutely can't be an LCM message.
            if count < 4 {
                debug!("Datagram too short to be message. Dropping.");
                continue;
            }

            // Try to process the message. If at least one of the subscriptions
            // accepts the message, notify the `Lcm` object. If the notify
            // channel is shut down, exit the loop and kill the thread.
            if self.process_datagram(&buf[0..count], from) && !self.notify() {
                break;
            }
        }

        Ok(())
    }

    /// Process the given datagram.
    fn process_datagram(&mut self, datagram: &[u8], sender: SocketAddr) -> bool {
        trace!(
            "Incoming datagram of size {} from {}.",
            datagram.len(),
            sender
        );

        match NetworkEndian::read_u32(&datagram[0..4]) {
            SHORT_HEADER_MAGIC => self.process_short_datagram(datagram),
            LONG_HEADER_MAGIC => self.process_frag_datagram(datagram, sender),
            _ => {
                debug!("Invalid magic in datagram. Dropping.");
                false
            }
        }
    }

    /// Retrieve the message from a short datagram
    fn process_short_datagram(&mut self, datagram: &[u8]) -> bool {
        use std::str;

        trace!("Incoming short datagram.");

        // Find the channel name. Anything after that is the message.
        let (channel, message) = {
            let channel_name_end = match datagram
                .iter()
                .skip(SMALL_HEADER_SIZE)
                .position(|&b| b == 0)
            {
                Some(p) => p + SMALL_HEADER_SIZE,
                None => {
                    debug!("Unable to parse channel name in datagram. Dropping.");
                    return false;
                }
            };

            let name_slice = &datagram[SMALL_HEADER_SIZE..channel_name_end];
            match str::from_utf8(name_slice) {
                Ok(s) => (s, &datagram[channel_name_end + 1..]),
                Err(_) => {
                    debug!("Invalid UTF-8 in channel name. Dropping.");
                    return false;
                }
            }
        };

        Backend::forward_message(&mut self.subscriptions, channel, message)
    }

    /// Retrieve the message portion from a fragment datagram.
    fn process_frag_datagram(&mut self, datagram: &[u8], sender: SocketAddr) -> bool {
        use std::str;

        trace!("Incoming fragment datagram.");

        let sequence_number = NetworkEndian::read_u32(&datagram[4..8]);
        let payload_size = NetworkEndian::read_u32(&datagram[8..12]) as usize;
        let fragment_offset = NetworkEndian::read_u32(&datagram[12..16]) as usize;
        let fragment_number = NetworkEndian::read_u16(&datagram[16..18]);
        let n_fragments = NetworkEndian::read_u16(&datagram[18..20]);

        if payload_size > MAX_MESSAGE_SIZE {
            debug!("Message too long. Dropping.");
            return false;
        }

        trace!("Recieved fragment {} of {}", fragment_number, n_fragments);

        let fragment = self.fragments
            .entry(sender)
            .or_insert_with(|| FragmentBuffer {
                parts_remaining: 0,
                sequence_number: 0,
                channel: String::new(),
                buffer: Vec::new(),
            });

        // If there is already a fragment, check to see if it is a part of this
        // message. If not, clear it out.
        if fragment.sequence_number != sequence_number || fragment.buffer.len() != payload_size {
            if fragment.parts_remaining != 0 {
                debug!(
                    "Dropping fragmented message. Missing {} parts.",
                    fragment.parts_remaining
                );
            }
            fragment.parts_remaining = n_fragments;
            fragment.sequence_number = sequence_number;
            fragment.channel.clear();
            fragment.buffer.resize(payload_size, 0);
        }

        // Place this fragment in the buffer.
        let message = if fragment_number == 0 {
            let channel_name_end =
                match datagram.iter().skip(FRAG_HEADER_SIZE).position(|&b| b == 0) {
                    Some(p) => p + FRAG_HEADER_SIZE,
                    None => {
                        debug!("Unable to parse channel name in datagram. Dropping.");
                        return false;
                    }
                };

            let name_slice = &datagram[FRAG_HEADER_SIZE..channel_name_end];
            match str::from_utf8(name_slice) {
                Ok(s) => {
                    if fragment.channel.is_empty() {
                        fragment.channel.push_str(s);
                    }

                    &datagram[channel_name_end + 1..]
                }
                Err(_) => {
                    debug!("Invalid UTF-8 in channel name. Dropping.");
                    return false;
                }
            }
        } else {
            &datagram[FRAG_HEADER_SIZE..]
        };

        fragment.parts_remaining -= 1;
        fragment.buffer[fragment_offset..fragment_offset + message.len()].copy_from_slice(message);

        // If we aren't waiting on any more parts, forward the message.
        if fragment.parts_remaining == 0 {
            Backend::forward_message(&mut self.subscriptions, &fragment.channel, &fragment.buffer)
        } else {
            false
        }
    }

    /// Sends the message to the callbacks.
    ///
    /// The function has this form to fight the borrow checker.
    fn forward_message(
        subscriptions: &mut Vec<SubscribeMsg>,
        channel: &str,
        message: &[u8],
    ) -> bool {
        // FIXME:
        // Dealing with unsubscriptions this way means that resources aren't
        // released until the first message received on the unsubscribed
        // channel.
        let mut forwarded = false;
        subscriptions.retain(|&(ref re, ref f)| {
            trace!(
                "Checking if channel \"{}\" matches regular expression \"{}\"",
                channel,
                re
            );
            if re.is_match(channel) {
                trace!("Channel \"{}\" matched subscription \"{}\"", channel, re);
                match (*f)(channel, message) {
                    Err(TrampolineError::MessageChannelClosed) => false,
                    Err(e) => {
                        warn!("Error decoding message: {}", e);
                        true
                    }
                    Ok(_) => {
                        forwarded = true;
                        true
                    }
                }
            } else {
                true
            }
        });

        forwarded
    }

    /// Checks to see if there are new pending subscriptions.
    fn check_for_subscriptions(&mut self) {
        self.subscriptions.extend(self.subscribe_rx.try_iter());
    }

    /// Notifies the provider object that there is at least one message queued.
    ///
    /// Returns false if the notification channel has been closed.
    fn notify(&self) -> bool {
        match self.notify_tx.try_send(()) {
            Ok(_) | Err(mpsc::TrySendError::Full(_)) => true,
            Err(mpsc::TrySendError::Disconnected(_)) => {
                debug!("Notification channel disconnected. Killing read thread.");
                false
            }
        }
    }
}

/// A partially complete message.
struct FragmentBuffer {
    /// The number of fragments still necessary for this message.
    parts_remaining: u16,

    /// The sequence number of this message.
    sequence_number: u32,

    /// The channel this message is to be published on.
    channel: String,

    /// The received parts of the message.
    buffer: Vec<u8>,
}
