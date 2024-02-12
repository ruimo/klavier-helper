use std::{ops::{RangeBounds, Bound, Index}, slice::Iter};

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
            if let Some(r) = self.add_internal(k.clone(), v.clone()) {
                removed.push((k.clone(), r));
            }
        }
        self.fire_event(|| StoreEvent::Changed { from_to: result, removed: removed.clone(), metadata });
        removed
    }

    pub fn bulk_add(&mut self, recs: Vec<(K, T)>, metadata: M) -> Vec<(K, T)> where K: Clone, T: Clone {
        let mut removed = vec![];

        for (k, v) in recs.iter() {
            if let Some(r) = self.add_internal(k.clone(), v.clone()) {
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

    pub fn finder(&self) -> Finder<K, T, M> {
        Finder {
            store: self,
            locator: None,
        }
    }
}

pub struct Finder<'a, K: Ord + Copy, T: Clone, M> {
    store: &'a Store<K, T, M>,
    locator: Option<usize>,
}

impl <'a, K: Ord + Copy, T: Clone, M> Finder<'a, K, T, M> {
    fn find_locator(&mut self, k: K) -> Option<usize> {
        match self.store.index(k) {
            Ok(idx) => {
                Some(idx)
            }
            Err(ins_pt) => {
                if ins_pt == 0 {
                    None
                } else {
                    Some(ins_pt - 1)
                }
            }
        }
    }

    pub fn just_before(&mut self, k: K) -> Option<&(K, T)> {
        let len = self.store.len();
        if len == 0 {
            return None
        }

        match self.locator {
            Some(locator) =>
                if locator == len - 1 {
                    let t = &self.store[locator];
                    if t.0 <= k {
                        Some(t)
                    } else {
                        self.find_locator(k).map(|l| &self.store[l])
                    }
                } else {
                    if self.store[locator].0 <= k && k < self.store[locator + 1].0 {
                        Some(&self.store[locator])
                    } else {
                        self.find_locator(k).map(|l| &self.store[l])
                    }
                }
            None => self.find_locator(k).map(|l| &self.store[l])
        }
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
    fn finder_empty() {
        let store: Store<i32, &str, &str> = Store::new(false);

        let mut finder = store.finder();
        assert_eq!(finder.just_before(0), None);
        assert_eq!(finder.just_before(1), None);
    }

    #[test]
    fn finder_one() {
        let mut store = Store::new(false);
        store.add(10, "10", "");

        let mut finder = store.finder();
        assert_eq!(finder.just_before(9), None);
        assert_eq!(finder.just_before(10), Some(&(10, "10")));
        assert_eq!(finder.just_before(11), Some(&(10, "10")));
    }

    #[test]
    fn finder() {
        let mut store = Store::new(false);
        store.add(10, "10", "");
        store.add(20, "20", "");

        let mut finder = store.finder();
        assert_eq!(finder.just_before(9), None);
        assert_eq!(finder.just_before(10), Some(&(10, "10")));
        assert_eq!(finder.just_before(11), Some(&(10, "10")));
        assert_eq!(finder.just_before(19), Some(&(10, "10")));
        assert_eq!(finder.just_before(20), Some(&(20, "20")));
        assert_eq!(finder.just_before(21), Some(&(20, "20")));
    }

    #[test]
    fn inclusive() {
        let mut store = Store::new(false);
        store.add(10, "10", "");

        let (idx, mut itr) = store.range(0..=i32::MAX);
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
            assert_eq!(v.is_none(), true);
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
            assert_eq!(v.is_none(), true);
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
            if let Some(value) = opt {
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
            if let Some(value) = opt {
                Some(vec![2, 3, 4])
            } else {
                panic!("Unexpected state.");
            }
        });
        assert_eq!(store.len(), 1);
        assert_eq!(store[0], (10, vec![2, 3, 4]));
    }
}