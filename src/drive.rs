use base64::prelude::*;
use rand::prelude::*;
use std::fmt::Display;

/// Represents a hard drive with variable bytes
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Drive {
    data: Vec<u8>,
    failed: bool,
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

    /// Creates a drive from a vec of data
    pub fn from_data(data: Vec<u8>) -> Self {
        Self {
            data,
            failed: false,
        }
    }

    /// Returns the byte at a given index
    pub fn byte_at(&self, idx: usize) -> u8 {
        self.panic_failed();
        self.data[idx]
    }

    /// Marks a drive as failed
    pub fn fail(&mut self) {
        self.failed = true;
    }

    /// Returns whether the drive has failed
    pub fn is_failed(&self) -> bool {
        self.failed
    }

    /// Creates a new drive by XORing each byte with another drive
    pub fn xor_drive(&self, other: &Drive) -> Drive {
        self.panic_failed();
        let mut data = self.data.clone();
        for i in 0..data.len() {
            data[i] ^= other.byte_at(i);
        }
        Drive::from_data(data)
    }

    /// Helper function to panic if the drive has failed
    fn panic_failed(&self) {
        if self.failed {
            panic!("Drive has failed! Data inaccessible!")
        }
    }
}
