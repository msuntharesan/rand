// Copyright 2018 Developers of the Rand project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Helper functions for implementing `RngCore` functions.
//!
//! For cross-platform reproducibility, these functions all use Little Endian:
//! least-significant part first. For example, `next_u64_via_u32` takes `u32`
//! values `x, y`, then outputs `(y << 32) | x`. To implement `next_u32`
//! from `next_u64` in little-endian order, one should use `next_u64() as u32`.
//!
//! Byte-swapping (like the std `to_le` functions) is only needed to convert
//! to/from byte sequences, and since its purpose is reproducibility,
//! non-reproducible sources (e.g. `OsRng`) need not bother with it.

use crate::RngCore;
use core::cmp::min;

/// Implement `next_u64` via `next_u32`, little-endian order.
pub fn next_u64_via_u32<R: RngCore + ?Sized>(rng: &mut R) -> u64 {
    // Use LE; we explicitly generate one value before the next.
    let x = u64::from(rng.next_u32());
    let y = u64::from(rng.next_u32());
    (y << 32) | x
}

/// Implement `fill_bytes` via `next_u64` and `next_u32`, little-endian order.
///
/// The fastest way to fill a slice is usually to work as long as possible with
/// integers. That is why this method mostly uses `next_u64`, and only when
/// there are 4 or less bytes remaining at the end of the slice it uses
/// `next_u32` once.
pub fn fill_bytes_via_next<R: RngCore + ?Sized>(rng: &mut R, dest: &mut [u8]) {
    let mut left = dest;
    while left.len() >= 8 {
        let (l, r) = { left }.split_at_mut(8);
        left = r;
        let chunk: [u8; 8] = rng.next_u64().to_le_bytes();
        l.copy_from_slice(&chunk);
    }
    let n = left.len();
    if n > 4 {
        let chunk: [u8; 8] = rng.next_u64().to_le_bytes();
        left.copy_from_slice(&chunk[..n]);
    } else if n > 0 {
        let chunk: [u8; 4] = rng.next_u32().to_le_bytes();
        left.copy_from_slice(&chunk[..n]);
    }
}

macro_rules! fill_via_chunks {
    ($src:expr, $dst:expr, $ty:ty) => {{
        const SIZE: usize = core::mem::size_of::<$ty>();
        let chunk_size_u8 = min($src.len() * SIZE, $dst.len());
        let chunk_size = (chunk_size_u8 + SIZE - 1) / SIZE;

        let mut iter_src = $src.iter();
        let mut chunks = $dst.chunks_exact_mut(SIZE);
        for (chunk, n) in (&mut chunks).zip(&mut iter_src) {
            chunk.copy_from_slice(&n.to_le_bytes());
        }
        let rem = chunks.into_remainder();
        if let Some(n) = iter_src.next() {
            rem.copy_from_slice(&n.to_le_bytes()[..rem.len()]);
        }

        (chunk_size, chunk_size_u8)
    }};
}

/// Implement `fill_bytes` by reading chunks from the output buffer of a block
/// based RNG.
///
/// The return values are `(consumed_u32, filled_u8)`.
///
/// `filled_u8` is the number of filled bytes in `dest`, which may be less than
/// the length of `dest`.
/// `consumed_u32` is the number of words consumed from `src`, which is the same
/// as `filled_u8 / 4` rounded up.
///
/// # Example
/// (from `IsaacRng`)
///
/// ```ignore
/// fn fill_bytes(&mut self, dest: &mut [u8]) {
///     let mut read_len = 0;
///     while read_len < dest.len() {
///         if self.index >= self.rsl.len() {
///             self.isaac();
///         }
///
///         let (consumed_u32, filled_u8) =
///             impls::fill_via_u32_chunks(&mut self.rsl[self.index..],
///                                        &mut dest[read_len..]);
///
///         self.index += consumed_u32;
///         read_len += filled_u8;
///     }
/// }
/// ```
pub fn fill_via_u32_chunks(src: &[u32], dest: &mut [u8]) -> (usize, usize) {
    fill_via_chunks!(src, dest, u32)
}

/// Implement `fill_bytes` by reading chunks from the output buffer of a block
/// based RNG.
///
/// The return values are `(consumed_u64, filled_u8)`.
/// `filled_u8` is the number of filled bytes in `dest`, which may be less than
/// the length of `dest`.
/// `consumed_u64` is the number of words consumed from `src`, which is the same
/// as `filled_u8 / 8` rounded up.
///
/// See `fill_via_u32_chunks` for an example.
pub fn fill_via_u64_chunks(src: &[u64], dest: &mut [u8]) -> (usize, usize) {
    fill_via_chunks!(src, dest, u64)
}

/// Implement `next_u32` via `fill_bytes`, little-endian order.
pub fn next_u32_via_fill<R: RngCore + ?Sized>(rng: &mut R) -> u32 {
    let mut buf = [0; 4];
    rng.fill_bytes(&mut buf);
    u32::from_ne_bytes(buf)
}

/// Implement `next_u64` via `fill_bytes`, little-endian order.
pub fn next_u64_via_fill<R: RngCore + ?Sized>(rng: &mut R) -> u64 {
    let mut buf = [0; 8];
    rng.fill_bytes(&mut buf);
    u64::from_ne_bytes(buf)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fill_via_u32_chunks() {
        let src = [1, 2, 3];
        let mut dst = [0u8; 11];
        assert_eq!(fill_via_u32_chunks(&src, &mut dst), (3, 11));
        assert_eq!(dst, [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0]);

        let mut dst = [0u8; 13];
        assert_eq!(fill_via_u32_chunks(&src, &mut dst), (3, 12));
        assert_eq!(dst, [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0]);

        let mut dst = [0u8; 5];
        assert_eq!(fill_via_u32_chunks(&src, &mut dst), (2, 5));
        assert_eq!(dst, [1, 0, 0, 0, 2]);
    }

    #[test]
    fn test_fill_via_u64_chunks() {
        let src = [1, 2];
        let mut dst = [0u8; 11];
        assert_eq!(fill_via_u64_chunks(&src, &mut dst), (2, 11));
        assert_eq!(dst, [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0]);

        let mut dst = [0u8; 17];
        assert_eq!(fill_via_u64_chunks(&src, &mut dst), (2, 16));
        assert_eq!(dst, [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0]);

        let mut dst = [0u8; 5];
        assert_eq!(fill_via_u64_chunks(&src, &mut dst), (1, 5));
        assert_eq!(dst, [1, 0, 0, 0, 0]);
    }
}
