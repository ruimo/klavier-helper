use std::{collections::{BTreeMap, btree_map::{Entry, self}}, borrow::Borrow, ops::RangeBounds};

#[derive(Clone)]
pub enum BagStoreEvent<K, T, M> {
    Added { added: T, metadata: M },
    AddedVec { added: Vec<T>, metadata: M },
    Removed(T),
    RemovedVec(Vec<T>),
    ClearedAll,
    Changed { from_to: Vec<((K, T), (K, T))>, removed: Vec<(K, T)>, metadata: M },
    BulkAddedRemoved { added: Vec<(K, T)>, removed: Vec<(K, T)>, metadata: M },
}

pub struct Iter<'a, K, T> {
    iter: btree_map::Iter<'a, K, Vec<T>>,
    key: Option<&'a K>,
    sub_iter: std::slice::Iter<'a, T>,
    #[allow(dead_code)]
    empty: Vec<T>,
}

pub struct RangeIter<'a, K, T> {
    iter: std::collections::btree_map::Range<'a, K, Vec<T>>,
    key: Option<&'a K>,
    sub_iter: std::slice::Iter<'a, T>,
    #[allow(dead_code)]
    empty: Vec<T>,
}

impl<'a, K, T> Iterator for Iter<'a, K, T> {
    type Item = (&'a K, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        match self.sub_iter.next() {
            None => {
                match self.iter.next() {
                    None => None,
                    Some((k, vec)) => {
                        self.key = Some(k);
                        self.sub_iter = vec.iter();
                        self.next()
                    },
                }
            },
            Some(e) => {
                Some((self.key.unwrap(), e))
            }
        }
    }
}

impl<'a, K, T> Iterator for RangeIter<'a, K, T> {
    type Item = (&'a K, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        match self.sub_iter.next() {
            None => {
                match self.iter.next() {
                    None => None,
                    Some((k, vec)) => {
                        self.key = Some(k);
                        self.sub_iter = vec.iter();
                        self.next()
                    },
                }
            },
            Some(e) => {
                Some((self.key.unwrap(), e))
            }
        }
    }
}

pub struct BagStore<K, T, M> {
    store: BTreeMap<K, Vec<T>>,
    empty: Vec<T>,
    events: Option<Vec<BagStoreEvent<K, T, M>>>,
    count: usize,
}

impl<K, T, M> BagStore<K, T, M> where K:Ord + 'static, T: PartialEq + Clone + 'static {
    pub fn new(hold_events: bool) -> Self {
        Self {
            store: BTreeMap::new(),
            empty: Vec::new(),
            events: if hold_events { Some(vec![]) } else { None },
            count: 0,
        }
    }
    
    pub fn get<'a>(&'a self, key: K) -> &'a Vec<T> {
        self.store.get(&key).unwrap_or(&self.empty)
    }

    fn fire_event<F>(&mut self, f: F) where F: FnOnce() -> BagStoreEvent<K, T, M> {
        if let Some(events) = self.events.as_mut() {
            events.push(f());
        }
    }
    
    pub fn pop_first(&mut self) -> Option<(K, Vec<T>)> where K: Clone + std::fmt::Debug {
        let key = self.store.iter().next().map(|e| e.0.clone());
        
        if let Some(k) = key {
            let ret = self.store.remove_entry(&k);
            if let Some((k, v)) = ret {
                self.fire_event(|| BagStoreEvent::RemovedVec(v.clone()));
                self.count -= v.len();
                return Some((k, v));
            }
        }

        None
    }

    pub fn peek_last(&self) -> Option<(&K, &Vec<T>)> {
        self.store.iter().last()
    }
    
    pub fn add(&mut self, key: K, e: T, metadata: M) where K: Clone, T: Clone {
        self.add_internal(key.clone(), e.clone());
        self.fire_event(|| BagStoreEvent::Added { added: e, metadata });
    }
    
    // Does not notify observers.
    fn add_internal(&mut self, key: K, e: T) where K: Clone, T: Clone {
        self.store.entry(key.clone()).or_insert(Vec::new()).push(e.clone());
        self.count += 1;
    }
    
    pub fn add_vec(&mut self, key: K, e: Vec<T>, metadata: M) where T: Clone {
        let vec = self.events.as_ref().map(|_| e.clone());
        
        self.add_vec_internal(key, e);
        
        if let Some(v) = vec {
            self.fire_event(|| BagStoreEvent::AddedVec{ added: v, metadata });
        }
    }

    fn add_vec_internal(&mut self, key: K, mut e: Vec<T>) where K:Ord + 'static {
        self.count += e.len();
        match self.store.entry(key) {
            Entry::Occupied(mut occ) => {
                occ.get_mut().append(&mut e);
            },
            Entry::Vacant(vac) => {
                vac.insert(e);
            },
        }
    }
    
    pub fn remove(&mut self, key: &K, e: &T) -> Option<T> where K: Clone, T: Clone {
        let ret = self.remove_internal(key, e);
        if ret.is_some() {
            self.fire_event(|| BagStoreEvent::Removed(e.clone()));
        }
        ret
    }
    
    pub fn remove_vec(&mut self, key: &K, value_table: &Vec<T>) where K: Clone, T: Clone {
        let removed: Vec<T> = self.remove_vec_internal(key, value_table);
        self.fire_event(|| BagStoreEvent::RemovedVec(removed));
    }
    
    fn remove_vec_internal(&mut self, key: &K, value_table: &Vec<T>) -> Vec<T> where K: Clone, T: Clone {
        let mut removed: Vec<T> = vec![];
        if let Some(cur) = self.store.get_mut(key) {
            for e in value_table.iter() {
                if let Some(idx) = cur.iter().position(|i| *i == *e) {
                    removed.push(cur.remove(idx));
                }
            }
        }
        self.count -= removed.len();
        removed
    }
    
    // Does not notify observers
    fn remove_internal(&mut self, key: &K, e: &T) -> Option<T> where T: Clone {
        let mut entry_becomes_empty = false;
        let ret = self.store.get_mut(&key).and_then(|vec| {
            vec.iter().position(|o| *o == *e).map(|idx| {
                let e = vec.remove(idx);
                if vec.is_empty() {
                    entry_becomes_empty = true;
                }
                e
            })  
        });
        if ret.is_some() {
            if entry_becomes_empty {
                self.store.remove(&key);
            }
            self.count -= 1;
        }
        ret
    }
    
    pub fn clear(&mut self) {
        self.store.clear();
        self.count = 0;
        self.fire_event(|| BagStoreEvent::ClearedAll);
    }
    
    pub fn range_vec<B, R>(&self, range: R) -> std::collections::btree_map::Range<'_, K, Vec<T>>
        where B: Ord + ?Sized, K: Borrow<B> + Ord, R: RangeBounds<B>
    {
        self.store.range(range)
    }

    pub fn range<'a, B, R>(&'a self, range: R) -> RangeIter<'a, K, T> 
        where B: Ord + ?Sized, K: Borrow<B> + Ord, R: RangeBounds<B>
    {
        RangeIter::<'a, K, T> {
            iter: self.range_vec(range),
            key: None,
            empty: vec![],
            sub_iter: self.empty.iter(),
        }
    }

    pub fn change(&mut self, from_to: &[((K, T), (K, T))], metadata: M) where T: Clone, K: Clone {
        let mut result: Vec<((K, T), (K, T))> = Vec::with_capacity(from_to.len());

        // Remove all 'from's in advance because adding 'to' will replace(remove) the existing 'from'.
        for ((from_k, from_v), to) in from_to.iter() {
            if let Some(removed) = self.remove_internal(&from_k.clone(), &from_v.clone()) {
                result.push((
                    ((*from_k).clone(), removed), to.clone()
                ));
            }
        }

        for (_, (k, v)) in result.iter() {
            self.add_internal(k.clone(), v.clone());
        }

        self.fire_event(|| BagStoreEvent::Changed { from_to: result, removed: vec![], metadata });
    }

    pub fn bulk_add(&mut self, models: Vec<(K, T)>, metadata: M) where K: Clone, T: Clone {
        for (key, value) in models.iter() {
            self.add_internal(key.clone(), value.clone());
        }
        self.fire_event(|| BagStoreEvent::BulkAddedRemoved { added: models, removed: vec![], metadata });
    }

    pub fn bulk_remove(&mut self, models: &[(K, T)], metadata: M) -> Vec<(K, T)> where K: Clone, T: Clone {
        let mut removed: Vec<(K, T)> = Vec::with_capacity(models.len());

        for (k, t) in models.iter() {
            if let Some(r) = self.remove_internal(k, t) {
                removed.push((k.clone(), r));
            }
        }

        self.fire_event(|| BagStoreEvent::BulkAddedRemoved { added: vec![], removed: removed.clone(), metadata });
        removed
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }
    
    pub fn iter_vec(&self) -> btree_map::Iter<'_, K, Vec<T>> {
        self.store.iter()
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, K, T> {
        Iter::<'a, K, T> {
            iter: self.iter_vec(),
            key: None,
            empty: vec![],
            sub_iter: self.empty.iter(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear_events(&mut self) {
        if let Some(events) = self.events.as_mut() {
            events.clear();
        }
    }

    pub fn events(&self) -> &Vec<BagStoreEvent<K, T, M>> {
        self.events.as_ref().expect("Event hold option is disabled. Call new(true).")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::btree_map::Range;
    
    use crate::{nan_free_f32::NanFreeF32};
    use super::{BagStore, BagStoreEvent};
    
    #[test]
    fn get_empty() {
        let store: BagStore<NanFreeF32, &str, i32> = BagStore::new(false);
        assert!(store.get(0.0.into()).is_empty());
    }
    
    #[test]
    fn add_one() {
        let mut store: BagStore<NanFreeF32, &str, i32> = BagStore::new(false);
        store.add(0.0.into(), "Hello", 0);
        assert!(store.get(1.0.into()).is_empty());
        let result = store.get(0.0.into());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "Hello");
    }
    
    #[test]
    fn add_multiple() {
        let mut store: BagStore<NanFreeF32, &str, i32> = BagStore::new(false);
        store.add(1.0.into(), "Hello", 0);
        store.add(1.0.into(), "World", 0);
        store.add(2.0.into(), "Foo", 0);
        assert!(store.get(0.0.into()).is_empty());
        let result = store.get(1.0.into());
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"Hello"));
        assert!(result.contains(&"World"));
        
        let result = store.get(2.0.into());
        assert_eq!(result.len(), 1);
        assert!(result.contains(&"Foo"));
    }
    
    #[test]
    fn remove_none() {
        let mut store: BagStore<NanFreeF32, &str, i32> = BagStore::new(false);
        assert_eq!(store.remove(&1.0.into(), &"Hello"), None);
    }
    
    #[test]
    fn remove() {
        let mut store: BagStore<NanFreeF32, &str, i32> = BagStore::new(false);
        store.add(0.0.into(), "Hello", 0);
        store.add(0.0.into(), "World", 0);
        store.add(1.0.into(), "Foo", 0);
        
        assert_eq!(store.remove(&0.0.into(), &"Hello"), Some("Hello"));
        let vec0 = store.get(0.0.into());
        assert_eq!(vec0.len(), 1);
        assert_eq!(vec0[0], "World");
        let vec1 = store.get(1.0.into());
        assert_eq!(vec1.len(), 1);
        assert_eq!(vec1[0], "Foo");
        assert_eq!(store.remove(&1.0.into(), &"Foo"), Some("Foo"));
        let mut z = store.iter_vec();
        let next = z.next();
        assert_eq!(*next.unwrap().0, NanFreeF32::from(0.0));
        assert_eq!(next.unwrap().1[0], "World");
        
        let next = z.next();
        assert_eq!(next, None)
    }
    
    #[test]
    fn pop_first() {
        let mut store: BagStore<NanFreeF32, &str, i32> = BagStore::new(false);
        assert_eq!(store.pop_first(), None);
        
        store.add(0.0.into(), "Hello", 0);
        assert_eq!(store.pop_first(), Some((0.0.into(), vec!["Hello"])));
        assert_eq!(store.pop_first(), None);
    }
    
    #[test]
    fn range_vec() {
        let mut store: BagStore<NanFreeF32, &str, i32> = BagStore::new(false);
        store.add(0.0.into(), "Hello", 0);
        store.add(0.0.into(), "World", 0);
        store.add(1.0.into(), "Foo", 0);
        store.add(1.0.into(), "Bar", 0);
        
        let mut z: Range<'_, NanFreeF32, Vec<&str>> = 
        store.range_vec::<NanFreeF32, std::ops::Range<NanFreeF32>>(NanFreeF32::from(-0.1)..NanFreeF32::from(0.0));
        assert_eq!(z.next(), None);
        
        let mut z: Range<'_, NanFreeF32, Vec<&str>> = 
        store.range_vec::<NanFreeF32, std::ops::Range<NanFreeF32>>(NanFreeF32::from(-0.1)..NanFreeF32::from(0.01));
        let t: (&NanFreeF32, &Vec<&str>) = z.next().unwrap();
        assert_eq!(*t.0, NanFreeF32::from(0.0));
        assert_eq!(t.1.len(), 2);
        assert_eq!(t.1[0], "Hello");
        assert_eq!(t.1[1], "World");
        assert_eq!(z.next(), None);
        
        let mut z: Range<'_, NanFreeF32, Vec<&str>> = 
        store.range_vec::<NanFreeF32, std::ops::Range<NanFreeF32>>(NanFreeF32::from(0.0)..NanFreeF32::from(1.0));
        let t: (&NanFreeF32, &Vec<&str>) = z.next().unwrap();
        assert_eq!(*t.0, NanFreeF32::from(0.0));
        assert_eq!(t.1.len(), 2);
        assert_eq!(t.1[0], "Hello");
        assert_eq!(t.1[1], "World");
        assert_eq!(z.next(), None);
        
        let mut z: Range<'_, NanFreeF32, Vec<&str>> = 
        store.range_vec::<NanFreeF32, std::ops::RangeInclusive<NanFreeF32>>(NanFreeF32::from(0.0)..=NanFreeF32::from(1.0));
        let t: (&NanFreeF32, &Vec<&str>) = z.next().unwrap();
        assert_eq!(*t.0, NanFreeF32::from(0.0));
        assert_eq!(t.1.len(), 2);
        assert_eq!(t.1[0], "Hello");
        assert_eq!(t.1[1], "World");
        let t: (&NanFreeF32, &Vec<&str>) = z.next().unwrap();
        assert_eq!(*t.0, NanFreeF32::from(1.0));
        assert_eq!(t.1.len(), 2);
        assert_eq!(t.1[0], "Foo");
        assert_eq!(t.1[1], "Bar");
        assert_eq!(z.next(), None);
        
        let mut z: Range<'_, NanFreeF32, Vec<&str>> = 
        store.range_vec::<NanFreeF32, std::ops::Range<NanFreeF32>>(NanFreeF32::from(0.9)..NanFreeF32::from(1.1));
        let t: (&NanFreeF32, &Vec<&str>) = z.next().unwrap();
        assert_eq!(*t.0, NanFreeF32::from(1.0));
        assert_eq!(t.1.len(), 2);
        assert_eq!(t.1[0], "Foo");
        assert_eq!(t.1[1], "Bar");
        assert_eq!(z.next(), None);
        
        let mut z: Range<'_, NanFreeF32, Vec<&str>> = 
        store.range_vec::<NanFreeF32, std::ops::Range<NanFreeF32>>(NanFreeF32::from(1.1)..NanFreeF32::from(2.0));
        assert_eq!(z.next(), None);
    }
    
    #[test]
    fn observe() {
        let mut store: BagStore<NanFreeF32, &str, i32> = BagStore::new(true);
        store.add(0.0.into(), "Hello", 123);

        {
            let events = store.events();
            assert_eq!(events.len(), 1);
            match events[0] {
                BagStoreEvent::Added { added, metadata } => {
                    assert_eq!(added, "Hello");
                    assert_eq!(metadata, 123);
                },
                _ => {
                    panic!("Test failed.");
                }   
            }
        }
            
        assert_eq!(store.remove(&0.0.into(), &"Hell"), None); // Not found.
        assert_eq!(store.events().len(), 1);
        
        assert_eq!(store.remove(&0.0.into(), &"Hello"), Some("Hello"));
        {
            let events = store.events();

            assert_eq!(events.len(), 2);
            let e = &events[1];
            match e {
                BagStoreEvent::Removed(s) => {
                    assert_eq!(*s, "Hello");
                },
                _ => {
                    panic!("Test failed.");
                }
            }
        }
    }
    
    #[test]
    fn change() {
        let mut store: BagStore<NanFreeF32, &str, i32> = BagStore::new(true);
        store.add(0.0.into(), "Hello", 0);
        store.add(0.0.into(), "World", 0);
        
        store.change(&vec![((0.0.into(), "Hello"), (1.0.into(), "Foo"))], 123);
        
        let vec0 = store.get(0.0.into());
        assert_eq!(vec0.len(), 1);
        assert_eq!(vec0[0], "World");
        
        let vec1 = store.get(1.0.into());
        assert_eq!(vec1.len(), 1);
        assert_eq!(vec1[0], "Foo");
        
        let events = store.events();
        assert_eq!(events.len(), 3);
        
        match &events[2] {
            BagStoreEvent::Changed { from_to, removed, metadata } => {
            assert_eq!(from_to.len(), 1);
            assert_eq!(removed.len(), 0);
            assert_eq!(from_to[0].0, (0.0.into(), "Hello"));
            assert_eq!(from_to[0].1, (1.0.into(), "Foo"));
            assert_eq!(*metadata, 123);
            },
            _ => panic!("Logic error."),
        }

//        if let BagStoreEvent::Change(c) = &events[2] {
//            let mc: &MyChanges = c.as_any()
//            .downcast_ref::<MyChanges>()
//            .expect("Logic error!");
//            assert_eq!(mc.changes.len(), 1);
//            assert_eq!(mc.changes[0].0, (0.0.into(), "Hello"));
//            assert_eq!(mc.changes[0].1, (1.0.into(), "Foo"));
//        } else {
            //panic!("Logic error.");
//        }
    }

    #[test]
    fn iter_test() {
        let mut store: BagStore<NanFreeF32, &str, i32> = BagStore::new(true);
        assert_eq!(store.iter().next(), None);

        store.add(0.0.into(), "Hello", 0);
        store.add(0.0.into(), "World", 0);
        store.add(1.0.into(), "Foo", 0);
        store.add(1.0.into(), "Bar", 0);
        store.add(2.0.into(), "Hoge", 0);

        let mut z = store.iter();
        assert_eq!(z.next(), Some((&0.0.into(), &"Hello")));
        assert_eq!(z.next(), Some((&0.0.into(), &"World")));
        assert_eq!(z.next(), Some((&1.0.into(), &"Foo")));
        assert_eq!(z.next(), Some((&1.0.into(), &"Bar")));
        assert_eq!(z.next(), Some((&2.0.into(), &"Hoge")));
        assert_eq!(z.next(), None);
    }

    #[test]
    fn range() {
        let mut store: BagStore<NanFreeF32, &str, i32> = BagStore::new(false);
        let mut z = store.range(..);
        assert_eq!(z.next(), None);

        store.add(0.0.into(), "Hello", 0);
        store.add(0.0.into(), "World", 0);
        store.add(1.0.into(), "Foo", 0);
        store.add(1.0.into(), "Bar", 0);
        store.add(2.0.into(), "Hoge", 0);

        let mut z = store.range(NanFreeF32::from(0.0)..);
        assert_eq!(z.next(), Some((&0.0.into(), &"Hello")));
        assert_eq!(z.next(), Some((&0.0.into(), &"World")));
        assert_eq!(z.next(), Some((&1.0.into(), &"Foo")));
        assert_eq!(z.next(), Some((&1.0.into(), &"Bar")));
        assert_eq!(z.next(), Some((&2.0.into(), &"Hoge")));
        assert_eq!(z.next(), None);

        let mut z = store.range(NanFreeF32::from(0.9)..);
        assert_eq!(z.next(), Some((&1.0.into(), &"Foo")));
        assert_eq!(z.next(), Some((&1.0.into(), &"Bar")));
        assert_eq!(z.next(), Some((&2.0.into(), &"Hoge")));
        assert_eq!(z.next(), None);


        let mut z = store.range(NanFreeF32::from(1.0)..);
        assert_eq!(z.next(), Some((&1.0.into(), &"Foo")));
        assert_eq!(z.next(), Some((&1.0.into(), &"Bar")));
        assert_eq!(z.next(), Some((&2.0.into(), &"Hoge")));
        assert_eq!(z.next(), None);

        let mut z = store.range(NanFreeF32::from(1.0)..NanFreeF32::from(1.5));
        assert_eq!(z.next(), Some((&1.0.into(), &"Foo")));
        assert_eq!(z.next(), Some((&1.0.into(), &"Bar")));
        assert_eq!(z.next(), None);

        let mut z = store.range(NanFreeF32::from(1.9)..);
        assert_eq!(z.next(), Some((&2.0.into(), &"Hoge")));
        assert_eq!(z.next(), None);
        
        let mut z = store.range(NanFreeF32::from(2.0)..);
        assert_eq!(z.next(), Some((&2.0.into(), &"Hoge")));
        assert_eq!(z.next(), None);

        let mut z = store.range(NanFreeF32::from(2.1)..);
        assert_eq!(z.next(), None);
    }
}
