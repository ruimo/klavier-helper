use std::{ops::{Bound, Deref, Index, RangeBounds}, slice::Iter};

#[derive(Clone, Debug)]
pub enum StoreEvent<K, T, M> {
    Added { added: T, metadata: M },
    Removed(T),
    ClearedAll,
    BulkAddedRemoved { added: Vec<(K, T)>, removed: Vec<(K, T)>, metadata: M },
    Changed { from_to: Vec<((K, T), (K, T))>, removed: Vec<(K, T)>, metadata: M },
}

#[derive(Clone)]
pub struct Store<K: Ord + Copy, T: Clone, M> {
    store: Vec<(K, T)>,
    events: Option<Vec<StoreEvent<K, T, M>>>,
}

impl<K: Ord + Copy, T: Clone, M> AsRef<Vec<(K, T)>> for Store<K, T, M> {
    fn as_ref(&self) -> &Vec<(K, T)> {
        self.store.as_ref()
    }
}

impl<K: Ord + Copy, T: Clone, M> Deref for Store<K, T, M> {
    type Target = [(K, T)];
    
    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl<K, T, M> Store<K, T, M> where K: Ord + Copy, T: Clone {
    pub fn new(hold_events: bool) -> Self {
        Self { 
            store: vec![],
            events: if hold_events { Some(vec![]) } else { None },
        }
    }

    pub fn with_capacity(capacity: usize, hold_events: bool) -> Self {
        Self { 
            store: Vec::with_capacity(capacity),
            events: if hold_events { Some(vec![]) } else { None },
        }
    }

    pub fn index(&self, key: K) -> Result<usize, usize> {
        self.store.binary_search_by_key(&key, |&(k, _)| k)
    }

    #[inline]
    pub fn peek_last(&self) -> Option<&(K, T)> {
        self.store.iter().last()
    }

    fn fire_event<F>(&mut self, f: F) where F: FnOnce() -> StoreEvent<K, T, M> {
        if let Some(events) = self.events.as_mut() {
            events.push(f());
        }
    }

    pub fn add(&mut self, key: K, value: T, metadata: M) -> Option<T> {
        let mut removed: Option<T> = None;
        if let Some(r) = self.add_internal(key, value.clone()) {
            removed = Some(r.clone());
            self.fire_event(|| StoreEvent::Removed(r));
        }
        self.fire_event(|| StoreEvent::Added { added: value, metadata });
        removed
    }

    fn add_internal(&mut self, key: K, value: T) -> Option<T> {
        match self.store.binary_search_by_key(&key, |&(k, _)| k) {
            Ok(i) => {
                let old = self.store[i].clone();
                self.store[i] = (key, value);
                Some(old.1)
            },
            Err(i) => {
                self.store.insert(i, (key, value));
                None
            },
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<(K, T)> {
        let ret = self.remove_internal(key);
        if let Some(removed) = ret.as_ref() {
            self.fire_event(|| StoreEvent::Removed(removed.1.clone()));
        }
        ret
    }

    fn remove_internal(&mut self, key: &K) -> Option<(K, T)> {
        match self.store.binary_search_by_key(key, |&(k, _)| k) {
            Ok(i) => {
                let e = self.store.remove(i);
                Some(e)
            },
            Err(_) => {
                None
            },
        }
    }
    
    pub fn range<R>(&self, bounds: R) -> (usize, &[(K, T)]) where R: RangeBounds<K> {
        if self.store.is_empty() {
            return (0, &self.store[0..0])
        }

        let start_bound: usize =
            match bounds.start_bound() {
                Bound::Unbounded => 0,
                Bound::Excluded(i) => {
                    match self.find(i) {
                        Ok(i) => i + 1,
                        Err(i) => i,
                    }
                },
                Bound::Included(i) => {
                    match self.find(i) {
                        Ok(i) => i,
                        Err(i) => i,
                    }
                },
            };

        let end_bound: usize =
            match bounds.end_bound() {
                Bound::Unbounded => self.store.len(),
                Bound::Excluded(i) => {
                    match self.find(i) {
                        Ok(i) => i,
                        Err(i) => i,
                    }
                },
                Bound::Included(i) => {
                    match self.find(i) {
                        Ok(i) => i + 1,
                        Err(i) => i,
                    }
                },
            };

        if start_bound == end_bound {
            (0, &self.store[0..0])
        } else {
            (start_bound, &self.store[start_bound..end_bound])
        }
    }

    pub fn change(&mut self, from_to: &[(&K, (K, T))], metadata: M) -> Vec<(K, T)> where T: Clone {
        let mut result: Vec<((K, T), (K, T))> = Vec::with_capacity(from_to.len());

        // Remove all 'from's in advance because adding 'to' will replace(remove) the existing 'from'.
        for (k, to) in from_to.iter() {
            if let Some(removed) = self.remove_internal(*k) {
                result.push((removed, to.clone()));
            }
        }

        // Adding 'to's may replace the existing.
        let mut removed = vec![];
        for (_, (k, v)) in result.iter() {
            if let Some(r) = self.add_internal(*k, v.clone()) {
                removed.push((*k, r));
            }
        }
        self.fire_event(|| StoreEvent::Changed { from_to: result, removed: removed.clone(), metadata });
        removed
    }

    pub fn bulk_add(&mut self, recs: Vec<(K, T)>, metadata: M) -> Vec<(K, T)> where K: Clone, T: Clone {
        let mut removed = vec![];

        for (k, v) in recs.iter() {
            if let Some(r) = self.add_internal(*k, v.clone()) {
                removed.push((*k, r));
            }
        }
        self.fire_event(|| StoreEvent::BulkAddedRemoved { added: recs, removed: removed.clone(), metadata });

        removed
    }

    pub fn bulk_remove(&mut self, recs: &[K], metadata: M) -> Vec<(K, T)> {
        let mut removed: Vec<(K, T)> = Vec::with_capacity(recs.len());

        for k in recs.iter() {
            if let Some(r) = self.remove_internal(k) {
                removed.push(r);
            }
        }
        self.fire_event(|| StoreEvent::BulkAddedRemoved { added: vec![], removed: removed.clone(), metadata });

        removed
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, (K, T)> {
        self.store.iter()
    }

    pub fn pop_first(&mut self) -> Option<(K, T)> where K: Clone {
        if self.store.is_empty() {
            return None;
        }

        let (k, v) = self.store.remove(0);
        self.fire_event(|| StoreEvent::Removed(v.clone()));

        Some((k, v))
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn clear(&mut self) {
        self.store.clear();
        self.fire_event(|| StoreEvent::ClearedAll);
    }

    pub fn clear_events(&mut self) {
        if let Some(events) = self.events.as_mut() {
            events.clear();
        }
    }

    #[inline]
    pub fn find(&self, key: &K) -> Result<usize, usize> {
        self.store.binary_search_by_key(key, |&(k, _)| k)
    }

    pub fn events(&self) -> &Vec<StoreEvent<K, T, M>> {
        self.events.as_ref().expect("Event hold option is disabled. Call new(true).")
    }

    pub fn update_at_idx(&mut self, idx: usize, new_value: T, metadata: M) {
        if self.events.is_none() {
            let e = &self.store[idx];
            self.store[idx] = (e.0, new_value);
        } else {
            let e = &self.store[idx].clone();
            self.store[idx] = (e.0, new_value.clone());
            self.fire_event(|| StoreEvent::Changed {
                from_to: vec![(e.clone(), (e.0, new_value))], removed: vec![], metadata    
            });
        }
    }

    pub fn replace(&mut self, k: &K, metadata: M, f: impl FnOnce(Option<&T>) -> T) {
        if self.events.is_none() {
            match self.find(k) {
                Ok(idx) => {
                    let current = &self.store[idx];
                    let new_value = f(Some(&current.1));
                    self.store[idx] = (*k, new_value);
                }
                Err(idx) => {
                    let new_value = f(None);
                    self.store.insert(idx, (*k, new_value));
                }
            }
        } else {
            match self.find(k) {
                Ok(idx) => {
                    let current = self.store[idx].clone();
                    let new_value = f(Some(&current.1));
                    self.store[idx] = (*k, new_value.clone());
                    self.fire_event(|| StoreEvent::Changed {
                        from_to: vec![(current, (*k, new_value))], removed: vec![], metadata
                    });
                }
                Err(idx) => {
                    let new_value = f(None);
                    self.store.insert(idx, (*k, new_value.clone()));
                    self.fire_event(|| StoreEvent::Added {
                        added: new_value, metadata
                    })
                }
            }
        }
    }

    pub fn replace_mut(&mut self, k: &K, metadata: M, f: impl FnOnce(Option<&mut T>) -> Option<T>) {
        if self.events.is_none() {
            match self.find(k) {
                Ok(idx) => {
                    let current = &mut self.store[idx];
                    match f(Some(&mut current.1)) {
                        None => {}
                        Some(new_value) => {
                            self.store[idx] = (*k, new_value);
                        }
                    }
                }
                Err(idx) => {
                    match f(None) {
                        None => {}
                        Some(new_value) => {
                            self.store.insert(idx, (*k, new_value));
                        }
                    }
                }
            }
        } else {
            match self.find(k) {
                Ok(idx) => {
                    let current = &mut self.store[idx];
                    let backup = current.clone();
                    let new_value = match f(Some(&mut current.1)) {
                        None => current.1.clone(),
                        Some(new_value) => {
                            self.store[idx] = (*k, new_value.clone());
                            new_value
                        }
                    };
                
                    self.fire_event(|| StoreEvent::Changed {
                        from_to: vec![(backup, (*k, new_value))], removed: vec![], metadata
                    });
                }
                Err(idx) => {
                    match  f(None) {
                        None => {}
                        Some(value) => {
                            self.store.insert(idx, (*k, value.clone()));
                            self.fire_event(|| StoreEvent::Added {
                                added: value, metadata
                            })
                        }
                    }
                }
            }
        }
    }

    #[inline]
    pub fn head_entry_option(&self) -> Option<&(K, T)> {
        self.iter().next()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Finds the entry with the largest key that is less than or equal to the given key,
    /// and returns an iterator starting from that position.
    ///
    /// This method performs a binary search to find the entry whose key is:
    /// - Equal to `k` (exact match), or
    /// - The largest key that is less than `k` (closest predecessor)
    ///
    /// # Arguments
    ///
    /// * `k` - The key to search for
    ///
    /// # Returns
    ///
    /// An iterator that yields entries starting from the found position.
    /// If no entry with key <= `k` exists, the iterator will be empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use klavier_helper::store::Store;
    /// let mut store: Store<i32, &str, ()> = Store::new(false);
    /// store.add(10, "ten", ());
    /// store.add(20, "twenty", ());
    /// store.add(30, "thirty", ());
    ///
    /// let mut iter = store.just_before(15);
    /// assert_eq!(iter.next(), Some(&(10, "ten")));
    /// assert_eq!(iter.next(), Some(&(20, "twenty")));
    /// assert_eq!(iter.next(), Some(&(30, "thirty")));
    /// assert_eq!(iter.next(), None);
    ///
    /// let mut iter = store.just_before(5);
    /// assert_eq!(iter.next(), None);  // No key <= 5
    /// ```
    ///
    /// # Performance
    ///
    /// Time complexity: O(log n) where n is the number of entries in the store.
    pub fn just_before(&self, k: K) -> Iter<'_, (K, T)> {
        if self.store.is_empty() {
            return self.store[0..0].iter();
        }

        match self.find(&k) {
            Ok(idx) => {
                self.store[idx..].iter()
            },
            Err(ins_pt) => {
                if ins_pt == 0 {
                    self.store[0..0].iter()
                } else {
                    self.store[ins_pt - 1..].iter()
                }
            }
        }
    }

    /// Finds the entry with the smallest key that is greater than or equal to the given key,
    /// and returns an iterator starting from that position.
    ///
    /// This method performs a binary search to find the entry whose key is:
    /// - Equal to `k` (exact match), or
    /// - The smallest key that is greater than `k` (closest successor)
    ///
    /// # Arguments
    ///
    /// * `k` - The key to search for
    ///
    /// # Returns
    ///
    /// An iterator that yields entries starting from the found position.
    /// If no entry with key >= `k` exists, the iterator will be empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use klavier_helper::store::Store;
    /// let mut store: Store<i32, &str, ()> = Store::new(false);
    /// store.add(10, "ten", ());
    /// store.add(20, "twenty", ());
    /// store.add(30, "thirty", ());
    ///
    /// let mut iter = store.just_after(15);
    /// assert_eq!(iter.next(), Some(&(20, "twenty")));
    /// assert_eq!(iter.next(), Some(&(30, "thirty")));
    /// assert_eq!(iter.next(), None);
    ///
    /// let mut iter = store.just_after(100);
    /// assert_eq!(iter.next(), None);  // No key >= 100
    /// ```
    ///
    /// # Performance
    ///
    /// Time complexity: O(log n) where n is the number of entries in the store.
    pub fn just_after(&self, k: K) -> Iter<'_, (K, T)> {
        if self.store.is_empty() {
            return self.store[0..0].iter();
        }

        match self.find(&k) {
            Ok(idx) => {
                self.store[idx..].iter()
            },
            Err(ins_pt) => {
                if ins_pt >= self.store.len() {
                    self.store[0..0].iter()
                } else {
                    self.store[ins_pt..].iter()
                }
            }
        }
    }

    pub fn retain_values<F>(&mut self, metadata: M, f: F) -> Vec<(K, T)>
      where F: Fn(&T) -> bool, K: Clone, T: Clone
    {
        let mut removed: Vec<(K, T)> = vec![];

        self.store.retain(|(k, v)| {
            if !f(v) {
                removed.push((*k, v.clone()));
                false
            } else {
                true
            }
        });

        self.fire_event(|| StoreEvent::BulkAddedRemoved { added: vec![], removed: removed.clone(), metadata });
        removed
    }
}

impl<K, T, M> Index<usize> for Store<K, T, M> where K: Ord + Copy, T: Clone {
    type Output = (K, T);

    fn index(&self, index: usize) -> &Self::Output {
        &self.store[index]
    }
}

#[cfg(test)]
mod tests {
    use crate::store::StoreEvent;
    use super::Store;

    #[test]
    fn inclusive() {
        let mut store = Store::new(false);
        store.add(10, "10", "");

        let (idx, itr) = store.range(0..=i32::MAX);
        assert_eq!(idx, 0);
        assert_eq!(itr.len(), 1);
        assert_eq!(itr[0], (10, "10"));
    }

    #[test]
    fn replace() {
        let mut store = Store::new(false);
        store.add(10, "10".to_owned(), "");

        store.replace(&10, "foo", |v| {
            format!("{}2", v.unwrap())
        });

        assert_eq!(store.len(), 1);
        assert_eq!(store[0].0, 10);
        assert_eq!(store[0].1, "102".to_owned());

        store.replace(&20, "foo", |v| {
            assert!(v.is_none());
            "20".to_owned()
        });
        assert_eq!(store.len(), 2);
        assert_eq!(store[0].0, 10);
        assert_eq!(store[0].1, "102".to_owned());
        assert_eq!(store[1].0, 20);
        assert_eq!(store[1].1, "20".to_owned());
    }

    #[test]
    fn replace_with_event() {
        let mut store = Store::new(true);
        store.add(10, "10".to_owned(), "");
        store.clear_events();

        store.replace(&10, "foo", |v| {
            format!("{}2", v.unwrap())
        });
        let events = store.events();
        assert_eq!(events.len(), 1);
        if let StoreEvent::Changed { from_to, removed, metadata } = &events[0] {
            assert_eq!(from_to.len(), 1);
            assert_eq!(removed.len(), 0);
            assert_eq!(*metadata, "foo");

            let (from, to) = &from_to[0];
            assert_eq!(from.0, 10);
            assert_eq!(from.1, "10".to_owned());
            assert_eq!(to.0, 10);
            assert_eq!(to.1, "102".to_owned());
        } else {
            panic!("Unexpected event {:?}", events);
        }

        assert_eq!(store.len(), 1);
        assert_eq!(store[0].0, 10);
        assert_eq!(store[0].1, "102".to_owned());
        store.clear_events();

        store.replace(&20, "bar", |v| {
            assert!(v.is_none());
            "20".to_owned()
        });
        assert_eq!(store.len(), 2);
        assert_eq!(store[0].0, 10);
        assert_eq!(store[0].1, "102".to_owned());
        assert_eq!(store[1].0, 20);
        assert_eq!(store[1].1, "20".to_owned());

        let events = store.events();
        assert_eq!(events.len(), 1);
        if let StoreEvent::Added { added, metadata } = &events[0] {
            assert_eq!(*added, "20".to_owned());
            assert_eq!(*metadata, "bar");
        } else {
            panic!("Unexpected event {:?}", events);
        }

    }

    #[test]
    fn replace_mut_with_events() {
        let mut store: Store<i32, Vec<i32>, &str> = Store::new(true);
        store.replace_mut(&10, "meta", |opt| {
            assert_eq!(opt, None);
            None
        });
        assert_eq!(store.len(), 0);
        assert_eq!(store.events().len(), 0);

        store.replace_mut(&10, "meta", |opt| {
            assert_eq!(opt, None);
            Some(vec![1, 2, 3])
        });
        let events = store.events();
        assert_eq!(events.len(), 1);
        if let StoreEvent::Added { added, metadata } = &events[0] {
            assert_eq!(added, &vec![1, 2, 3]);
            assert_eq!(*metadata, "meta");
        } else {
            panic!("Unexpected event {:?}", events);
        }
        assert_eq!(store.len(), 1);
        assert_eq!(store[0], (10, vec![1, 2, 3]));
        store.clear_events();

        // In-place update
        store.replace_mut(&10, "meta", |opt| {
            if let Some(value) = opt {
                value[0] = 100;
                None
            } else {
                panic!("Unexpected state.");
            }
        });
        let events = store.events();
        assert_eq!(events.len(), 1);
        if let StoreEvent::Changed { from_to, removed, metadata } = &events[0] {
            assert_eq!(from_to.len(), 1);
            let from_to = &from_to[0];
            assert_eq!(from_to, &((10, vec![1, 2, 3]), (10, vec![100, 2, 3])));
            assert_eq!(removed.len(), 0);
            assert_eq!(*metadata, "meta");
        } else {
            panic!("Unexpected event {:?}", events);
        }
        assert_eq!(store.len(), 1);
        assert_eq!(store[0], (10, vec![100, 2, 3]));
        store.clear_events();

        store.replace_mut(&10, "meta", |opt| {
            if let Some(_value) = opt {
                Some(vec![2, 3, 4])
            } else {
                panic!("Unexpected state.");
            }
        });
        let events = store.events();
        assert_eq!(events.len(), 1);
        if let StoreEvent::Changed { from_to, removed, metadata } = &events[0] {
            assert_eq!(from_to.len(), 1);
            let from_to = &from_to[0];
            assert_eq!(from_to, &((10, vec![100, 2, 3]), (10, vec![2, 3, 4])));
            assert_eq!(removed.len(), 0);
            assert_eq!(*metadata, "meta");
        } else {
            panic!("Unexpected event {:?}", events);
        }
        assert_eq!(store.len(), 1);
        assert_eq!(store[0], (10, vec![2, 3, 4]));
        store.clear_events();
     }

    #[test]
    fn replace_mut() {
        let mut store: Store<i32, Vec<i32>, ()> = Store::new(false);
        store.replace_mut(&10, (), |opt| {
            assert_eq!(opt, None);
            None
        });
        assert_eq!(store.len(), 0);

        store.replace_mut(&10, (), |opt| {
            assert_eq!(opt, None);
            Some(vec![1, 2, 3])
        });
        assert_eq!(store.len(), 1);
        assert_eq!(store[0], (10, vec![1, 2, 3]));

        // In-place update
        store.replace_mut(&10, (), |opt| {
            if let Some(value) = opt {
                value[0] = 100;
                None
            } else {
                panic!("Unexpected state.");
            }
        });
        assert_eq!(store.len(), 1);
        assert_eq!(store[0], (10, vec![100, 2, 3]));

        store.replace_mut(&10, (), |opt| {
            if let Some(_value) = opt {
                Some(vec![2, 3, 4])
            } else {
                panic!("Unexpected state.");
            }
        });
        assert_eq!(store.len(), 1);
        assert_eq!(store[0], (10, vec![2, 3, 4]));
    }

    #[test]
    fn borrow() {
        let mut store: Store<i32, String, ()> = Store::new(false);
        {
            let b: &[(i32, String)] = &store;
            assert_eq!(b.len(), 0);
        }

        store.add(1, "Hello".to_owned(), ());
        {
            let b: &[(i32, String)] = &store;
            assert_eq!(b.len(), 1);
            assert_eq!(b[0], (1, "Hello".to_owned()));
        }

        store.add(100, "World".to_owned(), ());
        let b: &[(i32, String)] = &store;
        assert_eq!(b.len(), 2);
        assert_eq!(b[0], (1, "Hello".to_owned()));
        assert_eq!(b[1], (100, "World".to_owned()));
    }

    #[test]
    fn retain() {
        let mut store: Store<i32, &str, i32> = Store::new(true);
        store.add(0, "0", 0);
        store.add(1, "11", 0);
        store.add(2, "22", 0);
        store.add(3, "3", 0);

        store.clear_events();
        let removed = store.retain_values(123, |v| v.len() == 1);
        assert_eq!(store.len(), 2);
        assert_eq!(store.first().unwrap(), &(0, "0"));
        assert_eq!(store.get(1).unwrap(), &(3, "3"));
        assert_eq!(removed, vec![(1, "11"), (2, "22")]);

        let events = store.events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            StoreEvent::BulkAddedRemoved { added, removed, metadata } => {
                assert_eq!(added.len(), 0);
                assert_eq!(removed, &vec![(1, "11"), (2, "22")]);
                assert_eq!(metadata, &123);
            }
            _ => panic!("Logic error."),
        }
    }

    #[test]
    fn store_just_before_empty() {
        let store: Store<i32, &str, &str> = Store::new(false);
        let mut iter = store.just_before(10);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn store_just_before_one() {
        let mut store = Store::new(false);
        store.add(10, "10", "");

        let mut iter = store.just_before(9);
        assert_eq!(iter.next(), None);

        let mut iter = store.just_before(10);
        assert_eq!(iter.next(), Some(&(10, "10")));
        assert_eq!(iter.next(), None);

        let mut iter = store.just_before(11);
        assert_eq!(iter.next(), Some(&(10, "10")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn store_just_before_multiple() {
        let mut store = Store::new(false);
        store.add(10, "10", "");
        store.add(20, "20", "");
        store.add(30, "30", "");

        let mut iter = store.just_before(5);
        assert_eq!(iter.next(), None);

        let mut iter = store.just_before(10);
        assert_eq!(iter.next(), Some(&(10, "10")));
        assert_eq!(iter.next(), Some(&(20, "20")));
        assert_eq!(iter.next(), Some(&(30, "30")));
        assert_eq!(iter.next(), None);

        let mut iter = store.just_before(15);
        assert_eq!(iter.next(), Some(&(10, "10")));
        assert_eq!(iter.next(), Some(&(20, "20")));
        assert_eq!(iter.next(), Some(&(30, "30")));
        assert_eq!(iter.next(), None);

        let mut iter = store.just_before(25);
        assert_eq!(iter.next(), Some(&(20, "20")));
        assert_eq!(iter.next(), Some(&(30, "30")));
        assert_eq!(iter.next(), None);

        let mut iter = store.just_before(100);
        assert_eq!(iter.next(), Some(&(30, "30")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn store_just_after_empty() {
        let store: Store<i32, &str, &str> = Store::new(false);
        let mut iter = store.just_after(10);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn store_just_after_one() {
        let mut store = Store::new(false);
        store.add(10, "10", "");

        let mut iter = store.just_after(9);
        assert_eq!(iter.next(), Some(&(10, "10")));
        assert_eq!(iter.next(), None);

        let mut iter = store.just_after(10);
        assert_eq!(iter.next(), Some(&(10, "10")));
        assert_eq!(iter.next(), None);

        let mut iter = store.just_after(11);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn store_just_after_multiple() {
        let mut store = Store::new(false);
        store.add(10, "10", "");
        store.add(20, "20", "");
        store.add(30, "30", "");

        let mut iter = store.just_after(5);
        assert_eq!(iter.next(), Some(&(10, "10")));
        assert_eq!(iter.next(), Some(&(20, "20")));
        assert_eq!(iter.next(), Some(&(30, "30")));
        assert_eq!(iter.next(), None);

        let mut iter = store.just_after(10);
        assert_eq!(iter.next(), Some(&(10, "10")));
        assert_eq!(iter.next(), Some(&(20, "20")));
        assert_eq!(iter.next(), Some(&(30, "30")));
        assert_eq!(iter.next(), None);

        let mut iter = store.just_after(15);
        assert_eq!(iter.next(), Some(&(20, "20")));
        assert_eq!(iter.next(), Some(&(30, "30")));
        assert_eq!(iter.next(), None);

        let mut iter = store.just_after(25);
        assert_eq!(iter.next(), Some(&(30, "30")));
        assert_eq!(iter.next(), None);

        let mut iter = store.just_after(30);
        assert_eq!(iter.next(), Some(&(30, "30")));
        assert_eq!(iter.next(), None);

        let mut iter = store.just_after(100);
        assert_eq!(iter.next(), None);
    }
}
