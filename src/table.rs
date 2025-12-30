use std::{fmt::Display, ops::BitXor};

/// Gets the nth bit from a u8
pub fn nth_bit(num: u8, idx: u8) -> u8 {
    (num >> idx) & 1
}

/// Applies the {02} generator from GF(2^8) modded by x^8 + x^4 + x^3 + x^2 + 1
fn raid6_generator(num: u8) -> u8 {
    let seventh = nth_bit(num, 7);
    (nth_bit(num, 6) << 7)
        | (nth_bit(num, 5) << 6)
        | (nth_bit(num, 4) << 5)
        | (nth_bit(num, 3).bitxor(seventh) << 4)
        | (nth_bit(num, 2).bitxor(seventh) << 3)
        | (nth_bit(num, 1).bitxor(seventh) << 2)
        | (nth_bit(num, 0) << 1)
        | (nth_bit(num, 7))
}

/// Represents the number of generators in the table
/// In this case, they are the powers of 2 up to and including 2^7
const MTABLE_NUM_GENS: usize = 256;
/// Represents the number of elements in the table
/// In this case, they are the numbers from 0 to 255
const MTABLE_NUM_ELMS: usize = 256;

/// Represents a table of repeated right multiplication by the generator {02} in GF(2^8)
///
/// # Examples
///
/// ```
/// let ktable = MTable::new_raid6();
/// ```
pub struct MTable {
    table: [[u8; MTABLE_NUM_GENS]; MTABLE_NUM_ELMS],
}

impl MTable {
    /// Generates the table for the Galois Field used by raid6, GF(2^8)
    ///
    /// Documentation: [The mathematics of RAID-6](https://www.kernel.org/pub/linux/kernel/people/hpa/raid6.pdf)
    pub fn new() -> Self {
        let mut table = [[0u8; MTABLE_NUM_GENS]; MTABLE_NUM_ELMS];
        for elm in 0..MTABLE_NUM_ELMS {
            for gen in 0..MTABLE_NUM_GENS {
                if gen == 0 {
                    // Safe cast since elm is 0..=255
                    table[elm][gen] = elm as u8;
                } else {
                    table[elm][gen] = raid6_generator(table[elm][gen - 1]);
                }
            }
        }
        MTable { table }
    }

    /// Applies the generator {02} to an input number
    pub fn apply(&self, input: u8) -> u8 {
        self.table[input as usize][1]
    }

    /// Applies the generator {02} to an input number n times
    pub fn applyn(&self, input: u8, n: usize) -> u8 {
        self.table[input as usize][n]
    }
}

impl Display for MTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in self.table {
            for elm in row {
                write!(f, "{elm:02X} ")?
            }
            write!(f, "\n")?
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1d() {
        let table = MTable::new();
        let should_be_1d = table.applyn(1, 8);
        // Source: Section 1, https://www.kernel.org/pub/linux/kernel/people/hpa/raid6.pdf
        // "Note, however: {02}^8 = {1d}"
        assert_eq!(should_be_1d, 29);
    }
}
