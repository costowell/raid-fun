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
    n_to_gn: [u8; 255],
    gn_to_n: [u8; 256],
}

impl MTable {
    /// Generates the table for the Galois Field used by raid6, GF(2^8)
    ///
    /// Documentation: [The mathematics of RAID-6](https://www.kernel.org/pub/linux/kernel/people/hpa/raid6.pdf)
    pub fn new() -> Self {
        let mut n_to_gn = [0u8; 255];
        let mut gn_to_n = [0u8; 256];
        n_to_gn[0] = 1; // g^0 = e = 1
        for n in 1..255 {
            n_to_gn[n] = raid6_generator(n_to_gn[n - 1]); // g^n = g * g^(n-1)
        }
        for (i, e) in n_to_gn.iter().enumerate() {
            gn_to_n[*e as usize] = i as u8
        }
        MTable {
            n_to_gn,
            gn_to_n,
        }
    }

    /// Applies the generator {02} to an input number
    pub fn apply(&self, input: u8) -> u8 {
        if input == 0 {
            return 0;
        }
        let n = self.gn_to_n[input as usize] as usize;
        let next_n = (n + 1).rem_euclid(255);
        self.n_to_gn[next_n]
    }

    /// Applies the generator {02} to an input number n times
    pub fn applyn(&self, input: u8, k: i16) -> u8 {
        if input == 0 {
            return 0;
        }
        let n = self.gn_to_n[input as usize] as i16;
        let next_n = (n + k).rem_euclid(255);
        self.n_to_gn[next_n as usize]
    }

    /// Returns a valid power n of the generator where g^n = input
    pub fn generator_power(&self, input: u8) -> i16 {
        if input == 0 {
            panic!("No power of the generator can yield 0: undefined")
        }
        self.gn_to_n[input as usize] as i16
    }
}

impl Display for MTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for num in self.n_to_gn {
            write!(f, "{num:02X} ")?
        }
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

    pub fn test_one() {
        let table = MTable::new();
        // g^0 = e = 1
        let should_be_zero = table.generator_power(1);
        assert_eq!(should_be_zero, 0);
        // g^1 = g
        let should_be_one = table.generator_power(2);
        assert_eq!(should_be_one, 1);
        // g^255 = e = 1
        let should_be_one = table.applyn(1, 255);
        assert_eq!(should_be_one, 1);
    }

    pub fn test_inverse() {
        let table = MTable::new();
        // Not including 255 because g^255 = g^0 = 1
        for i in 0..255 {
            let g = table.applyn(1, i);
            let n = table.generator_power(g);
            assert_eq!(n, i);
        }
    }

    pub fn test_inverse_elements() {
        let table = MTable::new();
        // Not including 255 because g^255 = g^0 = 1
        for i in 0..255 {
            let g = table.applyn(1, i);
            let e = table.applyn(g, -i);
            assert_eq!(e, 1);
        }
    }

    pub fn test_1d() {
        let table = MTable::new();
        let should_be_1d = table.applyn(1, 8);
        // Source: Section 1, https://www.kernel.org/pub/linux/kernel/people/hpa/raid6.pdf
        // "Note, however: {02}^8 = {1d}"
        assert_eq!(should_be_1d, 0x1d);
    }
}
