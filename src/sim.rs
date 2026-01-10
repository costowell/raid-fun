use std::ops::Not;

use rand::seq::IteratorRandom;

use crate::{
    drive::{self, Drive, DriveError},
    generator::{FromPower, Gen},
};

use anyhow::{Context, Result};
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
    #[error("drives must be replaced before being repaired")]
    DrivesNeedReplaced,
    #[error("array not initialized")]
    NotInitialized,
}

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
}

impl RaidSim {
    /// Creates a new instance of a Raid Simulation
    pub fn new(mode: RaidMode, num_drives: usize, drive_size: usize) -> Self {
        RaidSim {
            drives: (0..num_drives)
                .into_iter()
                .map(|_| Drive::empty(drive_size))
                .collect(),
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
        let unformatted = self.unformatted().count();
        let count = self.failed().count() + unformatted;
        if unformatted == self.drives.len() {
            RaidState::Uninit
        } else if count > 2 || (count > 1 && self.mode == RaidMode::Raid5) {
            RaidState::Failed
        } else if count > 0 {
            RaidState::Degraded
        } else {
            RaidState::Ok
        }
    }

    /// Initializes the array by formatting all drives
    pub fn init(&mut self) -> Result<()> {
        for d in &mut self.drives {
            d.format();
        }
        Ok(())
    }

    /// Writes a byte at a specific offset in the array
    pub fn write(&mut self, offset: usize, data: u8) -> Result<()> {
        if offset >= self.size() {
            return Err(RaidError::OffsetTooLarge(offset).into());
        }
        if self.state() == RaidState::Failed {
            return Err(RaidError::Failed.into());
        }
        let old_data = self.read(offset).unwrap();
        let drive_offset = offset % self.drive_size;
        let drive_index = offset / self.drive_size;
        let drive = self.data_drives_mut().nth(drive_index).unwrap();
        if !drive.has_failed() {
            drive.write(drive_offset, data)?;
        }

        // Compute new P parity
        let p_parity = self.p_parity_mut();
        if !p_parity.has_failed() {
            // Formally, if p is the original P parity byte and p_k is the new P parity byte where d_k (the byte on drive k) becomes d'
            // Then it follows that
            // p   = d_0 + d_1 + ... + d_n-1
            // p_k = d_0 + d_1 + ... + d' + ... + d_n-1
            // Then
            // p + p_k = d_k + d_'
            // Therefore
            // p_k = p + d_k + d'
            // Which means XORing the P parity byte, the old data on the drive, and the new data will yield the new P parity byte
            p_parity.write(drive_offset, p_parity.read(drive_offset)? ^ old_data ^ data)?;
        }

        // Compute new Q parity
        let q_parity = self.q_parity();
        if self.mode == RaidMode::Raid6 && !q_parity.has_failed() {
            let q_parity = self.q_parity_mut();
            // Formally, if q is the original Q parity byte and q_k is the new Q parity byte where d_k (the byte on drive k) becomes d'
            // Then it follows that
            // q   = (g^0 * d_0) + (g^1 * d_1) + ... + (g^n-1 * d_n-1)
            // q_k = (g^0 * d_0) + (g^1 * d_1) + ... + (g^k * d') + ... + (g^n-1 * d_n-1)
            // Then
            // q + q_k = (g^k * d_k) + (g^k * d')
            // Therefore
            // q_k = q + g^k * (d_k + d')
            // Which means XORing the old and new data, applying the generator g^k, then XORing the original Q parity byte will yield the new P parity byte
            q_parity.write(
                drive_offset,
                q_parity.read(drive_offset)? ^ (Gen::from_power(drive_index) * (old_data ^ data)),
            )?;
        }
        Ok(())
    }

    /// XORs the byte at `offset` across all data drives except the ones in `ignore`
    fn p_parity_offset_ignore(&self, offset: usize, ignore: Vec<usize>) -> Result<u8> {
        let data = self
            .data_drives()
            .enumerate()
            .filter(|(i, _)| !ignore.contains(i))
            .map(|(_, d)| d.read(offset))
            .collect::<drive::Result<Vec<u8>>>()?
            .into_iter()
            .reduce(|acc, x| acc ^ x)
            .unwrap();
        Ok(data)
    }

    fn q_parity_offset_ignore(&self, offset: usize, ignore: Vec<usize>) -> Result<u8> {
        let data = self
            .data_drives()
            .enumerate()
            .filter(|(i, _)| !ignore.contains(i))
            .map(|(i, d)| d.read(offset).map(|x| (i, x)))
            .collect::<drive::Result<Vec<(usize, u8)>>>()?
            .into_iter()
            .fold(0, |acc, (i, x)| acc ^ (Gen::from_power(i) * x));
        Ok(data)
    }

    /// Reads a byte at a specific offset in the array
    pub fn read(&self, offset: usize) -> Result<u8> {
        if offset >= self.size() {
            return Err(RaidError::OffsetTooLarge(offset).into());
        }
        if self.state() == RaidState::Failed {
            return Err(RaidError::Failed.into());
        }
        let drive_offset = offset % self.drive_size;
        let drive_index = offset / self.drive_size;
        let drive = self.data_drives().nth(drive_index).unwrap();
        if !drive.has_failed() {
            Ok(drive.read(drive_offset)?)
        } else {
            // At this point we are guaranteed at least one failed data drive because its the one we are trying to write to.
            // We are also guaranteed that the array isn't in a failed state because we check for it.
            // If we are RAID 5, we know there is exactly one failed data drive and we should use P parity to read.
            // If we are RAID 6, we know there is at most two failed drives, at least one being a data drive.
            //
            // In the case of one failed drive, it must be a data drive, therefore read using P parity in both RAID modes.
            // In the case of two failed drives, we have exactly three cases:
            // - Two data drives failed: Use P and Q parity to read
            // - One data drive and P parity failed: Use Q parity to read
            // - One data drive and Q parity failed: Use P parity to read

            // If one drive failed or two have failed and the other is Q parity
            if self.failed().count() == 1 || self.q_parity().has_failed() {
                let data = self.p_parity_offset_ignore(drive_offset, vec![drive_index])?
                    ^ self
                        .p_parity()
                        .read(drive_offset)
                        .context("failed to read parity")?;
                Ok(data)
            } else if self.p_parity().has_failed() {
                let data = self.q_parity_offset_ignore(drive_offset, vec![drive_index])?
                    ^ self
                        .q_parity()
                        .read(drive_offset)
                        .context("failed to read parity")?;
                let data = data / Gen::from_power(drive_index);
                Ok(data.value())
            } else {
                let x = drive_index as i16;
                let y = self
                    .data_drives()
                    .enumerate()
                    .filter(|(i, d)| *i != drive_index && d.has_failed())
                    .map(|(i, _)| i)
                    .next()
                    .expect("Expected a second distinct failed drive, found none")
                    as i16;
                let p_xy =
                    self.p_parity_offset_ignore(drive_offset, vec![x as usize, y as usize])?;
                let q_xy =
                    self.q_parity_offset_ignore(drive_offset, vec![x as usize, y as usize])?;
                let p = self.p_parity().read(drive_offset)?;
                let q = self.q_parity().read(drive_offset)?;
                let a = Gen::from_power(y - x) / (Gen::from_power(y - x) + 1);
                let b = Gen::from_power(-(x as i16)) / (Gen::from_power(y - x) + 1);

                Ok((a * (p ^ p_xy)) ^ (b * (q ^ q_xy)))
            }
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

    /// Returns an immutable reference to the drive used for Q parity
    pub fn q_parity(&self) -> &Drive {
        &self.drives[Q_INDEX]
    }
    /// Returns a mutable reference to the drive used for Q parity
    fn q_parity_mut(&mut self) -> &mut Drive {
        &mut self.drives[Q_INDEX]
    }

    /// Returns an iterator of tuples (I, D) where I is the absolute index in the drives array and D is an immutable reference to the corresponding data drive
    pub fn data_drives(&self) -> impl Iterator<Item = &Drive> {
        let start = match self.mode {
            RaidMode::Raid5 => 1,
            RaidMode::Raid6 => 2,
        };
        self.drives[start..].iter()
    }
    /// Returns an iterator of tuples (I, D) where I is the absolute index in the drives array and D is a mutable reference to the corresponding data drive
    fn data_drives_mut(&mut self) -> impl Iterator<Item = &mut Drive> {
        let start = match self.mode {
            RaidMode::Raid5 => 1,
            RaidMode::Raid6 => 2,
        };
        self.drives[start..].iter_mut()
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
    /// Returns an iterator of mutable references to drives that are unformatted
    pub fn unformatted_mut(&mut self) -> impl Iterator<Item = &mut Drive> {
        self.drives
            .iter_mut()
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
    }
    /// Chooses a random data drive that hasn't failed yet and marks it as failed
    pub fn fail_random_data(&mut self) {
        let drive = self
            .data_drives_mut()
            .filter_map(|d| d.has_failed().not().then_some(d))
            .choose(&mut rand::rng())
            .unwrap();
        drive.fail();
    }
    /// Mark the P parity drive as failed
    pub fn fail_p_parity(&mut self) {
        self.p_parity_mut().fail();
    }
    /// Mark the Q parity drive as failed
    pub fn fail_q_parity(&mut self) {
        self.q_parity_mut().fail();
    }
    /// Replaces failed drives with empty, functioning drives
    pub fn replace_failed_drives(&mut self) {
        for i in 0..self.drives.len() {
            if self.drives[i].has_failed() {
                let drive = Drive::empty(self.drive_size);
                self.drives[i] = drive;
            }
        }
    }

    fn repair_p_parity(&mut self) -> Result<()> {
        for i in 0..self.drive_size {
            let byte = self.p_parity_offset_ignore(i, vec![])?;
            self.p_parity_mut().write(i, byte)?;
        }
        self.p_parity_mut().format();
        Ok(())
    }
    fn repair_q_parity(&mut self) -> Result<()> {
        for i in 0..self.drive_size {
            let byte = self.q_parity_offset_ignore(i, vec![])?;
            self.q_parity_mut().write(i, byte)?;
        }
        self.q_parity_mut().format();
        Ok(())
    }
    fn repair_single_data_p_parity(&mut self, idx: usize) -> Result<()> {
        for i in 0..self.drive_size {
            let byte = self.p_parity_offset_ignore(i, vec![idx])? ^ self.p_parity().read(i)?;
            self.data_drives_mut().nth(idx).unwrap().write(i, byte)?;
        }
        self.data_drives_mut().nth(idx).unwrap().format();
        Ok(())
    }
    fn repair_single_data_q_parity(&mut self, idx: usize) -> Result<()> {
        for i in 0..self.drive_size {
            let byte = self.q_parity_offset_ignore(i, vec![idx])? ^ self.q_parity().read(i)?;
            let byte = byte / Gen::from_power(idx);
            self.data_drives_mut()
                .nth(idx)
                .unwrap()
                .write(i, byte.value())?;
        }
        self.data_drives_mut().nth(idx).unwrap().format();
        Ok(())
    }
    fn repair_double_data(&mut self, x: usize, y: usize) -> Result<()> {
        for i in 0..self.drive_size {
            let p_xy = self.p_parity_offset_ignore(i, vec![x as usize, y as usize])?;
            let q_xy = self.q_parity_offset_ignore(i, vec![x as usize, y as usize])?;
            let p = self.p_parity().read(i)?;
            let q = self.q_parity().read(i)?;
            let a = Gen::from_power(y - x) / (Gen::from_power(y - x) + 1);
            let b = Gen::from_power(-(x as i16)) / (Gen::from_power(y - x) + 1);

            let dx = (a * (p ^ p_xy)) ^ (b * (q ^ q_xy));
            let dy = p ^ p_xy ^ dx;
            self.data_drives_mut().nth(x).unwrap().write(i, dx)?;
            self.data_drives_mut().nth(y).unwrap().write(i, dy)?;
        }
        self.data_drives_mut().nth(x).unwrap().format();
        self.data_drives_mut().nth(y).unwrap().format();
        Ok(())
    }

    /// Repairs data for all unformatted drives with original data
    pub fn repair(&mut self) -> Result<()> {
        match self.state() {
            RaidState::Ok => Ok(()),
            RaidState::Failed => Err(RaidError::Failed.into()),
            RaidState::Uninit => Err(RaidError::NotInitialized.into()),
            RaidState::Degraded => {
                let p_unfmtd = !self.p_parity().is_formatted();
                let q_unfmtd = !self.q_parity().is_formatted();
                let num_unfmtd = self.unformatted().count();
                if num_unfmtd == 1 {
                    if p_unfmtd {
                        self.repair_p_parity()?;
                    } else if q_unfmtd {
                        self.repair_q_parity()?;
                    } else {
                        let idx = self
                            .data_drives()
                            .enumerate()
                            .filter_map(|(i, d)| d.is_formatted().not().then_some(i))
                            .next()
                            .unwrap();
                        self.repair_single_data_p_parity(idx)?;
                    }
                } else {
                    if p_unfmtd && q_unfmtd {
                        self.repair_p_parity()?;
                        self.repair_q_parity()?;
                    } else if q_unfmtd {
                        let idx = self
                            .data_drives()
                            .enumerate()
                            .filter_map(|(i, d)| d.is_formatted().not().then_some(i))
                            .next()
                            .unwrap();
                        self.repair_single_data_p_parity(idx)?;
                        self.repair_q_parity()?;
                    } else if p_unfmtd {
                        let idx = self
                            .data_drives()
                            .enumerate()
                            .filter_map(|(i, d)| d.is_formatted().not().then_some(i))
                            .next()
                            .unwrap();
                        self.repair_single_data_q_parity(idx)?;
                        self.repair_p_parity()?;
                    } else {
                        let mut iter = self
                            .data_drives()
                            .enumerate()
                            .filter_map(|(i, d)| d.is_formatted().not().then_some(i));
                        let x = iter.next().unwrap();
                        let y = iter.next().unwrap();
                        drop(iter);
                        self.repair_double_data(x, y)?;
                    }
                }
                Ok(())
            }
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
        sim.init().expect("Shit");
        let data = write_random(&mut sim);
        (sim, data)
    }

    fn write_random(sim: &mut RaidSim) -> Vec<u8> {
        let mut data = vec![0u8; sim.size()];
        rand::rng().fill(data.as_mut_slice());
        for i in 0..sim.size() {
            sim.write(i, data[i]).unwrap();
        }
        data
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
        let data = write_random(&mut sim);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid5_p_parity_failure() {
        let (mut sim, data) = init_random(RaidMode::Raid5);
        sim.fail_p_parity();
        assert_eq!(sim.state(), RaidState::Degraded);
        assert_sim_equal(&sim, &data);
        let data = write_random(&mut sim);
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
        assert_eq!(sim.unformatted().count(), 1);
        assert_eq!(sim.state(), RaidState::Degraded);
    }

    #[test]
    fn raid5_one_data_drive_repair() {
        let (mut sim, data) = init_random(RaidMode::Raid5);
        sim.fail_random_data();
        sim.replace_failed_drives();
        sim.repair().unwrap();
        assert_eq!(sim.state(), RaidState::Ok);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid5_p_parity_repair() {
        let (mut sim, data) = init_random(RaidMode::Raid5);
        sim.fail_p_parity();
        sim.replace_failed_drives();
        sim.repair().unwrap();
        assert_eq!(sim.state(), RaidState::Ok);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_test_init() {
        let (sim, data) = init_random(RaidMode::Raid6);
        assert_sim_equal(&sim, &data);
        assert_eq!(sim.state(), RaidState::Ok);
    }

    #[test]
    fn raid6_one_data_drive_failure() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_random_data();
        assert_eq!(sim.state(), RaidState::Degraded);
        assert_sim_equal(&sim, &data);
        let data = write_random(&mut sim);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_p_parity_failure() {
        let (mut sim, data) = init_random(RaidMode::Raid5);
        sim.fail_p_parity();
        assert_eq!(sim.state(), RaidState::Degraded);
        assert_sim_equal(&sim, &data);
        let data = write_random(&mut sim);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_q_parity_failure() {
        let (mut sim, data) = init_random(RaidMode::Raid5);
        sim.fail_q_parity();
        assert_eq!(sim.state(), RaidState::Degraded);
        assert_sim_equal(&sim, &data);
        let data = write_random(&mut sim);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_one_data_drive_and_p_parity_failure() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_random_data();
        sim.fail_p_parity();
        assert_eq!(sim.state(), RaidState::Degraded);
        assert_sim_equal(&sim, &data);
        let data = write_random(&mut sim);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_one_data_drive_and_q_parity_failure() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_random_data();
        sim.fail_q_parity();
        assert_eq!(sim.state(), RaidState::Degraded);
        assert_sim_equal(&sim, &data);
        let data = write_random(&mut sim);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_p_parity_and_q_parity_failure() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_p_parity();
        sim.fail_q_parity();
        assert_eq!(sim.state(), RaidState::Degraded);
        assert_sim_equal(&sim, &data);
        let data = write_random(&mut sim);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_two_data_drive_failure() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_random_data();
        sim.fail_random_data();
        assert_eq!(sim.state(), RaidState::Degraded);
        assert_eq!(sim.failed().count(), 2);
        assert_sim_equal(&sim, &data);
        let data = write_random(&mut sim);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_unformatted_still_degraded() {
        let (mut sim, _) = init_random(RaidMode::Raid6);
        assert_eq!(sim.state(), RaidState::Ok);
        sim.fail_random();
        sim.fail_random();
        assert_eq!(sim.state(), RaidState::Degraded);
        sim.replace_failed_drives();
        assert_eq!(sim.unformatted().count(), 2);
        assert_eq!(sim.state(), RaidState::Degraded);
    }

    #[test]
    fn raid6_one_data_drive_repair() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_random_data();
        sim.replace_failed_drives();
        sim.repair().unwrap();
        assert_eq!(sim.state(), RaidState::Ok);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_p_parity_repair() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_p_parity();
        sim.replace_failed_drives();
        sim.repair().unwrap();
        assert_eq!(sim.state(), RaidState::Ok);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_q_parity_repair() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_q_parity();
        sim.replace_failed_drives();
        sim.repair().unwrap();
        assert_eq!(sim.state(), RaidState::Ok);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_q_parity_and_one_data_drive_repair() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_q_parity();
        sim.fail_random_data();
        sim.replace_failed_drives();
        sim.repair().unwrap();
        assert_eq!(sim.state(), RaidState::Ok);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_p_parity_and_one_data_drive_repair() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_p_parity();
        sim.fail_random_data();
        sim.replace_failed_drives();
        sim.repair().unwrap();
        assert_eq!(sim.state(), RaidState::Ok);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_p_parity_and_q_parity_repair() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_p_parity();
        sim.fail_q_parity();
        sim.replace_failed_drives();
        sim.repair().unwrap();
        assert_eq!(sim.state(), RaidState::Ok);
        assert_sim_equal(&sim, &data);
    }

    #[test]
    fn raid6_two_data_drive_repair() {
        let (mut sim, data) = init_random(RaidMode::Raid6);
        sim.fail_random_data();
        sim.fail_random_data();
        sim.replace_failed_drives();
        sim.repair().unwrap();
        assert_eq!(sim.state(), RaidState::Ok);
        assert_sim_equal(&sim, &data);
    }
}
