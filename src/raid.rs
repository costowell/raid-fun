use crate::{
    drive::Drive,
    generator::{FromPower, Gen},
};
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
}

impl State {
    /// Creates a new RAID instance
    pub fn new(drive_size: usize, num_drives: usize, level: RaidLevel) -> Self {
        let mut drives = Vec::with_capacity(num_drives);
        for _ in 0..num_drives {
            drives.push(Drive::random(drive_size));
        }

        Self {
            drives,
            drive_size,
            level,
        }
    }

    /// Marks a random drive as failed and returns a copy of the drive before failure
    pub fn fail_random(&mut self) -> Drive {
        let r = rand::rng().random_range(0..self.drives.len());
        let old_drive = self.drives[r].clone();
        self.drives[r].fail();
        old_drive
    }

    /// Randomly selects two distinct drives and marks them as failed
    pub fn fail_two_random(&mut self) -> (Drive, Drive) {
        let mut idxs: Vec<usize> = (0..self.drives.len()).into_iter().collect();
        idxs.shuffle(&mut rand::rng());
        let old_dx = self.drives[idxs[0]].clone();
        let old_dy = self.drives[idxs[1]].clone();
        self.drives[idxs[0]].fail();
        self.drives[idxs[1]].fail();
        (old_dx, old_dy)
    }

    /// Finds all drive indices who have failed
    pub fn find_failed(&self) -> Vec<usize> {
        self.drives
            .iter()
            .enumerate()
            .filter(|(_, x)| x.is_failed())
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
        let mut data = vec![0u8; self.drive_size];
        for (d, drive) in self.drives.iter().enumerate() {
            for i in 0..self.drive_size {
                // Q_i = D_xi * g^d
                data[i] ^= drive.byte_at(i) * Gen::from_power(d as u8);
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
        let mut data = vec![0u8; self.drive_size];
        for (d, drive) in self.drives.iter().enumerate() {
            for i in 0..self.drive_size {
                if !ignore.contains(&d) {
                    data[i] ^= drive.byte_at(i) * Gen::from_power(d as u8);
                }
            }
        }
        Drive::from_data(data)
    }

    /// Applies the generator n times to a drive
    pub fn apply_gen(&self, drive: &Drive, gn: Gen) -> Drive {
        assert_eq!(
            self.level,
            RaidLevel::Raid6,
            "Generator application is only available for RAID6"
        );

        let mut data = vec![0u8; self.drive_size];
        for i in 0..self.drive_size {
            data[i] = (drive.byte_at(i) * gn).value();
        }
        Drive::from_data(data)
    }

    /// Computes the generator power of A given an x and a y which is to be applied to `P + P_xy`
    /// Formally, this returns n where g^n = (g^y-x)/(g^y-x + 1)
    ///
    /// Source: Page 5, equations 17 and 19, https://www.kernel.org/pub/linux/kernel/people/hpa/raid6.pdf
    pub fn compute_a(&self, x: u8, y: u8) -> Gen {
        assert_eq!(
            self.level,
            RaidLevel::Raid6,
            "Computing A is only available for RAID6"
        );
        let x = x as i16;
        let y = y as i16;
        Gen::from_power(y - x) / (Gen::from_power(y - x) + 1)
    }

    /// Computes the generator power of B given an x and a y which is to be applied to `Q + Q_xy`
    /// Formally, this returns n where g^n = (g^-x)/(g^y-x + 1)
    ///
    /// Source: Page 5, equations 18 and 19, https://www.kernel.org/pub/linux/kernel/people/hpa/raid6.pdf
    pub fn compute_b(&self, x: u8, y: u8) -> Gen {
        assert_eq!(
            self.level,
            RaidLevel::Raid6,
            "Computing B is only available for RAID6"
        );
        let x = x as i16;
        let y = y as i16;
        Gen::from_power(-x) / (Gen::from_power(y - x) + 1)
    }
}

// Test module
pub mod tests {
    use super::*;

    /// Tests the reconstitution of data after a single drive failure (RAID5)
    pub fn raid5_normal_fail() {
        // Init with 32 1KiB drives
        let mut raid = State::new(1024, 32, RaidLevel::Raid5);

        // Compute P drive
        let p_drive = raid.p_parity();

        // Fail a random drive, save the original content
        let orig_drive = raid.fail_random();

        // Find failed drive
        let failed_drive = raid.find_failed();
        let failed_idx = *failed_drive
            .first()
            .expect("Should have found a failed drive");

        // Recompute the failed drive
        let recomp_drive = raid
            .p_parity_ignore_idxs(vec![failed_idx])
            .xor_drive(&p_drive);

        // The recomputed drive should be what the failed drive was originally
        assert_eq!(recomp_drive, orig_drive);

        // The original parity drive should now agree with all the XORed drives with the failed drive swapped out for the recomputed drive
        assert_eq!(
            p_drive,
            raid.p_parity_ignore_idxs(vec![failed_idx])
                .xor_drive(&recomp_drive)
        );
    }

    /// Tests the reconstitution of data after a single data and parity drive failure (RAID6)
    pub fn raid6_normal_and_p_drive_fail() {
        // Init with 32 1KiB drives
        let mut raid = State::new(1024, 32, RaidLevel::Raid6);

        // Generate p drive
        let orig_p_drive = raid.p_parity();

        // Generate q drive
        let q_drive = raid.q_parity();

        // Mark a random drive as failed, assume P drive has also failed
        let orig_data_drive = raid.fail_random();

        // Find failed drive (assume only one)
        let failed_drive = raid.find_failed();
        let failed_idx = *failed_drive
            .first()
            .expect("Should have found a failed drive");

        // Recompute failed data drive
        let qx_drive = raid.q_parity_ignore_idxs(vec![failed_idx]);
        let tmp = q_drive.xor_drive(&qx_drive);
        let g_inv = Gen::from_power(failed_idx as u8).inverse();
        let data_drive = raid.apply_gen(&tmp, g_inv);
        assert_eq!(orig_data_drive, data_drive);

        // Recompute P drive
        let p_drive = raid
            .p_parity_ignore_idxs(vec![failed_idx])
            .xor_drive(&data_drive);
        assert_eq!(orig_p_drive, p_drive);
    }

    /// Tests the reconstitution of data after two data drives fail (RAID6)
    pub fn raid6_two_normal_fail() {
        // Init with 32 1KiB drives
        let mut raid = State::new(1024, 32, RaidLevel::Raid6);

        // Generate p and q drive
        let p = raid.p_parity();
        let q = raid.q_parity();

        // Mark two random drives as failed
        let (orig_dx, orig_dy) = raid.fail_two_random();

        // Get indices
        let failed_drives = raid.find_failed();
        assert_eq!(failed_drives.len(), 2);
        let dx_idx = failed_drives[1];
        let dy_idx = failed_drives[0];

        // Compute constants A and B
        let a = raid.compute_a(dx_idx as u8, dy_idx as u8);
        let b = raid.compute_b(dx_idx as u8, dy_idx as u8);

        // Compute P and Q but ignoring D_x and D_y
        let p_xy = raid.p_parity_ignore_idxs(vec![dx_idx, dy_idx]);
        let q_xy = raid.q_parity_ignore_idxs(vec![dx_idx, dy_idx]);

        // Compute D_x and D_y
        let p_xor_p_xy = &p.xor_drive(&p_xy);
        let dx = raid
            .apply_gen(p_xor_p_xy, a)
            .xor_drive(&raid.apply_gen(&q.xor_drive(&q_xy), b));
        let dy = p_xor_p_xy.xor_drive(&dx);
        assert!(dx == orig_dx || dx == orig_dy);
        assert!(dy == orig_dx || dy == orig_dy);
    }
}
