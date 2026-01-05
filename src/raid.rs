use crate::{drive::Drive, table::MTable};
use rand::{seq::SliceRandom, Rng};

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

    /// Randomly selects two distinct drives and corrupts them
    pub fn corrupt_two_random(&mut self) -> (Drive, Drive) {
        let mut idxs: Vec<usize> = (0..self.drives.len()).into_iter().collect();
        idxs.shuffle(&mut rand::rng());
        let old_dx = self.drives[idxs[0]].clone();
        let old_dy = self.drives[idxs[1]].clone();
        self.drives[idxs[0]].corrupt();
        self.drives[idxs[1]].corrupt();
        (old_dx, old_dy)
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
    pub fn p_parity_ignore_idxs(&self, ignore: Vec<usize>) -> Drive {
        let mut data = vec![0u8; self.drive_size];
        for i in 0..self.drive_size {
            data[i] = self
                .drives
                .iter()
                .enumerate()
                .filter(|(idx, _)| !ignore.contains(idx))
                .map(|(_, x)| x.byte_at(i))
                .reduce(|acc, x| acc ^ x)
                .unwrap()
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
                data[i] ^= table.applyn(drive.byte_at(i), d as i16);
            }
        }
        Drive::from_data(data)
    }

    /// Creates Q parity excluding one drive by index
    pub fn q_parity_ignore_idxs(&self, ignore: Vec<usize>) -> Drive {
        assert_eq!(
            self.level,
            RaidLevel::Raid6,
            "Q parity is only available for RAID6"
        );
        let table = self.table.as_ref().unwrap();

        let mut data = vec![0u8; self.drive_size];
        for (d, drive) in self.drives.iter().enumerate() {
            for i in 0..self.drive_size {
                if !ignore.contains(&d) {
                    data[i] ^= table.applyn(drive.byte_at(i), d as i16);
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
            data[i] ^= table.applyn(drive.byte_at(i), n as i16);
        }
        Drive::from_data(data)
    }

    /// Computes the generator power of A given an x and a y which is to be applied to `P + P_xy`
    /// Formally, this returns n where g^n = (g^y-x)/(g^y-x + 1)
    ///
    /// Source: Page 5, equations 17 and 19, https://www.kernel.org/pub/linux/kernel/people/hpa/raid6.pdf
    pub fn compute_a_power(&self, x: u8, y: u8) -> u8 {
        assert_eq!(
            self.level,
            RaidLevel::Raid6,
            "Computing A is only available for RAID6"
        );
        let table = self.table.as_ref().unwrap();
        let x = x as i16;
        let y = y as i16;

        let power = table.generator_power(table.applyn(1, y-x) ^ 1);
        ((y - x) - power).rem_euclid(255) as u8
    }

    /// Computes the generator power of B given an x and a y which is to be applied to `Q + Q_xy`
    /// Formally, this returns n where g^n = (g^-x)/(g^y-x + 1)
    ///
    /// Source: Page 5, equations 18 and 19, https://www.kernel.org/pub/linux/kernel/people/hpa/raid6.pdf
    pub fn compute_b_power(&self, x: u8, y: u8) -> u8 {
        assert_eq!(
            self.level,
            RaidLevel::Raid6,
            "Computing B is only available for RAID6"
        );
        let table = self.table.as_ref().unwrap();
        let x = x as i16;
        let y = y as i16;

        let power = table.generator_power(table.applyn(1, y-x) ^ 1);
        (-x + -power).rem_euclid(255) as u8
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
        let recomp_drive = raid.p_parity_ignore_idxs(vec![corrupted_idx]).xor_drive(&p_drive);

        // The recomputed drive should be what the corrupted drive was originally
        assert_eq!(recomp_drive, orig_drive);

        // The original parity drive should now agree with all the XORed drives with the corrupted drive swapped out for the recomputed drive
        assert_eq!(
            p_drive,
            raid.p_parity_ignore_idxs(vec![corrupted_idx]).xor_drive(&recomp_drive)
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
        let qx_drive = raid.q_parity_ignore_idxs(vec![corrupted_idx]);
        let tmp = q_drive.xor_drive(&qx_drive);
        let g_inv = 255 - corrupted_idx;
        let data_drive = raid.apply_gen(&tmp, g_inv);
        assert_eq!(orig_data_drive, data_drive);

        // Recompute P drive
        let p_drive = raid.p_parity_ignore_idxs(vec![corrupted_idx]).xor_drive(&data_drive);
        assert_eq!(orig_p_drive, p_drive);
    }

    /// Tests the detection of corruption in two data drives and the reconstitution of their data (RAID6)
    pub fn raid6_two_normal_corrupt() {
        // Init with 32 1KiB drives
        let mut raid = State::new(1024, 32, RaidLevel::Raid6);

        // Generate p and q drive
        let p = raid.p_parity();
        let q = raid.q_parity();

        // Corrupt two random drives
        let (orig_dx, orig_dy) = raid.corrupt_two_random();

        // Get indices
        let corrupted_drives = raid.find_corrupted();
        assert_eq!(corrupted_drives.len(), 2);
        let dx_idx = corrupted_drives[1];
        let dy_idx = corrupted_drives[0];

        let a = raid.compute_a_power(dx_idx as u8, dy_idx as u8) as usize;
        let b = raid.compute_b_power(dx_idx as u8, dy_idx as u8) as usize;

        let p_xy = raid.p_parity_ignore_idxs(vec![dx_idx, dy_idx]);
        let q_xy = raid.q_parity_ignore_idxs(vec![dx_idx, dy_idx]);

        let p_xor_p_xy = &p.xor_drive(&p_xy);
        let dx = raid.apply_gen(p_xor_p_xy, a).xor_drive(&raid.apply_gen(&q.xor_drive(&q_xy), b));
        let dy = p_xor_p_xy.xor_drive(&dx);
        assert!(dx == orig_dx || dx == orig_dy);
        assert!(dy == orig_dx || dy == orig_dy);
    }
}
