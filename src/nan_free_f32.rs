use std::{cmp::Ordering, fmt, ops::{Sub, Add, Mul, Div, AddAssign, SubAssign, MulAssign, DivAssign}};

/// A wrapper around f32 that guarantees the value is not NaN
///
/// This type implements Eq and Ord traits, making it suitable for use as a key
/// in collections like BTreeMap and BTreeSet.
#[derive(Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct NanFreeF32(f32);

/// Constant representing zero as a NanFreeF32
pub const ZERO: NanFreeF32 = NanFreeF32(0.0);
/// Constant representing the maximum value of f32 as a NanFreeF32
pub const MAX: NanFreeF32 = NanFreeF32(f32::MAX);

/// NanFreeF32 implements Eq because it guarantees no NaN values
impl Eq for NanFreeF32 {
}

impl fmt::Display for NanFreeF32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl NanFreeF32 {
    /// Converts the NanFreeF32 back to a regular f32
    pub fn to_f32(&self) -> f32 {
        self.0
    }
}

/// Returns the maximum of two NanFreeF32 values
pub fn max(f0: NanFreeF32, f1: NanFreeF32) -> NanFreeF32 {
    if f0 < f1 { f1 }
    else { f0 }
}

#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for NanFreeF32 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.0 < other.0 {
            Ordering::Less
        } else if self.0 > other.0 {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

/// Converts an f32 to a NanFreeF32
///
/// # Panics
///
/// This function will panic if the input value is NaN.
/// Consider checking with `f32::is_nan()` before conversion if the input
/// might contain NaN values.
impl From<f32> for NanFreeF32 {
    fn from(value: f32) -> Self {
        if value.is_nan() {
            panic!("NaN value detected in NanFreeF32::from(). NaN values are not allowed in NanFreeF32.");
        } else {
            Self(value)
        }
    }
}

impl Sub for NanFreeF32 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::from(self.0 - rhs.0)
    }
}

impl Add for NanFreeF32 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::from(self.0 + rhs.0)
    }
}

impl Mul for NanFreeF32 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::from(self.0 * rhs.0)
    }
}

impl Div for NanFreeF32 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::from(self.0 / rhs.0)
    }
}

impl AddAssign for NanFreeF32 {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self::from(self.0 + rhs.0);
    }
}

impl SubAssign for NanFreeF32 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = Self::from(self.0 - rhs.0);
    }
}

impl MulAssign for NanFreeF32 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = Self::from(self.0 * rhs.0);
    }
}

impl DivAssign for NanFreeF32 {
    fn div_assign(&mut self, rhs: Self) {
        *self = Self::from(self.0 / rhs.0);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use super::NanFreeF32;

    #[test]
    fn can_be_used_as_key() {
        let mut set = BTreeSet::<NanFreeF32>::new();
        set.insert(1.0.into());
        set.insert(3.0.into());
        set.insert(2.0.into());

        let mut z = set.iter();
        assert_eq!(*z.next().unwrap(), 1.0.into());
        assert_eq!(*z.next().unwrap(), 2.0.into());
        assert_eq!(*z.next().unwrap(), 3.0.into());
        assert_eq!(z.next(), None);
    }

    #[test]
    #[should_panic]
    fn nan_should_be_rejected() {
        let _ = NanFreeF32::from(f32::NAN);
    }

    #[test]
    fn sub() {
        assert_eq!(NanFreeF32::from(1.1) - NanFreeF32::from(0.1), NanFreeF32::from(1.1 - 0.1));
    }
    
    #[test]
    fn add() {
        assert_eq!(NanFreeF32::from(1.1) + NanFreeF32::from(0.1), NanFreeF32::from(1.1 + 0.1));
    }
    
    #[test]
    fn mul() {
        assert_eq!(NanFreeF32::from(2.0) * NanFreeF32::from(3.0), NanFreeF32::from(2.0 * 3.0));
    }
    
    #[test]
    fn div() {
        assert_eq!(NanFreeF32::from(6.0) / NanFreeF32::from(2.0), NanFreeF32::from(6.0 / 2.0));
    }
    
    #[test]
    fn assign_ops() {
        let mut a = NanFreeF32::from(1.0);
        a += NanFreeF32::from(2.0);
        assert_eq!(a, NanFreeF32::from(3.0));
        
        a -= NanFreeF32::from(1.0);
        assert_eq!(a, NanFreeF32::from(2.0));
        
        a *= NanFreeF32::from(3.0);
        assert_eq!(a, NanFreeF32::from(6.0));
        
        a /= NanFreeF32::from(2.0);
        assert_eq!(a, NanFreeF32::from(3.0));
    }
    
    #[test]
    fn display() {
        assert_eq!(format!("{}", NanFreeF32::from(42.5)), "42.5");
    }
}