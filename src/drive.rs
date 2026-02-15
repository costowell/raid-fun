use anyhow::{bail, Result};

/// Represents a hard drive with variable bytes
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Drive {
    data: Vec<u8>,
    failed: bool,
    formatted: bool,
}

impl Drive {
    /// Creates a drive filled with no data
    pub fn empty(size: usize) -> Self {
        Drive::from_data(vec![0u8; size])
    }

    /// Creates a drive from a vec of data
    pub fn from_data(data: Vec<u8>) -> Self {
        Self {
            data,
            failed: false,
            formatted: false,
        }
    }

    /// Returns true if the drive has not failed and is formatted
    pub fn writeable(&self) -> bool {
        !self.failed
    }

    fn writeable_result(&self) -> Result<()> {
        if self.writeable() {
            Ok(())
        } else {
            bail!("Failed to access drive, failed")
        }
    }

    /// Sets the drive's data
    pub fn set_data(&mut self, data: Vec<u8>) -> Result<()> {
        self.writeable_result()?;
        assert_eq!(data.len(), self.data.len());
        self.data = data;
        Ok(())
    }

    /// Marks a drive as failed
    pub fn fail(&mut self) {
        self.failed = true;
    }

    /// Returns whether the drive has failed
    pub fn has_failed(&self) -> bool {
        self.failed
    }

    /// Marks a drive as formatted
    pub fn format(&mut self) {
        self.formatted = true;
    }

    /// Returns whether the drive is formatted
    pub fn is_formatted(&self) -> bool {
        self.formatted
    }

    pub fn usable(&self) -> bool {
        !self.failed && self.formatted
    }

    /// Reads the byte at the specified offset
    pub fn read(&self, offset: usize) -> Result<u8> {
        self.writeable_result()?;
        Ok(self.data[offset])
    }

    /// Reads a slice of a specified length at a specified offset
    pub fn read_slice(&self, offset: usize, len: usize) -> Result<&[u8]> {
        self.writeable_result()?;
        Ok(&self.data[offset..(offset + len)])
    }

    /// Writes the byte at the specified offset
    pub fn write(&mut self, offset: usize, data: u8) -> Result<()> {
        self.writeable_result()?;
        self.data[offset] = data;
        Ok(())
    }

    /// Writes the slice at the specified offset
    pub fn write_slice(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        self.writeable_result()?;
        self.data[offset..data.len()].copy_from_slice(data);
        Ok(())
    }
}
