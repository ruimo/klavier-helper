use std::collections::HashMap;
use std::hash::Hash;

#[derive(Default)]
pub struct FlyWeight<'a, T> {
    pool: HashMap<&'a T, &'a T>,
}

impl<'a, T> FlyWeight<'a, T>
    where T: Eq + Hash
{
    pub fn new() -> FlyWeight<'a, T> {
        FlyWeight {
            pool: HashMap::new(),
        }
    }

    pub fn intern(&mut self, instance: &'a T) -> &'a T {
        self.pool.entry(instance).or_insert(instance)
    }
}

#[cfg(test)]
mod tests {
    use crate::fly_weight::FlyWeight;

    #[derive(Hash, PartialEq, Eq)]
    struct Data {
        value: i32,
    }

    impl Data {
        fn new(value: i32) -> Data {
            Data {
                value
            }
        }
    }

    #[test]
    fn assume_interned() {
        let d0 = Data::new(0);
        let d1 = Data::new(0);
        let mut pool = FlyWeight::new();

        let d0_interned = pool.intern(&d0);
        assert!(std::ptr::eq(&d0, d0_interned));

        let d1_interned = pool.intern(&d1);
        assert!(std::ptr::eq(&d0, d1_interned));

        let d2 = Data::new(1);
        let d2_interned = pool.intern(&d2);
        assert!(std::ptr::eq(&d2, d2_interned));
    }
}

