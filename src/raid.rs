use crate::{drive::Drive, table::MTable};
use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidLevel {
    Raid5,
    Raid6,
}

/// State of the RAID simulation
pub struct State {
    drives: Vec<Drive>,
    drive_size: usize,
    level: RaidLevel,
    table: Option<MTable>,
}

impl State {
    /// Creates a new RAID instance
    pub fn new(drive_size: usize, num_drives: usize, level: RaidLevel) -> Self {
        let mut drives = Vec::with_capacity(num_drives);
        for _ in 0..num_drives {
            drives.push(Drive::random(drive_size));
        }

        let table = match level {
            RaidLevel::Raid6 => Some(MTable::new()),
            RaidLevel::Raid5 => None,
        };

        Self {
            drives,
            drive_size,
            level,
            table,
        }
    }

    /// Corrupts a random drive and returns its original data
    pub fn corrupt_random(&mut self) -> Drive {
        let r = rand::rng().random_range(0..self.drives.len());
        let old_drive = self.drives[r].clone();
        self.drives[r].corrupt();
        old_drive
    }

    /// Finds all drive indices whose stored checksums do not agree with their computed checksums
    pub fn find_corrupted(&self) -> Vec<usize> {
        self.drives
            .iter()
            .enumerate()
            .filter(|(_, x)| x.cksum() != x.compute_cksum())
            .map(|(i, _)| i)
            .collect()
    }

    /// Creates a new drive by XORing the ith byte of every drive together (P parity)
    pub fn p_parity(&self) -> Drive {
        let mut data = vec![0u8; self.drive_size];
        for drive in &self.drives {
            for i in 0..self.drive_size {
                data[i] ^= drive.byte_at(i);
            }
        }
        Drive::from_data(data)
    }

    /// Creates P parity excluding one drive by index and including another
    pub fn p_parity_ignore_include(&self, ignore_idx: usize, include: Option<&Drive>) -> Drive {
        let mut data = vec![0u8; self.drive_size];
        for i in 0..self.drive_size {
            data[i] = self
                .drives
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx != ignore_idx)
                .map(|(_, x)| x.byte_at(i))
                .reduce(|acc, x| acc ^ x)
                .unwrap()
                ^ include.map(|x| x.byte_at(i)).unwrap_or(0);
        }
        Drive::from_data(data)
    }

    /// Creates Q parity drive
    pub fn q_parity(&self) -> Drive {
        assert_eq!(
            self.level,
            RaidLevel::Raid6,
            "Q parity is only available for RAID6"
        );
        let table = self.table.as_ref().unwrap();

        let mut data = vec![0u8; self.drive_size];
        for (d, drive) in self.drives.iter().enumerate() {
            for i in 0..self.drive_size {
                data[i] ^= table.applyn(drive.byte_at(i), d);
            }
        }
        Drive::from_data(data)
    }

    /// Creates Q parity excluding one drive by index
    pub fn q_parity_ignore_idx(&self, ignore: usize) -> Drive {
        assert_eq!(
            self.level,
            RaidLevel::Raid6,
            "Q parity is only available for RAID6"
        );
        let table = self.table.as_ref().unwrap();

        let mut data = vec![0u8; self.drive_size];
        for (d, drive) in self.drives.iter().enumerate() {
            for i in 0..self.drive_size {
                if d == ignore {
                    data[i] ^= table.applyn(0, d);
                } else {
                    data[i] ^= table.applyn(drive.byte_at(i), d);
                }
            }
        }
        Drive::from_data(data)
    }

    /// Applies the generator n times to a drive
    pub fn apply_gen(&self, drive: &Drive, n: usize) -> Drive {
        assert_eq!(
            self.level,
            RaidLevel::Raid6,
            "Generator application is only available for RAID6"
        );
        let table = self.table.as_ref().unwrap();

        let mut data = vec![0u8; self.drive_size];
        for i in 0..self.drive_size {
            data[i] ^= table.applyn(drive.byte_at(i), n);
        }
        Drive::from_data(data)
    }
}

// Test module
pub mod tests {
    use super::*;

    /// Tests the detection of corruption in a data drive and the reconstitution of its data
    pub fn raid5_normal_corrupt() {
        // Init with 32 1KiB drives
        let mut raid = State::new(1024, 32, RaidLevel::Raid5);

        // Compute P drive
        let p_drive = raid.p_parity();

        // Corrupt a random drive, save the original content
        let orig_drive = raid.corrupt_random();

        // Now that a drive is corrupted, the newly computed parity drive should not equal the original parity drive
        assert_ne!(raid.p_parity(), p_drive);

        // Determine which drive got corrupted
        let corrupted_drive = raid.find_corrupted();
        let corrupted_idx = *corrupted_drive
            .first()
            .expect("Should have found a corrupted drive");

        // Recompute the corrupted drive
        let recomp_drive = raid.p_parity_ignore_include(corrupted_idx, Some(&p_drive));

        // The recomputed drive should be what the corrupted drive was originally
        assert_eq!(recomp_drive, orig_drive);

        // The original parity drive should now agree with all the XORed drives with the corrupted drive swapped out for the recomputed drive
        assert_eq!(
            p_drive,
            raid.p_parity_ignore_include(corrupted_idx, Some(&recomp_drive))
        );
    }

    /// Tests the detection of corruption in one data drive and one p drive and the reconstitution of their data (RAID6)
    pub fn raid6_normal_and_p_drive_corrupt() {
        // Init with 32 1KiB drives
        let mut raid = State::new(1024, 32, RaidLevel::Raid6);

        // Generate p drive
        let orig_p_drive = raid.p_parity();

        // Generate q drive
        let q_drive = raid.q_parity();

        // Corrupt random drive, assume P drive is also corrupted
        let orig_data_drive = raid.corrupt_random();

        // Find corrupted drive (assume only one)
        let corrupted_drive = raid.find_corrupted();
        let corrupted_idx = *corrupted_drive
            .first()
            .expect("Should have found a corrupted drive");

        // Recompute corrupted data drive
        let qx_drive = raid.q_parity_ignore_idx(corrupted_idx);
        let tmp = q_drive.xor_drive(&qx_drive);
        let g_inv = 255 - corrupted_idx;
        let data_drive = raid.apply_gen(&tmp, g_inv);
        assert_eq!(orig_data_drive, data_drive);

        // Recompute P drive
        let p_drive = raid.p_parity_ignore_include(corrupted_idx, Some(&data_drive));
        assert_eq!(orig_p_drive, p_drive);
    }

    /// Tests the detection of corruption in two data drives and the reconstitution of their data (RAID6)
    pub fn raid6_two_normal_corrupt() {}
}
