use std::net::Ipv4Addr;

/// The LCM network provider.
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
