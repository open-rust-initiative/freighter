//!	Decode pack file by the `ObjDecodedMap`
use std::fmt::{Display, self};
use std::{collections::HashMap, rc::Rc};
use crate::git::Metadata;
use crate::git::hash::{Hash,HashType};

use crate::git::object::types::ObjectType;

use crate::git::ObjClass;
use super::super::{blob,commit,tag,tree};
use super::cache::PackObjectCache;
use colored::Colorize;
///!对取出的object字段进行进一步解码与包装
/// 用于存储解析出的object抽象对象的hashmap
#[derive(Default)]
pub struct ObjDecodedMap{
   pub _map_hash:HashMap<Hash,Rc<ObjClass>>
}//
//在解析完object后执行的进一步的解码过程
impl ObjDecodedMap {
    /// 通过cache对不同结构进行进一步解析
    #[allow(unused)]
    pub fn update_from_cache(&mut self, cache:& PackObjectCache) {
        for (key, value) in cache.by_hash.iter() {
            let metadata = 
                Metadata {
                    t: value.object_type ,  
                    h: HashType::Sha1,
                    id: value.hash(),
                    size: value.contents.len(),
                    data:value.contents.to_vec(),
                };
            
            let _obj:ObjClass=match value.object_type {// 交给各自的new函数,通过metadata来解码
                ObjectType::Blob => ObjClass::BLOB(blob::Blob::new(metadata)),
                ObjectType::Commit => ObjClass::COMMIT(commit::Commit::new(metadata) ),
                ObjectType::Tag => ObjClass::TAG(tag::Tag::new(metadata)),
                ObjectType::Tree =>  ObjClass::TREE(tree::Tree::new(metadata)),
            }; 
            self._map_hash.insert(key.clone(),Rc::new(_obj));
        }
        
    }

}

impl Display for ObjDecodedMap{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        
        for (key, value) in self._map_hash.iter(){
            writeln!(f,"*********************").unwrap();
            writeln!(f,"Hash: {}", key).unwrap();
            writeln!(f,"{}", value).unwrap();
        }
        writeln!(f,"{}",String::from("Finish Printf for ObjDecodedMap").blue())
    }
}
#[cfg(test)]
mod tests {
    use super::ObjDecodedMap;

    
    #[test]
    pub fn test_map_new(){
        let mut _map = ObjDecodedMap::default();
    }
}