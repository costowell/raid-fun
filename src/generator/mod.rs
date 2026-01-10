mod table;

use crate::generator::table::MTable;
use std::ops::{Add, BitXor, BitXorAssign, Div, Mul};

use static_init::dynamic;

#[dynamic]
static TABLE: MTable = MTable::new();

/// Magic value representing 0 in the field.
/// Because there exists no n where g^n = 0 and since g^255 is redundant of g^0, we have a special case representing 0.
/// This exhausts all possible values a u8 can hold.
const ZERO: u8 = 255;

pub trait FromPower<T> {
    fn from_power(n: T) -> Self;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Gen {
    n: u8,
}

impl From<u8> for Gen {
    fn from(value: u8) -> Self {
        if value == 0 {
            Self::zero()
        } else {
            Self {
                n: TABLE.gn_to_n[value as usize],
            }
        }
    }
}

impl Mul for Gen {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        if rhs.n == ZERO || self.n == ZERO {
            Self::Output::zero()
        } else {
            Self::Output {
                n: (((self.n as u16) + (rhs.n as u16)) % 255) as u8,
            }
        }
    }
}

impl Mul<u8> for Gen {
    type Output = Self;

    fn mul(self, rhs: u8) -> Self::Output {
        self * Gen::from(rhs)
    }
}
impl Mul<Gen> for u8 {
    type Output = Gen;

    fn mul(self, rhs: Gen) -> Self::Output {
        Gen::from(self) * rhs
    }
}

impl Div<Gen> for Gen {
    type Output = Gen;

    fn div(self, rhs: Gen) -> Self::Output {
        self * rhs.inverse()
    }
}
impl Div<Gen> for u8 {
    type Output = Gen;

    fn div(self, rhs: Gen) -> Self::Output {
        Gen::from(self) / rhs
    }
}

impl Add<u8> for Gen {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        Self::Output {
            n: TABLE.gn_to_n[(TABLE.n_to_gn[self.n as usize] ^ rhs) as usize],
        }
    }
}
impl Add<Gen> for u8 {
    type Output = Gen;

    fn add(self, rhs: Gen) -> Self::Output {
        rhs + self
    }
}

impl FromPower<u8> for Gen {
    fn from_power(n: u8) -> Self {
        Self { n }
    }
}

impl FromPower<i16> for Gen {
    fn from_power(n: i16) -> Self {
        Self {
            n: n.rem_euclid(255) as u8,
        }
    }
}
impl FromPower<i32> for Gen {
    fn from_power(n: i32) -> Self {
        Self {
            n: n.rem_euclid(255) as u8,
        }
    }
}
impl FromPower<usize> for Gen {
    fn from_power(n: usize) -> Self {
        Self {
            n: n.rem_euclid(255) as u8,
        }
    }
}

impl BitXor<u8> for Gen {
    type Output = u8;

    fn bitxor(self, rhs: u8) -> Self::Output {
        self.value() ^ rhs
    }
}

impl BitXor<Gen> for u8 {
    type Output = u8;

    fn bitxor(self, rhs: Gen) -> Self::Output {
        self ^ rhs.value()
    }
}

impl BitXorAssign<Gen> for u8 {
    fn bitxor_assign(&mut self, rhs: Gen) {
        if rhs.n == ZERO {
            return;
        }
        *self ^= rhs.value();
    }
}

impl Gen {
    /// Gets g^-n from g^n
    pub fn inverse(self) -> Self {
        if self.n == ZERO {
            panic!("Division by zero not allowed.")
        }
        // g^0 is it's own inverse
        if self.n == 0 {
            return self;
        }
        Self { n: 255 - self.n }
    }
    /// Returns g^n
    pub fn value(self) -> u8 {
        if self.n == ZERO {
            0
        } else {
            TABLE.n_to_gn[self.n as usize]
        }
    }
    /// Returns the power of the generator
    pub fn power(self) -> u8 {
        self.n
    }
    /// Returns a value representing zero
    pub fn zero() -> Self {
        Self { n: ZERO }
    }
}

pub mod tests {
    use super::*;

    pub fn test_zero() {
        // 0 * g^i == 0
        for i in 0..255 {
            assert_eq!(Gen::zero() * Gen::from_power(i), Gen::zero());
        }
    }

    pub fn test_one() {
        // 1 == g^0
        assert_eq!(Gen::from(1).power(), 0);
        // g^1 == g == 2
        assert_eq!(Gen::from(2).power(), 1);
        // g^255 == g^0
        assert_eq!(Gen::from_power(255), Gen::from_power(0));
    }

    pub fn test_inverses() {
        // Not including 255 because g^255 = g^0 = 1
        for i in 0..255 {
            assert_eq!(Gen::from_power(i) * Gen::from_power(-i), Gen::from_power(0));
        }
    }

    pub fn test_1d() {
        // Source: Section 1, https://www.kernel.org/pub/linux/kernel/people/hpa/raid6.pdf
        // "Note, however: {02}^8 = {1d}"
        assert_eq!(Gen::from_power(8).value(), 0x1d);
    }
}
