pub mod store;
pub mod bag_store;
pub mod nan_free_f32;
pub mod err_log;
pub mod fly_weight;

pub struct Sliding<'a, T> where T: Clone {
  z: &'a mut dyn Iterator<Item = T>,
  prev: Option<T>,
}

impl<'a, T> Iterator for Sliding<'a, T> where T: Clone {
    type Item = (T, T);

    fn next(&mut self) -> Option<Self::Item> {
      if let Some(t) = self.z.next() {
        if let Some(prev) = self.prev.take() {
          let ret = Some((prev, t.clone()));
          self.prev = Some(t);
          ret
        } else {
          self.prev = Some(t);
          self.next()
        }

      } else {
        None
      }
    }
}

pub fn sliding<'a, T>(z: &'a mut dyn Iterator<Item = T>) -> impl Iterator<Item = (T, T)> + 'a where T: Clone {
  Sliding::<'a, T> { z, prev: None }
}

pub fn merge_option<T, F>(opt0: Option<T>, opt1: Option<T>, f: F) -> Option<T>
    where F: FnOnce(T, T) -> T
{
    if let Some(a) = opt0 {
        if let Some(b) = opt1 {
            Some(f(a, b))
        } else {
            Some(a)
        }
    } else {
        opt1
    }
}

#[cfg(test)]
mod tests {
    use crate::{merge_option, sliding};

  #[test]
  fn empty() {
    let tbl: Vec<i32> = vec![];
    let mut z = tbl.iter();
    let mut s = sliding(&mut z);
    assert_eq!(s.next(), None);
  }

  #[test]
  fn single() {
    let tbl: Vec<i32> = vec![1];
    let mut z = tbl.iter();
    let mut s = sliding(&mut z);
    assert_eq!(s.next(), None);
  }

  #[test]
  fn two() {
    let tbl: Vec<i32> = vec![1, 5];
    let mut z = tbl.iter();
    let mut s = sliding(&mut z);
    assert_eq!(s.next(), Some((&1, &5)));
    assert_eq!(s.next(), None);
  }

  #[test]
  fn many() {
    let tbl = ["One", "Two", "Three"];
    let mut z = tbl.iter();
    let mut s = sliding(&mut z);
    assert_eq!(s.next(), Some((&"One", &"Two")));
    assert_eq!(s.next(), Some((&"Two", &"Three")));
    assert_eq!(s.next(), None);
  }

  #[test]
  fn can_merge_option() {
      assert_eq!(merge_option(None, None, |_, _| 0), None);
      assert_eq!(merge_option(Some(1), None, |_, _| 2), Some(1));
      assert_eq!(merge_option(None, Some(1), |_, _| 2), Some(1));
      assert_eq!(merge_option(Some(1), Some(2), |v0, v1| v0 + v1), Some(3));
  }
}