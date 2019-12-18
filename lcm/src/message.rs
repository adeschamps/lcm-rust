use std::io::{Read, Write};
use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};

use error::*;

/// A type that can be encoded and decoded according to the LCM protocol.
pub trait Marshall: Sized {
    /// Encodes a message into a buffer.
    /// `Lcm` uses a `Vec<u8>` with its capacity set to the value returned by [`size()`].
    fn encode(&self, buffer: &mut dyn Write) -> Result<(), EncodeError>;

    /// Decodes a message from a buffer.
    fn decode(buffer: &mut dyn Read) -> Result<Self, DecodeError>;

    /// Returns the number of bytes this message is expected to take when encoded.
    fn size(&self) -> usize;
}

/// A message that can be send and received by the LCM protocol.
pub trait Message: Marshall {
    /// The message hash for this type.
    const HASH: u64;

    /// Encodes a message into a buffer, with the message hash at the beginning.
    fn encode_with_hash(&self) -> Result<Vec<u8>, EncodeError> {
        let size = Self::HASH.size() + self.size();
        let mut buffer = Vec::with_capacity(size);
        Self::HASH.encode(&mut buffer)?;
        self.encode(&mut buffer)?;
        Ok(buffer)
    }

    /// Decodes a message from a buffer,
    /// and also checks that the hash at the beginning is correct.
    fn decode_with_hash(mut buffer: &mut dyn Read) -> Result<Self, DecodeError> {
        let hash: u64 = Marshall::decode(&mut buffer)?;
        if hash != Self::HASH {
            return Err(DecodeError::HashMismatch { expected: Self::HASH, found: hash});
        }
        Marshall::decode(buffer)
    }
}

macro_rules! impl_marshall {
    ( $type:ty, $read:ident, $write:ident $(, $endian:ident )* ) => {
        impl Marshall for $type {
            fn encode(&self, buffer: &mut dyn Write) -> Result<(), EncodeError> {
                buffer.$write::<$($endian),*>(*self)?;
                Ok(())
            }

            fn decode(buffer: &mut dyn Read) -> Result<Self, DecodeError> {
                let res = buffer.$read::<$($endian),*>()?;
                Ok(res)
            }

            fn size(&self) -> usize {
                ::std::mem::size_of::<$type>()
            }
        }
    };
}

impl_marshall!(u8, read_u8, write_u8);
impl_marshall!(u64, read_u64, write_u64, NetworkEndian);

impl_marshall!(i8, read_i8, write_i8);
impl_marshall!(i16, read_i16, write_i16, NetworkEndian);
impl_marshall!(i32, read_i32, write_i32, NetworkEndian);
impl_marshall!(i64, read_i64, write_i64, NetworkEndian);

impl_marshall!(f32, read_f32, write_f32, NetworkEndian);
impl_marshall!(f64, read_f64, write_f64, NetworkEndian);

impl Marshall for bool {
    fn encode(&self, buffer: &mut dyn Write) -> Result<(), EncodeError> {
        let value: i8 = if *self { 1 } else { 0 };
        value.encode(buffer)
    }

    fn decode(buffer: &mut dyn Read) -> Result<Self, DecodeError> {
        let value = i8::decode(buffer)?;
        match value {
            0 => Ok(false),
            1 => Ok(true),
            v => Err(DecodeError::InvalidBoolean(v)),
        }
    }

    fn size(&self) -> usize {
        ::std::mem::size_of::<i8>()
    }
}

impl Marshall for String {
    fn encode(&self, buffer: &mut dyn Write) -> Result<(), EncodeError> {
        let len: i32 = self.len() as i32 + 1;
        len.encode(buffer)?;
        for &b in self.as_bytes() {
            b.encode(buffer)?;
        }
        (0 as u8).encode(buffer)
    }

    fn decode(buffer: &mut dyn Read) -> Result<Self, DecodeError> {
        // Until fallable allocation is stable, we can't use
        // Vec::with_capacity because an invalid input could cause a
        // panic.

        let len = i32::decode(buffer)?;
        if len <= 0 {
            return Err(DecodeError::InvalidSize(i64::from(len)));
        }
        let len = len - 1;
        let mut buf = Vec::new();
        for _ in 0..len {
            buf.push(u8::decode(buffer)?);
        }
        let result = String::from_utf8(buf).map_err(|e| DecodeError::Utf8Error(e))?;
        match buffer.read_u8() {
            Ok(0) => Ok(result),
            Ok(_) => Err(DecodeError::MissingNullTerminator),
            Err(e) => Err(e)?,
        }
    }

    fn size(&self) -> usize {
        ::std::mem::size_of::<i32>() + self.len() + 1
    }
}

#[cfg(test)]
mod test {
    use super::*;

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
}
