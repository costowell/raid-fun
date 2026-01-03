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

/// Represents a table of repeated multiplication by the generator {02} in GF(2^8) modded by x^8 + x^4 + x^3 + x^2 + 1
///
/// # Examples
///
/// ```
/// let table = MTable::new();
/// let num = table.applyn(1, 8); // 1 * {02}^8
/// ```
///
/// Since {02} is a generator of GF(2^8), it is cyclic.
/// This means we can find all g^n where 1 <= n <= 255 and create tables which map g^n -> n and n -> g^n.
/// When given a number x, we use the first table to find an n where x = g^n.
/// With a little algebra we know that x*g^k = g^(n+k) which tells us what x is after applying the generator k times.
/// By using the second table, we can find g^(n+k) by indexing at (n + k) mod 255.
pub struct MTable {
    gen_to_num: [u8; 255],
    num_to_gen: [u8; 256],
}

impl MTable {
    /// Generates the table for the Galois Field used by raid6, GF(2^8)
    ///
    /// Documentation: [The mathematics of RAID-6](https://www.kernel.org/pub/linux/kernel/people/hpa/raid6.pdf)
    pub fn new() -> Self {
        let mut gen_to_num = [0u8; 255];
        let mut num_to_gen = [0u8; 256];
        gen_to_num[0] = 1; // g^0 = e = 1
        for elm in 1..255 {
            gen_to_num[elm] = raid6_generator(gen_to_num[elm - 1]);
        }
        for (i, e) in gen_to_num.iter().enumerate() {
            num_to_gen[*e as usize] = i as u8
        }
        MTable {
            gen_to_num,
            num_to_gen,
        }
    }

    /// Applies the generator {02} to an input number
    pub fn apply(&self, input: u8) -> u8 {
        if input == 0 {
            return 0;
        }
        let gen_num = self.num_to_gen[input as usize];
        let next_gen_num = (gen_num + 1) as usize % 255;
        self.gen_to_num[next_gen_num as usize]
    }

    /// Applies the generator {02} to an input number n times
    pub fn applyn(&self, input: u8, n: usize) -> u8 {
        if input == 0 {
            return 0;
        }
        let gen_num = self.num_to_gen[input as usize];
        let next_gen_num = (gen_num as usize + n) as usize % 255;
        self.gen_to_num[next_gen_num as usize]
    }
}

impl Display for MTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for num in self.gen_to_num {
            write!(f, "{num:02X} ")?
        }
        write!(f, "\n")?;
        Ok(())
    }
}

pub mod tests {
    use super::*;

    pub fn test_zero() {
        let table = MTable::new();
        let should_be_zero = table.apply(0);
        assert_eq!(should_be_zero, 0);
        let should_be_zero = table.applyn(0, 10);
        assert_eq!(should_be_zero, 0);
    }

    pub fn test_1d() {
        let table = MTable::new();
        let should_be_1d = table.applyn(1, 8);
        // Source: Section 1, https://www.kernel.org/pub/linux/kernel/people/hpa/raid6.pdf
        // "Note, however: {02}^8 = {1d}"
        assert_eq!(should_be_1d, 0x1d);
    }
}
