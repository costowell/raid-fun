use base64::prelude::*;
use std::fmt::Display;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DriveError {
    #[error("Drive failed")]
    Failed,
    #[error("Drive unformatted")]
    Unformatted,
}

pub type Result<T> = std::result::Result<T, DriveError>;

/// Represents a hard drive with variable bytes
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Drive {
    data: Vec<u8>,
    failed: bool,
    formatted: bool,
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
        !self.failed && self.formatted
    }

    fn writeable_result(&self) -> Result<()> {
        if self.writeable() {
            Ok(())
        } else if self.failed {
            Err(DriveError::Failed)
        } else {
            Err(DriveError::Unformatted)
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

    /// Reads the byte at the specified offset
    pub fn read(&self, offset: usize) -> Result<u8> {
        self.writeable_result()?;
        Ok(self.data[offset])
    }

    /// Writes the byte at the specified offset
    pub fn write(&mut self, offset: usize, data: u8) -> Result<()> {
        self.writeable_result()?;
        self.data[offset] = data;
        Ok(())
    }
}
