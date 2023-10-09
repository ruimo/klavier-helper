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
    
    pub fn range<R>(&self, bounds: R) -> (usize, Iter<'_, (K, T)>) where R: RangeBounds<K> {
        if self.store.is_empty() {
            return (0, self.store[0..0].iter())
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
                        Err(i) => i + 1,
                    }
                },
            };

        if start_bound == end_bound {
            (0, self.store[0..0].iter())
        } else {
            (start_bound, self.store[start_bound..end_bound].iter())
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

    pub fn update(&mut self, idx: usize, new_value: T) {
        let e = &self.store[idx];
        self.store[idx] = (e.0, new_value);
    }

    #[inline]
    pub fn head_entry_option(&self) -> Option<&(K, T)> {
        self.iter().next()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

impl<K, T, M> Index<usize> for Store<K, T, M> where K: Ord + Copy, T: Clone {
    type Output = (K, T);

    fn index(&self, index: usize) -> &Self::Output {
        &self.store[index]
    }
}
