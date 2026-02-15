use divan::Bencher;
use raid::RaidSim;
use rand::Rng;

fn main() {
    divan::main();
}

fn rand_vec(size: usize) -> Vec<u8> {
    let mut data = vec![0u8; size];
    rand::rng().fill(data.as_mut_slice());
    data
}

#[divan::bench(args = [16, 32, 64, 128, 257])]
fn raid6_single_write_num_drives_scale(bencher: Bencher, num_drives: usize) {
    let mut sim = RaidSim::new(raid::RaidMode::Raid6, num_drives, 1024 * 16);
    sim.init().unwrap();
    let payload = rand_vec(sim.size());
    bencher.bench_local(move || {
        for (i, b) in payload.iter().enumerate() {
            sim.write(i, *b).unwrap();
        }
    });
}

#[divan::bench(args = [16, 32, 64, 128, 257])]
fn raid6_slice_write_num_drives_scale(bencher: Bencher, num_drives: usize) {
    let mut sim = RaidSim::new(raid::RaidMode::Raid6, num_drives, 1024 * 16);
    sim.init().unwrap();
    let payload = rand_vec(sim.size());
    bencher.bench_local(move || {
        sim.write_slice(0, payload.as_slice()).unwrap();
    });
}

#[divan::bench(args = [1024, 1024*16, 1024*16*16, 1024*16*16*16])]
fn raid6_single_write_drive_size_scale(bencher: Bencher, drive_size: usize) {
    let mut sim = RaidSim::new(raid::RaidMode::Raid6, 16, drive_size);
    sim.init().unwrap();
    let payload = rand_vec(sim.size());
    bencher.bench_local(move || {
        for (i, b) in payload.iter().enumerate() {
            sim.write(i, *b).unwrap();
        }
    });
}

#[divan::bench(args = [1024, 1024*16, 1024*16*16, 1024*16*16*16])]
fn raid6_slice_write_drive_size_scale(bencher: Bencher, drive_size: usize) {
    let mut sim = RaidSim::new(raid::RaidMode::Raid6, 16, drive_size);
    sim.init().unwrap();
    let payload = rand_vec(sim.size());
    bencher.bench_local(move || {
        sim.write_slice(0, payload.as_slice()).unwrap();
    });
}
