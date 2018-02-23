use std::net::Ipv4Addr;

/// The LCM network provider.
///
/// The default provider is the UDPM provider on `239.255.76.67:7667` with a
/// TTL of 0.
pub enum Provider {
    /// UDP Multicast provider.
    Udpm {
        /// The multicast address.
        addr: Ipv4Addr,

        /// The multicast port.
        port: u16,

        /// Time To Live of the of transmitted packets.
        ///
        /// A value of `0` will keep the packets on localhost. A value of `1`
        /// will keep packets within the local network. It is unlikely you want
        /// a value that is not one of those two.
        ttl: u32,
    },
}
impl Default for Provider {
    fn default() -> Self {
        Provider::Udpm {
            addr: Ipv4Addr::new(239, 255, 76, 67),
            port: 7667,
            ttl: 0
        }
    }
}
