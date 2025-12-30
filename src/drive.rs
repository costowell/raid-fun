use base64::prelude::*;
use rand::prelude::*;
use std::fmt::Display;

/// Represents a hard drive with variable bytes
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Drive {
    data: Vec<u8>,
    cksum: u8,
}

impl Display for Drive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Drive<{}>({})",
            self.data.len(),
            BASE64_STANDARD.encode(&self.data)
        )
    }
}

impl Drive {
    /// Creates a drive filled with random data
    pub fn random(size: usize) -> Self {
        let mut data = vec![0u8; size];
        rand::rng().fill_bytes(&mut data);
        Drive::from_data(data)
    }

    /// Creates a drive filled with 0s
    pub fn empty(size: usize) -> Self {
        Self {
            data: vec![0u8; size],
            cksum: 0,
        }
    }

    /// Creates a drive from a vec of data
    pub fn from_data(data: Vec<u8>) -> Self {
        let mut d = Self { data, cksum: 0 };
        d.cksum = d.compute_cksum();
        d
    }

    /// Returns the size of the drive
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Computes the checksum from the current data.
    pub fn compute_cksum(&self) -> u8 {
        let mut cksum = 0;
        for d in &self.data {
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
    ///
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

    /// Overwrites the drive with random data
    /// This does NOT update the stored checksum
    pub fn randomize(&mut self) {
        rand::rng().fill_bytes(&mut self.data);
    }

    /// Overwrites the drive with 0s
    /// This does NOT update the stored checksum
    pub fn erase(&mut self) {
        self.data.fill(0);
    }

    /// Creates a new drive by XORing each byte with another drive
    pub fn xor_drive(&self, other: &Drive) -> Drive {
        let mut data = self.data.clone();
        for i in 0..data.len() {
            data[i] ^= other.byte_at(i);
        }
        Drive::from_data(data)
    }
}
