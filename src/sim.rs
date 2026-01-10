use std::ops::Not;

use rand::seq::IteratorRandom;

use crate::drive::{self, Drive, DriveError};

use thiserror::Error;

const P_INDEX: usize = 0;
const Q_INDEX: usize = 1;

#[derive(Error, Debug)]
pub enum RaidError {
    #[error("drive error")]
    DriveError(#[from] DriveError),
    #[error("offset of {0} bigger than array")]
    OffsetTooLarge(usize),
    #[error("array failed")]
    Failed,
}

type Result<T> = std::result::Result<T, RaidError>;

#[derive(Debug, Eq, PartialEq)]
pub enum RaidMode {
    Raid5,
    Raid6,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RaidState {
    /// Array has not been initialized yet
    Uninit,
    /// Array is functioning as normal
    Ok,
    /// One or more drives has failed, extra computation is needed to retrieve some data
    Degraded,
    /// Too many drives have failed, data has been lost
    Failed,
}

#[derive(Debug)]
pub struct RaidSim {
    drives: Vec<Drive>,
    drive_size: usize,
    mode: RaidMode,
    state: RaidState,
}

impl RaidSim {
    /// Creates a new instance of a Raid Simulation
    pub fn new(mode: RaidMode, num_drives: usize, drive_size: usize) -> Self {
        RaidSim {
            drives: (0..num_drives)
                .into_iter()
                .map(|_| Drive::empty(drive_size))
                .collect(),
            state: RaidState::Uninit,
            drive_size,
            mode,
        }
    }

    /// Gets the total number of bytes storable in the array
    pub fn size(&self) -> usize {
        self.data_drives().count() * self.drive_size
    }

    /// Gets the current state of the array
    pub fn state(&self) -> RaidState {
        self.state.clone()
    }

    /// Initializes the array by formatting all drives
    pub fn init(&mut self) -> Result<()> {
        for d in &mut self.drives {
            d.format();
        }
        self.state = RaidState::Ok;
        Ok(())
    }

    /// Writes a byte at a specific offset in the array
    pub fn write(&mut self, offset: usize, data: u8) -> Result<()> {
        if offset >= self.size() {
            return Err(RaidError::OffsetTooLarge(offset));
        }
        if self.state == RaidState::Failed {
            return Err(RaidError::Failed);
        }
        let old_data = self.read(offset).unwrap();
        let drive_offset = offset % self.drive_size;
        let drive_index = offset / self.drive_size;
        let (_, drive) = self.data_drives_mut().nth(drive_index).unwrap();
        drive.write(drive_offset, data)?;
        let p_parity = self.p_parity_mut();
        p_parity.write(drive_offset, p_parity.read(drive_offset)? ^ old_data ^ data)?;
        Ok(())
    }

    /// Reads a byte at a specific offset in the array
    pub fn read(&self, offset: usize) -> Result<u8> {
        if offset >= self.size() {
            return Err(RaidError::OffsetTooLarge(offset));
        }
        if self.state == RaidState::Failed {
            return Err(RaidError::Failed);
        }
        let drive_offset = offset % self.drive_size;
        let drive_index = offset / self.drive_size;
        let (abs_idx, drive) = self.data_drives().nth(drive_index).unwrap();
        if drive.has_failed() {
            let data = self
                .data_drives()
                .filter(|(i, _)| *i != abs_idx)
                .map(|(_, d)| d.read(drive_offset))
                .collect::<drive::Result<Vec<u8>>>()?
                .into_iter()
                .reduce(|acc, x| acc ^ x)
                .unwrap()
                ^ self.p_parity().read(drive_offset)?;
            Ok(data)
        } else {
            Ok(drive.read(drive_offset)?)
        }
    }

    /// Returns an immutable reference to the drive used for P parity
    pub fn p_parity(&self) -> &Drive {
        &self.drives[P_INDEX]
    }
    /// Returns a mutable reference to the drive used for P parity
    fn p_parity_mut(&mut self) -> &mut Drive {
        &mut self.drives[P_INDEX]
    }

    /// Updates the P parity drive to be consistent with the current array
    fn write_p_parity(&mut self) -> Result<()> {
        let mut p_drive_data = vec![0u8; self.drive_size];
        for (_, d) in self.data_drives() {
            for i in 0..self.drive_size {
                p_drive_data[i] ^= d.read(i)?;
            }
        }
        self.p_parity_mut().set_data(p_drive_data)?;
        Ok(())
    }

    /// Returns an iterator of tuples (I, D) where I is the absolute index in the drives array and D is an immutable reference to the corresponding data drive
    pub fn data_drives(&self) -> impl Iterator<Item = (usize, &Drive)> {
        let start = match self.mode {
            RaidMode::Raid5 => 1,
            RaidMode::Raid6 => 2,
        };
        self.drives[start..]
            .iter()
            .enumerate()
            .map(move |(i, d)| (i + start, d))
    }
    /// Returns an iterator of tuples (I, D) where I is the absolute index in the drives array and D is a mutable reference to the corresponding data drive
    fn data_drives_mut(&mut self) -> impl Iterator<Item = (usize, &mut Drive)> {
        let start = match self.mode {
            RaidMode::Raid5 => 1,
            RaidMode::Raid6 => 2,
        };
        self.drives[start..]
            .iter_mut()
            .enumerate()
            .map(move |(i, d)| (i + start, d))
    }

    /// Returns an iterator of immutable references to drives that have failed
    pub fn failed(&self) -> impl Iterator<Item = &Drive> {
        self.drives
            .iter()
            .filter_map(|d| d.has_failed().then_some(d))
    }
    /// Returns an iterator of immutable references to drives that are unformatted
    pub fn unformatted(&self) -> impl Iterator<Item = &Drive> {
        self.drives
            .iter()
            .filter_map(|d| d.is_formatted().not().then_some(d))
    }
    /// Returns an iterator of immutable references to drives that haven't failed
    pub fn not_failed(&self) -> impl Iterator<Item = &Drive> {
        self.drives
            .iter()
            .filter_map(|d| d.has_failed().not().then_some(d))
    }
    /// Returns an iterator of mutable references to drives that haven't failed
    pub fn not_failed_mut(&mut self) -> impl Iterator<Item = &mut Drive> {
        self.drives
            .iter_mut()
            .filter_map(|d| d.has_failed().not().then_some(d))
    }
    /// Chooses a random drive that hasn't failed yet and marks it as failed
    pub fn fail_random(&mut self) {
        let drive = self.not_failed_mut().choose(&mut rand::rng()).unwrap();
        drive.fail();
        self.update_state();
    }
    /// Chooses a random data drive that hasn't failed yet and marks it as failed
    pub fn fail_random_data(&mut self) {
        let drive = self
            .data_drives_mut()
            .filter_map(|(_, d)| d.has_failed().not().then_some(d))
            .choose(&mut rand::rng())
            .unwrap();
        drive.fail();
        self.update_state();
    }
    /// Mark the P parity drive as failed
    pub fn fail_p_parity(&mut self) {
        self.p_parity_mut().fail();
        self.update_state();
    }
    /// Updates the state of the array to match
    pub fn update_state(&mut self) {
        let count = self.failed().count() + self.unformatted().count();
        if count > 2 || (count > 1 && self.mode == RaidMode::Raid5) {
            self.state = RaidState::Failed;
        } else if count > 0 {
            self.state = RaidState::Degraded;
        } else {
            self.state = RaidState::Ok;
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    const NUM_DRIVES: usize = 16;
    const DRIVE_SIZE: usize = 1024;

    fn init_random(mode: RaidMode) -> (RaidSim, Vec<u8>) {
        let mut sim = RaidSim::new(mode, NUM_DRIVES, DRIVE_SIZE);
        let mut data = vec![0u8; sim.size()];
        rand::rng().fill(data.as_mut_slice());
        sim.init().expect("Shit");
        for i in 0..sim.size() {
            sim.write(i, data[i]).unwrap();
        }
        (sim, data)
    }

    fn assert_sim_equal(sim: &RaidSim, data: &Vec<u8>) {
        for i in 0..sim.size() {
            if sim.read(i).unwrap() != data[i] {
                panic!(
                    "sim.read(i) != data[i], i={}, {} != {}",
                    i,
                    sim.read(i).unwrap(),
                    data[i]
                );
            }
        }
    }

    #[test]
    fn raid5_test_init() {
        let (sim, data) = init_random(RaidMode::Raid5);
        assert_sim_equal(&sim, &data);
        assert_eq!(sim.state(), RaidState::Ok);
    }

    #[test]
    fn raid5_one_data_drive_failure() {
        let (mut sim, data) = init_random(RaidMode::Raid5);
        sim.fail_random_data();
        assert_eq!(sim.state(), RaidState::Degraded);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid5_p_parity_failure() {
        let (mut sim, data) = init_random(RaidMode::Raid5);
        sim.fail_p_parity();
        assert_eq!(sim.state(), RaidState::Degraded);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid5_one_data_drive_and_p_parity_failure() {
        let (mut sim, _) = init_random(RaidMode::Raid5);
        sim.fail_random_data();
        sim.fail_p_parity();
        assert_eq!(sim.state(), RaidState::Failed);
        assert!(sim.write(0, 0).is_err());
        assert!(sim.read(0).is_err());
    }

    #[test]
    fn raid5_two_data_drive_failure() {
        let (mut sim, _) = init_random(RaidMode::Raid5);
        sim.fail_random_data();
        sim.fail_random_data();
        assert_eq!(sim.state(), RaidState::Failed);
        assert!(sim.write(0, 0).is_err());
        assert!(sim.read(0).is_err());
    }

    #[test]
    fn raid5_unformatted_still_degraded() {
        let (mut sim, _) = init_random(RaidMode::Raid5);
        assert_eq!(sim.state(), RaidState::Ok);
        sim.fail_random();
        assert_eq!(sim.state(), RaidState::Degraded);
        sim.replace_failed_drives();
        assert_eq!(sim.state(), RaidState::Degraded);
    }
}
