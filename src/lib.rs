pub mod store;
pub mod bag_store;
pub mod nan_free_f32;
pub mod err_log;

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

#[cfg(test)]
mod tests {
    use crate::sliding;

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
    let tbl = vec!["One", "Two", "Three"];
    let mut z = tbl.iter();
    let mut s = sliding(&mut z);
    assert_eq!(s.next(), Some((&"One", &"Two")));
    assert_eq!(s.next(), Some((&"Two", &"Three")));
    assert_eq!(s.next(), None);
  }
}