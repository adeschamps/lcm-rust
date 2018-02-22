use std::io;
use std::net::{UdpSocket, SocketAddr, IpAddr, Ipv4Addr};

/// The LCM backend used for receiving messages without blocking the main
/// thread.
pub struct Receiver {
    socket: UdpSocket,
}
impl Receiver {
    /// Create a `Receiver` with the specified settings.
    pub fn new(addr: &Ipv4Addr, port: u16, ttl: u32) -> io::Result<Self> {
        let socket = Receiver::setup_udp_socket(addr, port, ttl)?;

        Ok(Receiver {
            socket,
        })
    }

    /// Set up the UDP socket.
    fn setup_udp_socket(addr: &Ipv4Addr, port: u16, ttl: u32) -> io::Result<UdpSocket> {
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
