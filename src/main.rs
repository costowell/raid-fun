// main.rs
mod drive;
mod generator;
mod raid;

use std::panic;

struct Test {
    name: &'static str,
    func: fn(),
}

fn main() {
    let tests = vec![
        Test {
            name: "Generator 1d",
            func: generator::tests::test_1d,
        },
        Test {
            name: "Generator zero",
            func: generator::tests::test_zero,
        },
        Test {
            name: "Generator one",
            func: generator::tests::test_one,
        },
        Test {
            name: "Generator inverse",
            func: generator::tests::test_inverses,
        },
        Test {
            name: "RAID5 data drive corrupt",
            func: raid::tests::raid5_normal_corrupt,
        },
        Test {
            name: "RAID6 data and P drive corrupt",
            func: raid::tests::raid6_normal_and_p_drive_corrupt,
        },
        Test {
            name: "RAID6 two data drives corrupt",
            func: raid::tests::raid6_two_normal_corrupt,
        },
    ];

    let mut passed = 0;
    let mut failed = 0;

    for test in &tests {
        print!("Running {}... ", test.name);
        let result = panic::catch_unwind(panic::AssertUnwindSafe(test.func));

        match result {
            Ok(_) => {
                println!("PASSED");
                passed += 1;
            }
            Err(e) => {
                println!("FAILED");
                if let Some(s) = e.downcast_ref::<String>() {
                    println!("  Error: {}", s);
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    println!("  Error: {}", s);
                }
                failed += 1;
            }
        }
    }

    println!("\n{} passed, {} failed", passed, failed);

    if failed > 0 {
        std::process::exit(1);
    }
}
