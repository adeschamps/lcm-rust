/// LCM's magic number for short messages.
const SHORT_HEADER_MAGIC: u32 = 0x3230434c;
/// LCM's magic number for message fragments.
const LONG_HEADER_MAGIC: u32 = 0x3330434c;

/// An LCM datagram.
///
/// This can either be a complete datagram or a fragment of a message.
pub enum Datagram<'a> {
    Complete(&'a [u8]),
    Fragment,
}
impl<'a> Datagram<'a> {
    /// Parses raw bytes into a `Datagram`.
    pub fn parse(buf: &'a [u8]) -> Result<Self, &str> {
        unimplemented!();
    }
}
