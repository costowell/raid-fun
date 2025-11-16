#[cfg(test)]
mod tests {
    use rand::Rng;
    use test_log::test;

    use crate::drive::Drive;

    /// Container for simulating RAID5
    struct RAID5<const SIZE: usize, const N: usize> {
        drives: [Drive<SIZE>; N],
    }

    impl<const SIZE: usize, const N: usize> RAID5<SIZE, N> {
        /// Creates a new RAID5 instance
        pub fn new() -> Self {
            // TODO: super ugly, find a better way
            let mut drives = [Drive::<SIZE>::empty(); N];
            for drive in &mut drives {
                drive.randomize();
                drive.set_cksum(drive.compute_cksum());
            }
            Self { drives }
        }

        /// Corrupts a random drive and returns its original data
        pub fn corrupt_random(&mut self) -> Drive<SIZE> {
            let r = rand::rng().random_range(0..N);
            let old_drive = self.drives[r].clone();
            self.drives[r].corrupt();
            old_drive
        }

        /// Creates a new drive by XORing the ith byte of every drive together
        pub fn xor_drives(&self) -> Drive<SIZE> {
            let mut data = [0u8; SIZE];
            for i in 0..SIZE {
                data[i] = self
                    .drives
                    .iter()
                    .map(|x| x.byte_at(i))
                    .reduce(|acc, x| acc ^ x)
                    .unwrap();
            }
            Drive::<SIZE>::from_data(data)
        }

        /// Creates a new drive by XORing the ith byte of every drive together, excluding one drive and including another
        pub fn xor_drives_ignore_include(
            &self,
            ignore: &Drive<SIZE>,
            include: &Drive<SIZE>,
        ) -> Drive<SIZE> {
            let mut data = [0u8; SIZE];
            for i in 0..SIZE {
                data[i] = self
                    .drives
                    .iter()
                    .filter(|x| *x != ignore)
                    .map(|x| x.byte_at(i))
                    .reduce(|acc, x| acc ^ x)
                    .unwrap()
                    ^ include.byte_at(i);
            }
            Drive::<SIZE>::from_data(data)
        }

        /// Finds all drives who's stored checksums do not agree with their computed checksums
        pub fn find_corrupted(&self) -> Vec<&Drive<SIZE>> {
            self.drives
                .iter()
                .filter(|x| x.cksum() != x.compute_cksum())
                .collect()
        }
    }

    /// Tests the detection of corruption in a data drive and the reconstitution of its data
    #[test]
    fn raid5_normal_corrupt() {
        // Init with 32 1KiB drives
        let mut raid5 = RAID5::<1024, 32>::new();

        // Compute P drive
        let p_drive = raid5.xor_drives();

        // Corrupt a random drive, save the original content
        let orig_drive = raid5.corrupt_random();

        // Now that a drive is corrupted, the newly computed parity drive should not equal the original parity drive
        assert_ne!(raid5.xor_drives(), p_drive);

        // Determine which drive got corrupted
        let corrupted_drive = raid5.find_corrupted();
        let corrupted_drive = corrupted_drive.first().unwrap();

        // Recompute the corrupted drive
        let recomp_drive = raid5.xor_drives_ignore_include(corrupted_drive, &p_drive);

        // The recomputed drive should be what the corrupted drive was originally
        assert_eq!(recomp_drive, orig_drive);

        // The original parity drive should now agree with all the XORed drives with the corrupted drive swapped out for the recomputed drive
        assert_eq!(
            p_drive,
            raid5.xor_drives_ignore_include(corrupted_drive, &recomp_drive)
        )
    }
}
