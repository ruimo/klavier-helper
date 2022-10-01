use std::any::Any;

pub trait BulkRemove<K, T> where K: Clone, T: Clone {
    fn len(&self) -> usize;
    fn iter(&self) -> Box<dyn Iterator<Item = (K, T)> + '_>;
    fn clone_trait(&self) -> Box<dyn BulkRemove<K, T>>;
    fn as_any(&self) -> &dyn Any; // For down casting
}

impl<K: Clone, T: Clone> Clone for Box<dyn BulkRemove<K, T>> {
    fn clone(&self) -> Self {
        self.clone_trait()
    }
}