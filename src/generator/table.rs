use std::{fmt::Display, ops::BitXor};

/// Gets the nth bit from a u8
pub fn nth_bit(num: u8, idx: u8) -> u8 {
    (num >> idx) & 1
}

/// Applies the {02} generator from GF(2^8), what is effectively the element x, modded by x^8 + x^4 + x^3 + x^2 + 1
fn raid6_generator(num: u8) -> u8 {
    let m = if nth_bit(num, 7) == 1 {
        0x1d // Effectively x^4 + x^3 + x^2 + 1
    } else {
        0
    };
    (num << 1) ^ m
}

/// Represents a table of repeated multiplication by the generator {02} in GF(2^8) modded by x^8 + x^4 + x^3 + x^2 + 1
///
/// Since {02} is a generator of GF(2^8), it is cyclic.
/// This means we can find all g^n where 1 <= n <= 255 and create tables which map g^n -> n and n -> g^n.
/// When given a number x, we use the first table to find an n where x = g^n.
/// With a little algebra we know that x*g^k = g^(n+k) which tells us what x is after applying the generator k times.
/// By using the second table, we can find g^(n+k) by indexing at (n + k) mod 255.
pub struct MTable {
    pub n_to_gn: [u8; 255],
    pub gn_to_n: [u8; 256],
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
        MTable { n_to_gn, gn_to_n }
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
