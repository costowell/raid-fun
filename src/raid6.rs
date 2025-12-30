#[cfg(test)]
mod tests {
    use rand::Rng;
    use test_log::test;

    use crate::{drive::Drive, table::MTable};

    /// Container for simulating RAID5
    struct RAID6<const SIZE: usize, const N: usize> {
        drives: [Drive<SIZE>; N],
        table: MTable,
    }

    impl<const SIZE: usize, const N: usize> RAID6<SIZE, N> {
        /// Creates a new RAID5 instance
        pub fn new() -> Self {
            // TODO: super ugly, find a better way
            let mut drives = [Drive::<SIZE>::empty(); N];
            for drive in &mut drives {
                drive.randomize();
                drive.set_cksum(drive.compute_cksum());
            }
            Self {
                drives: drives,
                table: MTable::new(),
            }
        }

        /// Creates a new drive by XORing the ith byte of every drive together
        /// Typically to compute the P drive
        pub fn xor_drives(&self) -> Drive<SIZE> {
            let mut data = [0u8; SIZE];
            for drive in self.drives {
                for i in 0..SIZE {
                    data[i] ^= drive.byte_at(i);
                }
            }
            Drive::<SIZE>::from_data(data)
        }

        /// Creates a new drive by XORing the ith byte of every drive together, excluding one drive and including another
        pub fn xor_drives_ignore_idx_include_drive(
            &self,
            ignore: usize,
            include: &Drive<SIZE>,
        ) -> Drive<SIZE> {
            let mut data = [0u8; SIZE];
            for i in 0..SIZE {
                data[i] = self
                    .drives
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| *idx != ignore)
                    .map(|(_, x)| x.byte_at(i))
                    .reduce(|acc, x| acc ^ x)
                    .unwrap()
                    ^ include.byte_at(i);
            }
            Drive::<SIZE>::from_data(data)
        }

        pub fn apply_gen(&self, drive: &Drive<SIZE>, n: usize) -> Drive<SIZE> {
            let mut data = [0u8; SIZE];
            for i in 0..SIZE {
                data[i] ^= self.table.applyn(drive.byte_at(i), n);
            }
            Drive::from_data(data)
        }

        pub fn q_drive_ignore_idx(&self, ignore: usize) -> Drive<SIZE> {
            let mut data = [0u8; SIZE];
            for (d, drive) in self.drives.iter().enumerate() {
                for i in 0..SIZE {
                    if d == ignore {
                        data[i] ^= self.table.applyn(0, d);
                    } else {
                        data[i] ^= self.table.applyn(drive.byte_at(i), d);
                    }
                }
            }
            Drive::<SIZE>::from_data(data)
        }

        /// Creates a new drive by XORing the ith byte of every drive together after multiplying it by a generator of GF(2^8) i many times
        pub fn q_drive(&self) -> Drive<SIZE> {
            let mut data = [0u8; SIZE];
            for (d, drive) in self.drives.iter().enumerate() {
                for i in 0..SIZE {
                    data[i] ^= self.table.applyn(drive.byte_at(i), d);
                }
            }
            Drive::<SIZE>::from_data(data)
        }

        /// Corrupts a random drive and returns its original data
        pub fn corrupt_random(&mut self) -> Drive<SIZE> {
            let r = rand::rng().random_range(0..N);
            let old_drive = self.drives[r].clone();
            self.drives[r].corrupt();
            old_drive
        }

        /// Finds all drive indices who's stored checksums do not agree with their computed checksums
        pub fn find_corrupted(&self) -> Vec<usize> {
            self.drives
                .iter()
                .enumerate()
                .filter(|(_, x)| x.cksum() != x.compute_cksum())
                .map(|(i, _)| i)
                .collect()
        }
    }

    /// Tests the detection of corruption in one data drive and one p drive and the reconstitution of their data
    #[test]
    fn raid6_normal_and_p_drive_corrupt() {
        // Init with 32 1KiB drives
        let mut raid6 = RAID6::<1024, 32>::new();

        // Generate p drive
        let orig_p_drive = raid6.xor_drives();

        // Generate q drive
        let q_drive = raid6.q_drive();

        // Corrupt random drive, assume P drive is also corrupted
        let orig_data_drive = raid6.corrupt_random();

        // Find corrupted drive (assume only one)
        let corrupted_drive = raid6.find_corrupted();
        let corrupted_idx = *corrupted_drive.first().unwrap();

        // Recompute corrupted data drive
        let qx_drive = raid6.q_drive_ignore_idx(corrupted_idx);
        let tmp = q_drive.xor_drive(&qx_drive);
        let g_inv = 255 - corrupted_idx;
        let data_drive = raid6.apply_gen(&tmp, g_inv);
        assert_eq!(orig_data_drive, data_drive);

        // Recompute P drive
        let p_drive = raid6.xor_drives_ignore_idx_include_drive(corrupted_idx, &data_drive);
        assert_eq!(orig_p_drive, p_drive);
    }
    /// Tests the detection of corruption in two data drives and the reconstitution of their data
    #[test]
    fn raid6_two_normal_corrupt() {}
}
