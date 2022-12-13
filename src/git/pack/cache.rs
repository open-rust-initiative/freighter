//!	Build Cache Info for the decode packed object
use std::collections::BTreeMap;
use std::sync::Arc;
use super::super::hash::Hash;

use super::super::object::Object;

use std::rc::Rc;

/// #### Build Cache Info for the decode packed object
/// There are two hashmap for object ,<br>
/// the keys is `hash value` of The object 
#[derive(Default,Clone)]
pub struct PackObjectCache {
  pub by_hash: BTreeMap<Hash, Arc<Object>>,
  pub by_offset: BTreeMap<Hash,u64>,
}
// 
impl PackObjectCache{

  /// update cache by input object:`Rc<Object>` and the offset:`u64`
  pub fn update(&mut self, object: Arc<Object> , offset : u64 ){
    self.by_hash.insert(object.hash(), object.clone());
    self.by_offset.insert(object.hash(),offset);
  }
  #[allow(unused)]
  pub fn clean(&mut self){
    self.by_hash.clear();
    self.by_offset.clear();
  }
  
  pub fn offset_object(&mut self,offset :u64) -> Option<&mut Arc<Object>>{
    for (key,value) in self.by_offset.iter_mut() {
        if offset == *value{
          return self.by_hash.get_mut(key);
        }
    }
    None
  
  }
  
  pub fn hash_object(&mut self,hash :Hash) -> Option<&mut Arc<Object>>{
    self.by_hash.get_mut(&hash)
  }
}