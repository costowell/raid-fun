mod drive;
mod raid5;
mod table;

use static_init::dynamic;

use crate::table::MTable;

#[dynamic]
static MTABLE: MTable = MTable::new();

fn main() {
    println!("Run 'cargo test -- --nocapture' instead")
}
