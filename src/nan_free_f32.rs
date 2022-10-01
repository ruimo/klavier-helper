use std::{cmp::Ordering, ops::Sub};

#[derive(Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct NanFreeF32(f32);

pub const ZERO: NanFreeF32 = NanFreeF32(0.0);
pub const MAX: NanFreeF32 = NanFreeF32(f32::MAX);

impl Eq for NanFreeF32 {
}

impl NanFreeF32 {
    pub fn to_f32(&self) -> f32 {
        self.0
    }
}

pub fn max(f0: NanFreeF32, f1: NanFreeF32) -> NanFreeF32 {
    if f0 < f1 { f1 }
    else { f0 }
}

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

impl From<f32> for NanFreeF32 {
    fn from(value: f32) -> Self {
        if value.is_nan() {
            panic!("Nan is not allowed.");
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
}