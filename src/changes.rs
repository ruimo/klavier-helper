use std::any::Any;

pub trait Changes<K, T> where T: Clone {
    fn len(&self) -> usize;
    fn iter(&self) -> Box<dyn Iterator<Item = ((K, T), (K, T))> + '_>;
//    fn clone_trait(&self) -> Box<dyn Changes<K, T>>;
//    fn as_any(&self) -> &dyn Any; // For down casting
}

//impl<K, T: Clone> Clone for Box<dyn Changes<K, T>> {
//    fn clone(&self) -> Self {
//        self.clone_trait()
//    }
//}
