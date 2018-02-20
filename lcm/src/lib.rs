//! This is just a placeholder for the actual implementation of
//! LCM. It just contains the `Message` trait, so that tests can
//! compile.

extern crate byteorder;

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

/// A message that can be encoded and decoded according to the LCM protocol.
pub trait Message: Sized {
    /// Returns the message hash for this type.
    /// Returns `0` for all primitive types.
    /// Generated `Lcm` types should implement this function.
    const HASH: u64;

    /// Encodes a message into a buffer, with the message hash at the beginning.
    fn encode_with_hash(&self) -> io::Result<Vec<u8>> {
        let size = Self::HASH.size() + self.size();
        let mut buffer = Vec::with_capacity(size);
        Self::HASH.encode(&mut buffer)?;
        self.encode(&mut buffer)?;
        Ok(buffer)
    }

    /// Decodes a message from a buffer,
    /// and also checks that the hash at the beginning is correct.
    fn decode_with_hash(mut buffer: &mut Read) -> io::Result<Self> {
        let hash: u64 = Message::decode(&mut buffer)?;
        if hash != Self::HASH {
            return Err(io::Error::new(io::ErrorKind::Other, "Invalid hash"));
        }
        Message::decode(buffer)
    }

    /// Encodes a message into a buffer.
    /// `Lcm` uses a `Vec<u8>` with its capacity set to the value returned by [`size()`].
    fn encode(&self, buffer: &mut Write) -> io::Result<()>;

    /// Decodes a message from a buffer.
    fn decode(buffer: &mut Read) -> io::Result<Self>;

    /// Returns the number of bytes this message is expected to take when encoded.
    fn size(&self) -> usize;
}

macro_rules! impl_message {
    ( $type:ty, $read:ident, $write:ident $(, $endian:ident )* ) => {
        impl Message for $type {
            const HASH: u64 = 0;

            fn encode(&self, buffer: &mut Write) -> io::Result<()> {
                buffer.$write::<$($endian),*>(*self)
            }

            fn decode(buffer: &mut Read) -> io::Result<Self> {
                buffer.$read::<$($endian),*>()
            }

            fn size(&self) -> usize {
                ::std::mem::size_of::<$type>()
            }
        }
    };
}

impl_message!(u8, read_u8, write_u8);
impl_message!(u64, read_u64, write_u64, NetworkEndian);

impl_message!(i8, read_i8, write_i8);
impl_message!(i16, read_i16, write_i16, NetworkEndian);
impl_message!(i32, read_i32, write_i32, NetworkEndian);
impl_message!(i64, read_i64, write_i64, NetworkEndian);

impl_message!(f32, read_f32, write_f32, NetworkEndian);
impl_message!(f64, read_f64, write_f64, NetworkEndian);

impl Message for bool {
    const HASH: u64 = 0;

    fn encode(&self, buffer: &mut Write) -> io::Result<()> {
        let value: i8 = if *self { 1 } else { 0 };
        value.encode(buffer)
    }

    fn decode(buffer: &mut Read) -> io::Result<Self> {
        let value = i8::decode(buffer)?;
        match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Booleans should be encoded as 0 or 1",
            )),
        }
    }

    fn size(&self) -> usize {
        ::std::mem::size_of::<i8>()
    }
}

impl Message for String {
    const HASH: u64 = 0;

    fn encode(&self, buffer: &mut Write) -> io::Result<()> {
        let len: i32 = self.len() as i32 + 1;
        len.encode(buffer)?;
        for &b in self.as_bytes() {
            b.encode(buffer)?;
        }
        (0 as u8).encode(buffer)
    }

    fn decode(buffer: &mut Read) -> io::Result<Self> {
        // Until fallable allocation is stable, we can't use
        // Vec::with_capacity because an invalid input could cause a
        // panic.

        let len = i32::decode(buffer)?;
        if len <= 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Attempting to decode a string with an invalid size.",
            ));
        }
        let len = len - 1;
        let mut buf = Vec::new();
        for _ in 0..len {
            buf.push(u8::decode(buffer)?);
        }
        let result = String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        match buffer.read_u8() {
            Ok(0) => Ok(result),
            Ok(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected null terminator",
            )),
            Err(e) => Err(e),
        }
    }

    fn size(&self) -> usize {
        ::std::mem::size_of::<i32>() + self.len() + 1
    }
}

#[test]
fn decode_string() {
    let s: String = "Hello, world!".into();
    let mut buffer = Vec::new();
    s.encode(&mut buffer).unwrap();

    let decoded = String::decode(&mut buffer.as_slice()).unwrap();
    assert_eq!(decoded, "Hello, world!");
}

#[test]
fn decode_null_string() {
    let mut buffer: &[u8] = &[255, 0, 0, 0];
    let decoded = String::decode(&mut buffer);
    assert!(decoded.is_err());
}
