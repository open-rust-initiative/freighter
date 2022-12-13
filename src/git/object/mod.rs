//!Object struct , contain the raw info cut from the pack file or other file
//! 

use types::ObjectType;
use super::hash::Hash;
use super::Metadata;
pub mod types;
pub mod delta;
//Object内存存储类型 
#[derive(Clone, Debug)]
pub struct Object {
  pub object_type: ObjectType,
  pub contents: Vec<u8>,
}
impl Object {
    /// object 的 hash转化函数
    pub fn hash(&self) -> Hash {
      Hash::from_obj(&self)
    }
   // pub fn GetObjectFromPack()
    pub fn to_metadata(&self) -> Metadata{
      Metadata{
        t: self.object_type,
        h: super::hash::HashType::Sha1,
        id: self.hash(),
        size: self.contents.len(),
        data: self.contents.clone(),
    }
    }
  }

//TODO: 测试object的hash生成
#[cfg(test)]
mod tests{
    use super::Object;

  #[test] 
  fn test_obj_hash(){
    let _obj=Object{
      object_type:super::types::ObjectType::Blob,
      contents : String::from("hellosss").into_bytes(),
    };
    print!("{}",_obj.hash())  ;//602091219933865cace5ab8cd78b424735c82e6c

  }
}