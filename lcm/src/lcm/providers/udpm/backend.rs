use std::io;
use std::sync::mpsc;
use std::net::{UdpSocket, SocketAddr};

use super::SubscribeMsg;
use super::datagram::Datagram;

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
}
impl Backend {
    /// Create a `Backend` with the specified channels.
    pub fn new(socket: UdpSocket, notify_tx: mpsc::SyncSender<()>, subscribe_rx: mpsc::Receiver<SubscribeMsg>) -> Self {
        Backend {
            socket, notify_tx, subscribe_rx,
            subscriptions: Vec::new(),
        }
    }

    /// Enter the `Backend` execution loop.
    ///
    /// This function will wait for events on the UDP socket and forward them
    /// through the appropriate channels based on subscriptions. It will only
    /// exit if the notification channel closes (which signifies that the
    /// client provider object has been deleted).
    pub fn run(mut self) -> io::Result<()> {
        let mut buf = [0u8; 65535];

        loop {
            // Wait for an incoming datagram
            let (count, from) = self.socket.recv_from(&mut buf)?;

            // If the message used the whole buffer then there is a good chance
            // that some bytes were discarded. We should warn the user.
            if count == buf.len() {
                warn!("Read buffer fully utilized. Bytes may have been dropped.");
            }

            // Parse the message from raw bytes to an LCM datagram
            let datagram = match Datagram::parse(&buf[0..count]) {
                Ok(d)  => d,
                Err(e) => {
                    warn!("Bad datagram received: {}", e);
                    continue;
                }
            };

            // Make sure the subscription list is fully up-to-date
            self.check_for_subscriptions();

            // Try to process the message. If at least one of the subscriptions
            // accepts the message, notify the `Lcm` object. If the notify
            // channel is shut down, exit the loop and kill the thread.
            if self.process_datagram(datagram, from) && !self.notify() {
                break;
            }
        }

        Ok(())
    }

    /// Process the given datagram.
    fn process_datagram(&self, datagram: Datagram, sender: SocketAddr) -> bool {
        // TODO:
        // If the datagram is a fragment, add that to the fragment map. If the
        // fragments now make a complete message, proceed with that. Otherwise,
        // go down the list of subscriptions, check the regex against the
        // channel, and see if anyone reports that they succeeded.
        unimplemented!();
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
            Ok(_) | Err(mpsc::TrySendError::Full(_)) => { true },
            Err(mpsc::TrySendError::Disconnected(_)) => {
                debug!("Notification channel disconnected. Killing read thread.");
                false
            }
        }
    }
}
