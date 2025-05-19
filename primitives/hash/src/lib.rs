//! Provides the hash implementation for all borderless-primitives.
//!
//! This library contains the implementation of the [`Hash256`], that is used throughout the entire borderless stack.
//! Internally, it uses the sha-3 hash function to digest binary data and actually generate the hash.
//!
#![warn(missing_docs)]
pub use hash_generated::Hash256;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::fmt::Display;
use std::ops::Add;
use std::{
    convert::{TryFrom, TryInto},
    fmt,
};

// NOTE: The code generation creates a lot of artifacts,
// which result in a lot of warnings.
#[allow(unused_imports, dead_code, missing_docs)]
#[allow(clippy::all)]
mod hash_generated;

/// Easy way to feed an arbitrary amount of data into the [`Hasher`] and retrieving its result.
///
/// # Examples
/// ```
/// # #[macro_use] extern crate borderless_hash;
/// # fn main() {
/// let hash = calc_hash!(b"hash", b"over", b"some", b"amount", b"of", b"data");
/// # }
/// ```
///
/// Which is equivalent to:
/// ```
/// # use borderless_hash::Hasher;
/// let mut hasher = Hasher::new();
/// hasher.update(b"hash");
/// hasher.update(b"over");
/// hasher.update(b"some");
/// hasher.update(b"amount");
/// hasher.update(b"of");
/// hasher.update(b"data");
/// let hash = hasher.finalize();
/// ```
#[macro_export]
macro_rules! calc_hash {
    ( $( $x:expr ),* ) => {
        {
            let mut temp_hasher = $crate::Hasher::new();
            $(
                temp_hasher.update($x);
            )*
                temp_hasher.finalize()
        }
    };
}

/// Applies the hexadecimal-encoding scheme to some data but only returns the first eight characters.
///
/// Can be used to print a hash to console.
///
/// Note: The output of this function cannot be decoded again,
/// since we remove a relevant portion of the information!
pub fn b16_display<T: AsRef<[u8]>>(input: T) -> String {
    let out = base16::encode_lower(&input);
    out.chars().take(8).collect()
}

/// Hasher that produces the [`Hash256`] as an output.
///
/// Thin wrapper around the [`Sha3_256`] type.
pub struct Hasher(Sha3_256);

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher {
    /// Creates a new `Hasher` instance.
    pub fn new() -> Self {
        Hasher(Sha3_256::new())
    }

    /// Process data, updating the internal state.
    pub fn update<T: AsRef<[u8]>>(&mut self, data: &T) {
        self.0.update(data.as_ref());
    }

    /// Reset hasher instance to its initial state.
    pub fn reset(&mut self) {
        self.0.reset();
    }

    /// Retrieve result and consume hasher instance.
    pub fn finalize(self) -> Hash256 {
        self.0.finalize().into()
    }
}

// Helper for serde to force base16 encoding when serializing Hash256
enum Base16HashSerializer {}

impl Base16HashSerializer {
    pub fn serialize<S, Input>(bytes: Input, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        Input: AsRef<[u8]>,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&base16::encode_lower(&bytes))
        } else {
            serializer.serialize_bytes(bytes.as_ref())
        }
    }

    pub fn deserialize<'de, D, Output>(deserializer: D) -> Result<Output, D::Error>
    where
        D: serde::Deserializer<'de>,
        Output: From<[u8; 32]>,
    {
        struct Base16Visitor;

        impl<'de> serde::de::Visitor<'de> for Base16Visitor {
            type Value = [u8; 32];

            fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                write!(formatter, "Expecting base16 ASCII text or byte array")
            }

            fn visit_str<E>(self, v: &str) -> ::std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut output = [0u8; 32];
                base16::decode_slice(v, &mut output).map_err(serde::de::Error::custom)?;
                Ok(output)
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let output: [u8; 32] = v.try_into().map_err(serde::de::Error::custom)?;
                Ok(output)
            }
        }

        if deserializer.is_human_readable() {
            deserializer
                .deserialize_str(Base16Visitor)
                .map(|vec| Output::from(vec))
        } else {
            deserializer
                .deserialize_bytes(Base16Visitor)
                .map(Into::into)
        }
    }
}

impl Hash256 {
    /// Creates a new hash from a byte representation of some data.
    ///
    /// ```
    /// # use borderless_hash::Hash256;
    /// let data = "This is some data that can be hashed !";
    /// let hash = Hash256::digest(&data);
    /// ```
    pub fn digest<T: AsRef<[u8]>>(bytes: &T) -> Self {
        let buf = Sha3_256::digest(bytes.as_ref());
        Self(buf.into())
    }

    /// Creates a new hash from a byte representation of some data.
    ///
    /// A special byte `\x00` is fed into the hasher, before the data is added.
    /// Can be used for merkle hash trees that conform to [`RFC6962`].
    ///
    /// [`RFC6962`]: https://www.rfc-editor.org/rfc/rfc6962#section-2.1
    pub fn digest_w_x00<T: AsRef<[u8]>>(bytes: &T) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(b"\x00");
        hasher.update(bytes);
        hasher.finalize()
    }

    /// Creates a new hash from a byte representation of some data.
    ///
    /// A special byte `\x01` is fed into the hasher, before the data is added.
    /// Can be used for merkle hash trees that conform to [`RFC6962`].
    ///
    /// [`RFC6962`]: https://www.rfc-editor.org/rfc/rfc6962#section-2.1
    pub fn digest_w_x01<T: AsRef<[u8]>>(bytes: &T) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(b"\x01");
        hasher.update(bytes);
        hasher.finalize()
    }

    /// Creates a `Hash256`, where all bits are set to `0`.
    ///
    /// This is mostly used for testing, where we want to create hashes without some data.
    pub fn zero() -> Self {
        Hash256([0u8; 32])
    }

    /// Creates a new hash without any data.
    ///
    /// This returns the initial state of the hasher, without updating it with data.
    /// The following variants all create the same output hash:
    /// ```
    /// # use borderless_hash::{Hash256, Hasher};
    /// let hash = Hash256::empty();
    /// let h2 = Hash256::digest(b"");
    /// let mut hasher = Hasher::new();
    /// let h3 = hasher.finalize();
    ///
    /// assert_eq!(hash, h2);
    /// assert_eq!(hash, h3);
    /// ```
    pub fn empty() -> Self {
        Hash256::digest(b"")
    }

    /// Constructs a new hash from two other hashes.
    ///
    /// Note: The sum of two hashes is not commutative, so the order of the hashes matter:
    /// ```
    /// # use borderless_hash::Hash256;
    /// let h1 = Hash256::empty();
    /// let h2 = Hash256::zero();
    ///
    /// assert_ne!(Hash256::sum(&h1, &h2), Hash256::sum(&h2, &h1));
    /// ```
    ///
    /// Alternatively you can use the `+` operator directly, since the `Hash256` implements `Add`:
    /// ```
    /// # use borderless_hash::Hash256;
    /// let h1 = Hash256::empty();
    /// let h2 = Hash256::zero();
    /// let sum = Hash256::sum(&h1, &h2);
    ///
    /// assert_eq!(h1 + h2, sum);
    /// ```
    pub fn sum(h1: &Hash256, h2: &Hash256) -> Hash256 {
        let mut hasher = Hasher::new();
        hasher.update(h1);
        hasher.update(h2);
        hasher.finalize()
    }

    /// Converts the hash to an u64 value.
    ///
    /// The `Hash256` can also be used to generate random values (which are equally distributed,
    /// due to the cryptographic properties of the underlying hash-function).
    pub fn to_u64(&self) -> u64 {
        let mut out: u64 = 0u64;
        for i in 0..32 {
            out = out
                .overflowing_add(
                    (self.0.as_slice()[i] as u64)
                        .overflowing_shl(8u32 * (i as u32))
                        .0,
                )
                .0;
        }
        out
    }

    /// Consumes the hash and returns the underlying byte-slice as a vector
    pub fn into_vec(self) -> Vec<u8> {
        self.into()
    }

    /// Consumes the hash and returns the underlying byte-slice
    pub fn into_slice(self) -> [u8; 32] {
        self.into()
    }
}

impl Add for Hash256 {
    type Output = Hash256;

    fn add(self, rhs: Self) -> Self::Output {
        Hash256::sum(&self, &rhs)
    }
}

impl From<Hash256> for u64 {
    fn from(hash: Hash256) -> Self {
        hash.to_u64()
    }
}

impl From<sha3::digest::Output<Sha3_256>> for Hash256 {
    fn from(hash: sha3::digest::Output<Sha3_256>) -> Self {
        Hash256(hash.into())
    }
}

/// Error that indicates an invalid slice length.
///
/// The `Hash256` can only be build from `[u8; 32]`,
/// so if we try to create it from a slice of unknown size `&[u8]`,
/// the operation may fail and return this error.
#[derive(Debug)]
pub struct InvalidSliceLength(pub usize);

impl Display for InvalidSliceLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to decode hash - invalid slice length. Expected 32bytes, got {}",
            self.0
        )
    }
}

impl std::error::Error for InvalidSliceLength {}

impl From<[u8; 32]> for Hash256 {
    fn from(slice: [u8; 32]) -> Self {
        Hash256(slice)
    }
}

impl From<&[u8; 32]> for Hash256 {
    fn from(slice: &[u8; 32]) -> Self {
        Hash256(*slice)
    }
}

impl From<Hash256> for [u8; 32] {
    fn from(h: Hash256) -> Self {
        h.0
    }
}

impl TryFrom<&[u8]> for Hash256 {
    type Error = InvalidSliceLength;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 32 {
            return Err(InvalidSliceLength(value.len()));
        }
        let mut buf = [0; 32];
        buf.copy_from_slice(value);
        Ok(Hash256(buf))
    }
}

impl TryFrom<Vec<u8>> for Hash256 {
    type Error = InvalidSliceLength;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        value.as_slice().try_into()
    }
}

impl TryFrom<&Vec<u8>> for Hash256 {
    type Error = InvalidSliceLength;

    fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
        value.as_slice().try_into()
    }
}

impl From<Hash256> for Vec<u8> {
    fn from(h: Hash256) -> Self {
        h.0.to_vec()
    }
}

impl fmt::Display for Hash256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &b16_display(self.0))
    }
}

impl From<Hash256> for String {
    fn from(h: Hash256) -> Self {
        base16::encode_lower(&h.0)
    }
}

impl AsRef<[u8; 32]> for Hash256 {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

// Implement some traits for Sha256
impl PartialOrd for Hash256 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Hash256 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl Eq for Hash256 {}

impl AsRef<[u8]> for Hash256 {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Serialize for Hash256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Base16HashSerializer::serialize(self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for Hash256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Base16HashSerializer::deserialize(deserializer)
    }
}

impl std::hash::Hash for Hash256 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Simple traits that are based on the [`Hash256`] type
pub mod traits {
    use super::Hash256;

    /// The `Hashed` trait indicates, that the hash over some data was already calculated and saved.
    ///
    /// Datatypes that internally save their hash can implement this, so that the hash does not need to be re-calculated
    /// everytime we ask for it.
    /// Note: All types that implement `Hashed` automatically implement `Hashable`,
    /// since they can always return a copy of the internally saved hash.
    pub trait Hashed {
        /// Returns a reference to the pre-calculated hash of the datatype.
        fn hash(&self) -> &Hash256;
    }

    // TODO: Maybe rename this to something like "id_hash" ? As this is not the raw hash of the binary data... idk
    /// The `Hashable` trait indicates, that we can generate a [`Hash256`] from this type.
    ///
    /// Note: In general, we can calculate a `Hash256` from anything that implements `AsRef<[u8]>`,
    /// but in our scenario not all items have a hash that is identical to their binary representation.
    /// This trait is here to fix this issue, as implementers can define a complete custom method of
    /// how the object-hash should be calculated.
    pub trait Hashable {
        /// Calculates and returns the hash of the datatype
        fn calc_hash(&self) -> Hash256;
    }

    // All Types that implement 'Hashed' automatically implement 'Hashable'
    impl<T: Hashed> Hashable for T {
        fn calc_hash(&self) -> Hash256 {
            *self.hash()
        }
    }

    // Since the hash itself holds a hash...
    impl Hashed for Hash256 {
        fn hash(&self) -> &Hash256 {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::traits::*;
    use super::*;

    #[test]
    fn hash_sum_matches() {
        let h1 = Sha3_256::digest("hash-1".as_bytes()).to_vec();
        let h2 = Sha3_256::digest("hash-2".as_bytes()).to_vec();
        // Calculate hash by concatenation
        let mut concat = Vec::new();
        for byte in h1.iter() {
            concat.push(*byte);
        }
        for byte in h2.iter() {
            concat.push(*byte);
        }
        let res_hash = Sha3_256::digest(concat.as_slice());
        println!("{:?}", res_hash);
        // Calculate hash using hasher update
        let mut hasher = Sha3_256::new();
        hasher.update(h1);
        hasher.update(h2);
        let res_update = hasher.finalize();
        println!("{:?}", res_update);
        assert_eq!(res_hash, res_update);
    }

    #[test]
    fn hash_comparable() {
        let h1 = Sha3_256::digest("hash-1".as_bytes()).to_vec();
        let h2 = Sha3_256::digest("hash-2".as_bytes()).to_vec();
        assert_ne!(h1 < h2, h2 < h1);
        assert_ne!(h1 == h2, h2 < h1);
        assert_eq!(h1, h1);
        assert_eq!(h1 == h1, h2 == h2);
    }

    #[test]
    fn zero_hash() {
        assert_eq!(
            base16::encode_lower(&Hash256::zero()),
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(Hash256::zero().0, [0u8; 32]);
    }

    #[test]
    fn empty_hash() {
        assert_eq!(Hash256::empty(), Hash256::digest(b""));
        let hasher = Hasher::new();
        assert_eq!(Hash256::empty(), hasher.finalize());
    }

    #[test]
    fn hash_addition() {
        let h1 = Hash256::zero() + Hash256::empty();
        let h2 = Hash256::sum(&Hash256::zero(), &Hash256::empty());
        let h3 = Hash256::empty() + Hash256::zero();
        let h4 = Hash256::sum(&Hash256::empty(), &Hash256::zero());
        assert_eq!(h1, h2);
        assert_eq!(h3, h4);
        assert_ne!(h1, h3);
        assert_ne!(h2, h4);
        let mut hasher = Hasher::new();
        hasher.update(&Hash256::zero());
        hasher.update(&Hash256::empty());
        assert_eq!(h1, hasher.finalize());
    }

    #[test]
    fn default_hash() {
        assert_eq!(Hash256::zero(), Hash256::default());
    }

    #[test]
    fn digest_x00() {
        let msg = "random-message";
        let h1 = Hash256::digest_w_x00(&msg);
        let mut hasher = Hasher::new();
        hasher.update(b"\x00");
        hasher.update(&msg);
        let h2 = hasher.finalize();
        assert_eq!(h1, h2);
    }

    #[test]
    fn digest_x01() {
        let msg = "random-message";
        let h1 = Hash256::digest_w_x01(&msg);
        let mut hasher = Hasher::new();
        hasher.update(b"\x01");
        hasher.update(&msg);
        let h2 = hasher.finalize();
        assert_eq!(h1, h2);
    }

    #[test]
    fn generic_array_equals_slice() {
        let slice = [5u8; 32];
        let hash = Hash256::from(slice);
        assert_eq!(Hash256::digest(&hash), Hash256::digest(&slice));
    }

    #[test]
    fn calc_hash_macro() {
        let h = calc_hash!(
            &Hash256::zero(),
            &Hash256::empty(),
            b"\x01",
            &"foo",
            &[1u8, 2u8, 3u8]
        );
        let mut hasher = Hasher::new();
        hasher.update(&Hash256::zero());
        hasher.update(&Hash256::empty());
        hasher.update(b"\x01");
        hasher.update(&"foo");
        hasher.update(&[1u8, 2u8, 3u8]);
        assert_eq!(h, hasher.finalize());
    }

    #[test]
    fn hash_ordering() {
        let mut original = Vec::new();
        for i in 0..100u32 {
            original.push(calc_hash!(&i.to_be_bytes()));
        }
        let mut fb_type: Vec<Hash256> = original.to_vec();

        // Sort this
        original.sort_unstable();
        fb_type.sort_unstable();

        let transformed_back: Vec<Hash256> = fb_type.into_iter().collect();
        assert_eq!(original, transformed_back);
    }

    #[test]
    fn b16_encode_decode() {
        let hash = Hash256::empty();
        let encoded = base16::encode_lower(&hash);
        let decoded = base16::decode(&encoded).unwrap();
        let reconstructed = Hash256::try_from(&decoded).unwrap();
        assert_eq!(hash, reconstructed);
    }

    #[test]
    fn reset_hasher() {
        let mut hasher = Hasher::default();
        hasher.update(&Hash256::zero());
        hasher.update(&Hash256::zero());
        hasher.reset();
        let hash = hasher.finalize();
        assert_eq!(hash, Hash256::empty());
    }

    #[test]
    fn to_u64() {
        let hash = Hash256::empty();
        let res = 1025330622980758127;
        assert_eq!(hash.to_u64(), res);
        let into: u64 = hash.into();
        assert_eq!(into, res);
    }

    #[test]
    fn from_and_into_slice() {
        let hash = Hash256::empty();
        let slice = hash.into_slice();
        let reconstructed = Hash256::from(&slice);
        assert_eq!(reconstructed, Hash256::empty());
        let other = Hash256::from(reconstructed.as_ref());
        assert_eq!(reconstructed, other);
    }

    #[test]
    fn from_and_into_vec() -> Result<(), Box<dyn std::error::Error>> {
        let hash = Hash256::empty();
        let mut vec = hash.into_vec();
        let reconstructed = Hash256::try_from(&vec)?;
        assert_eq!(reconstructed, Hash256::empty());

        let reconstructed = Hash256::try_from(vec.as_slice())?;
        assert_eq!(reconstructed, Hash256::empty());

        vec.pop();
        let fail = Hash256::try_from(&vec);
        assert!(fail.is_err());
        let fail = Hash256::try_from(vec![]);
        assert!(fail.is_err());
        Ok(())
    }

    #[test]
    fn hashed() {
        let hash = Hash256::empty();
        assert_eq!(*hash.hash(), hash);
    }

    #[test]
    fn display() {
        assert_eq!(Hash256::empty().to_string(), "a7ffc6f8");
        assert_eq!(Hash256::zero().to_string(), "00000000");
        let hash = Hash256::empty();
        let full_string: String = hash.into();
        assert_ne!(
            hash.to_string(),
            full_string,
            "Hash's display method should not display full length string"
        );
    }

    #[test]
    fn serialize_hash() {
        let hash = Hash256::digest(b"hash");
        let out = serde_json::to_string(&hash).unwrap();
        assert_eq!(
            out,
            "\"d7333e98f53ddf1de70dd986f6a73f0c0d92f928458873b0d2a8a09ac22c191a\""
        );
    }

    #[test]
    fn deserialize_hash() {
        let json: Hash256 = serde_json::from_str(
            "\"d7333e98f53ddf1de70dd986f6a73f0c0d92f928458873b0d2a8a09ac22c191a\"",
        )
        .unwrap();
        let hash = Hash256::digest(b"hash");
        assert_eq!(json, hash);
    }

    #[test]
    fn debug_output() {
        let hash = Hash256::digest(b"hash");
        let dbg = format!("{hash:?}");
        assert_eq!(dbg, "Hash256 { bytes: [215, 51, 62, 152, 245, 61, 223, 29, 231, 13, 217, 134, 246, 167, 63, 12, 13, 146, 249, 40, 69, 136, 115, 176, 210, 168, 160, 154, 194, 44, 25, 26] }");
    }

    #[test]
    fn fb_generated_interfaces() {
        let bytes = [42u8; 32];
        let mut hash = Hash256::new(&bytes);
        assert_eq!(hash.0, bytes);

        for v in hash.bytes().iter() {
            assert_eq!(v, 42);
        }

        let other = [1u8; 32];
        hash.set_bytes(&other);
        assert_eq!(hash.0, other);
    }
}
