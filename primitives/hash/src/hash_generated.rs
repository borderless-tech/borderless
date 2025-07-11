// automatically generated by the FlatBuffers compiler, do not modify


// @generated

use core::mem;
use core::cmp::Ordering;

extern crate flatbuffers;
use self::flatbuffers::{EndianScalar, Follow};

// struct Hash256, aligned to 1
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq)]
pub struct Hash256(pub [u8; 32]);
impl Default for Hash256 { 
  fn default() -> Self { 
    Self([0; 32])
  }
}
impl core::fmt::Debug for Hash256 {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    f.debug_struct("Hash256")
      .field("bytes", &self.bytes())
      .finish()
  }
}

impl flatbuffers::SimpleToVerifyInSlice for Hash256 {}
impl<'a> flatbuffers::Follow<'a> for Hash256 {
  type Inner = &'a Hash256;
  #[inline]
  unsafe fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
    <&'a Hash256>::follow(buf, loc)
  }
}
impl<'a> flatbuffers::Follow<'a> for &'a Hash256 {
  type Inner = &'a Hash256;
  #[inline]
  unsafe fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
    flatbuffers::follow_cast_ref::<Hash256>(buf, loc)
  }
}
impl<'b> flatbuffers::Push for Hash256 {
    type Output = Hash256;
    #[inline]
    unsafe fn push(&self, dst: &mut [u8], _written_len: usize) {
        let src = ::core::slice::from_raw_parts(self as *const Hash256 as *const u8, Self::size());
        dst.copy_from_slice(src);
    }
}

impl<'a> flatbuffers::Verifiable for Hash256 {
  #[inline]
  fn run_verifier(
    v: &mut flatbuffers::Verifier, pos: usize
  ) -> Result<(), flatbuffers::InvalidFlatbuffer> {
    use self::flatbuffers::Verifiable;
    v.in_buffer::<Self>(pos)
  }
}

impl<'a> Hash256 {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    bytes: &[u8; 32],
  ) -> Self {
    let mut s = Self([0; 32]);
    s.set_bytes(bytes);
    s
  }

  pub fn bytes(&'a self) -> flatbuffers::Array<'a, u8, 32> {
    // Safety:
    // Created from a valid Table for this object
    // Which contains a valid array in this slot
    unsafe { flatbuffers::Array::follow(&self.0, 0) }
  }

  pub fn set_bytes(&mut self, items: &[u8; 32]) {
    // Safety:
    // Created from a valid Table for this object
    // Which contains a valid array in this slot
    unsafe { flatbuffers::emplace_scalar_array(&mut self.0, 0, items) };
  }

}

