use rand::prelude::*;
use std::fmt::Display;

use base64::prelude::*;

/// Represents a hard drive with N bytes
///
/// Technically, this drive represent N+1 bytes because of the checksum
/// This can be easily be included in the data slice, but adds annoying bounds checking and ultimately does not serve the purpose of education.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Drive<const N: usize> {
    data: [u8; N],
    cksum: u8,
}

impl<const N: usize> Display for Drive<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Drive<{}>({})", N, BASE64_STANDARD.encode(self.data))
    }
}

impl<const N: usize> Drive<N> {
    /// Creates a drive filled with random data
    pub fn random() -> Self {
        let mut data = [0u8; N];
        rand::rng().fill_bytes(&mut data);
        Drive::from_data(data)
    }
    /// Creates a drive filled with 0s
    pub fn empty() -> Self {
        Self {
            data: [0u8; N],
            cksum: 0,
        }
    }
    /// Creates a drive from a slice of data
    pub fn from_data(data: [u8; N]) -> Self {
        let mut d = Self { data, cksum: 0 };
        d.cksum = d.compute_cksum();
        d
    }

    /// Computes the checksum from the current data.
    /// This is explicitly NOT the stored checksum.
    pub fn compute_cksum(&self) -> u8 {
        let mut cksum = 0;
        for d in self.data {
            cksum ^= d;
        }
        cksum
    }
    /// Sets the stored checksum
    pub fn set_cksum(&mut self, cksum: u8) {
        self.cksum = cksum
    }
    /// Returns the stored checksum
    pub fn cksum(&self) -> u8 {
        self.cksum
    }
    /// Returns the byte at a given index
    pub fn byte_at(&self, idx: usize) -> u8 {
        self.data[idx]
    }
    /// Corrupts a drive by adding 1 to all bytes
    /// This does NOT update the stored checksum
    ///
    /// Why not just randomize the data?
    /// It is quite improbable, and basically impossible at large drive sizes, that the drive does not change.
    /// However, I'm trying to maintain a *little* determinism by guaranteeing the drive's data is changed.
    /// For the time being, the way the data is corrupted isn't important.
    pub fn corrupt(&mut self) {
        for byte in &mut self.data {
            *byte = (((*byte as u16) + 1) % 256) as u8
        }
    }
    /// Overwrites the drive with random
    /// This does NOT update the stored checksum
    pub fn randomize(&mut self) {
        rand::rng().fill_bytes(&mut self.data);
    }
    /// Overwrites the drive with 0s
    /// This does NOT update the stored checksum
    pub fn erase(&mut self) {
        self.data = [0u8; N];
    }
}
