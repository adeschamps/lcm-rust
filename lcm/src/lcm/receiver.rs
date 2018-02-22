use std::sync::mpsc;
use std::net::UdpSocket;

/// The LCM backend used for receiving messages without blocking the main
/// thread.
pub struct Receiver {
    /// The multicast socket used for receiving datagrams.
    socket: UdpSocket,

    /// The channel used to notify the `Lcm` object that messages have been
    /// queued.
    notify_tx: mpsc::SyncSender<()>,
}
impl Receiver {
    /// Create a `Receiver` with the specified settings.
    pub fn new(socket: UdpSocket, notify_tx: mpsc::SyncSender<()>) -> Self {
        Receiver { socket, notify_tx }
    }

    /// Enter the `Receiver` execution loop.
    ///
    /// This function will wait for events on the UDP socket and forward them
    /// through the appropriate channels based on subscriptions. It will only
    /// exit if the subscription channel closes (which signifies that the
    /// client `Lcm` object has been deleted).
    pub fn run(&self) {
        
    }
}
