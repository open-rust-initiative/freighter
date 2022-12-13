//!Encode and Decode The Pack File ,which is in the dir:`.git/object/pack/*.pack`
//! 
use std::convert::TryInto;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use self::cache::PackObjectCache;

use super::hash::Hash;
use super::idx::Idx;
use super::object::delta::*;
use super::object::Object;

use crate::errors::GitError;
use crate::git::pack::decode::ObjDecodedMap;
use crate::utils;
use std::convert::TryFrom;
use std::fs::File;
use std::rc::Rc;

mod cache;
mod diff;
pub mod decode;
pub mod encode;
//TODO:

// These two line can used to the idx write
//let mut first_byte_objects = [0u32; 1 << u8::BITS];
//first_byte_objects[_hash.0[0] as usize] += 1;

// A mind to simplify the hashmap
//let mut object_offsets = Vec::with_capacity(object_num as usize);
// object_offsets.push((_hash, offset));

/// ### Pack文件结构<br>
///  `head`: always = "PACK" <br>
/// `version`: version code <br>
/// `number_of_objects` : Total mount of objects <br>
/// `signature`:Hash <br>
/// `result`: decoded cache,
#[allow(unused)]
#[derive(Default)]
pub struct Pack {
    head: [u8; 4],
    version: u32,
    number_of_objects: usize,
    pub signature: Hash, 
    pub result: PackObjectCache,
}

impl Pack {
    /// Git [Pack Format](https://github.com/git/git/blob/master/Documentation/technical/pack-format.txt)
    /// Git Pack-Format [Introduce](https://git-scm.com/docs/pack-format)
    /// ## Decode the Pack File without the `.idx` File
    ///  - in: pack_file: &mut File
    ///  - out: The `Pack` Struct
    #[allow(unused)]
    pub fn decode(pack_file: &mut File) -> Result<Self, GitError> {
        // Check the Header of Pack File
        let mut _pack = Self::check_header(pack_file)?;

        // Init the cache for follow object parse
        let mut cache = PackObjectCache::default();

        for _ in 0.._pack.number_of_objects {
            //update offset of the Object
            let offset = utils::get_offset(pack_file).unwrap();
            //Get the next Object by the Pack::next_object() func
            let object = Pack::next_object(pack_file, offset, &mut cache).unwrap();
            // Larger offsets would require a version-2 pack index
            let offset = u32::try_from(offset)
                .map_err(|_| GitError::InvalidObjectInfo(format!("Packfile is too large")))
                .unwrap();
        }
        _pack.result = cache;
        // CheckSum sha-1
        let _id: [u8; 20] = utils::read_bytes(pack_file).unwrap();
        _pack.signature = Hash::from_row(&_id[..]);

        Ok(_pack)
    }

    /// Check the Header of the Pack File ,<br>
    /// include the **"PACK" head** , **Version Number** and  **Number of the Objects**
    fn check_header(pack_file: &mut File) -> Result<Self, GitError> {
        //init a Pack Struct ,which is all empty
        let mut _pack = Self {
            head: [0, 0, 0, 0],
            version: 0,
            number_of_objects: 0,
            signature: Hash::default(),
            result: PackObjectCache::default(),
        };

        // Get the Pack Head 4 b ,which should be the "PACK"
        let magic = utils::read_bytes(pack_file).unwrap();
        if magic != *b"PACK" {
            return Err(GitError::InvalidPackHeader(format!(
                "{},{},{},{}",
                magic[0], magic[1], magic[2], magic[3]
            )));
        }
        _pack.head = magic;

        //Get the Version Number
        let version = utils::read_u32(pack_file).unwrap();
        if version != 2 {
            return Err(GitError::InvalidPackFile(format!("Current File")));
        }
        _pack.version = version;

        let object_num = utils::read_u32(pack_file).unwrap();
        _pack.number_of_objects = object_num as usize;

        Ok(_pack)
    }

    #[allow(unused)]
    pub fn decode_by_idx(idx: &mut Idx, pack_file: &mut File) -> Result<Self, GitError> {
        let mut _pack = Self::check_header(pack_file)?;
        let object_num = idx.number_of_objects;
        _pack.number_of_objects = u32::try_from(object_num)
            .map_err(|_| GitError::InvalidObjectInfo(format!("Packfile is too large")))
            .unwrap() as usize;
        let mut cache = PackObjectCache::default();

        for idx_item in idx.idx_items.iter() {
            Pack::next_object(pack_file, idx_item.offset.try_into().unwrap(), &mut cache).unwrap();
        }
        let mut result = decode::ObjDecodedMap::default();
        result.update_from_cache(&mut cache);
        _pack.signature = idx.pack_signature.clone();
        Ok(_pack)
    }

    pub fn decode_raw_data(pack_file: &mut File) -> (Self,Vec<u8>){
        let mut  raw_pack = Self::check_header(pack_file).unwrap();
        let mut _raw:Vec<u8> =Vec::new();
        pack_file.read_to_end(&mut _raw).unwrap();
        let raw_info = _raw[.._raw.len()-20].to_vec();
        let _hash = Hash::from_row(&_raw[_raw.len()-20..]);
        raw_pack.signature = _hash;
        (raw_pack,raw_info)
    }
    /// Get the Object from File by the Give Offset<br>
    /// By the way , the cache can hold the fount object
    pub fn next_object(
        pack_file: &mut File,
        offset: u64,
        cache: &mut PackObjectCache,
    ) -> Result<Arc<Object>, GitError> {
        use super::object::types::PackObjectType::{self, *};
        utils::seek(pack_file, offset)?;
        let (object_type, size) = utils::read_type_and_size(pack_file)?;
        let object_types = PackObjectType::type_number2_type(object_type);

        //Get the Object according to the Types Enum
        let object = match object_types {
            // Undelta representation
            Some(Base(object_type)) => utils::read_zlib_stream_exact(pack_file, |decompressed| {
                let mut contents = Vec::with_capacity(size);
                decompressed.read_to_end(&mut contents)?;
                if contents.len() != size {
                    return Err(GitError::InvalidObjectInfo(format!(
                        "Incorrect object size"
                    )));
                }
                Ok(Object {
                    object_type,
                    contents,
                })
            }),
            // Delta; base object is at an offset in the same packfile
            Some(OffsetDelta) => {
                let delta_offset = utils::read_offset_encoding(pack_file)?;
                let base_offset = offset.checked_sub(delta_offset).ok_or_else(|| {
                    GitError::InvalidObjectInfo(format!("Invalid OffsetDelta offset"))
                })?;
                let offset = utils::get_offset(pack_file)?;
                let base_object = if let Some(object) = cache.offset_object(base_offset) {
                    Arc::clone(object)
                } else {
                    //递归调用 找出base object
                    Pack::next_object(pack_file, base_offset, cache)?
                };
                utils::seek(pack_file, offset)?;
                let objs = apply_delta(pack_file, &base_object)?;
                Ok(objs)
            }
            // Delta; base object is given by a hash outside the packfile
            //TODO : This Type need to be completed
            Some(HashDelta) => {
                let hash = utils::read_hash(pack_file)?;
                let object;
                let base_object = if let Some(object) = cache.hash_object(hash) {
                    object
                } else {
                    object = read_object(hash)?;
                    &object
                };
                apply_delta(pack_file, &base_object)
            }
            None => return Err(GitError::InvalidObjectType(object_type.to_string())),
        }?;

        // //Debug Code: Print the hash & type of the parsed object
        // match PackObjectType::type_number2_type(object_type) {
        //     Some(a) => println!("Hash:{} \t Types: {:?}",object.hash(), a),
        //     None =>{},
        // }

        let obj = Arc::new(object);
        cache.update(obj.clone(), offset);
        Ok(obj)
    }

    pub fn get_object_number(&self)-> usize{
        return self.number_of_objects as usize;
    }
    pub fn get_cache(&self) -> PackObjectCache{
        return self.result.clone();
    }
    pub fn get_hash(&self) -> Hash{
        return self.signature.clone() ;
    }
    
    /// Decode a pack file according to the given pack file path 
    /// # Examples 
    /// ```
    ///  let decoded_pack = Pack::decode_file("./resources/data/test/pack-6590ba86f4e863e1c2c985b046e1d2f1a78a0089.pack");
    ///  assert_eq!(
    ///    "6590ba86f4e863e1c2c985b046e1d2f1a78a0089",
    ///    decoded_pack.signature.to_plain_str()
    ///  );
    /// ```
    /// 
    #[allow(unused)]
    pub fn decode_file(file:&str)->Pack{
        let mut pack_file = File::open(&Path::new(
            file,
        ))
        .unwrap();
        let decoded_pack = match Pack::decode(&mut pack_file) {
            Ok(f) => f,
            Err(e) => panic!("{}", e.to_string()),
        };
        assert_eq!(*b"PACK", decoded_pack.head);
        assert_eq!(2, decoded_pack.version);
        let mut result = ObjDecodedMap::default();
        result.update_from_cache(&decoded_pack.result);
        print!("{}",result);
        decoded_pack
    }
}

///
#[cfg(test)]
mod tests {

    use crate::git::idx::Idx;
  
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    use std::path::Path;
    use super::Pack;

   
    /// Test the pack File decode standalone
    #[test]
    fn test_decode_pack_file1() {
        let decoded_pack = Pack::decode_file("./resources/data/test/pack-6590ba86f4e863e1c2c985b046e1d2f1a78a0089.pack");
        assert_eq!(
            "6590ba86f4e863e1c2c985b046e1d2f1a78a0089",
            decoded_pack.signature.to_plain_str()
        );
    }
    #[test]
    fn test_decode_pack_file_with_print() {
        let decoded_pack = Pack::decode_file("./pack-73bb49337b1b89f8d75a46be49ae16fa395f19f1.pack");
        assert_eq!(
            "73bb49337b1b89f8d75a46be49ae16fa395f19f1",
            decoded_pack.signature.to_plain_str()
        );
    }
    #[test]
    fn test_parse_simple_pack() {
        let decoded_pack = Pack::decode_file("./resources/test1/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack");
        assert_eq!(
            "1d0e6c14760c956c173ede71cb28f33d921e232f",
            decoded_pack.signature.to_plain_str()
        );
        print!("{}",decoded_pack.get_object_number());
    }
    //"./resources/test2/pack-8c81e90db37ef77494efe4f31daddad8b494e099.pack",


    #[test] 
    fn test_read_raw_pack(){
        let mut pack_file = File::open(&Path::new(
            "./resources/test1/pack-1d0e6c14760c956c173ede71cb28f33d921e232f.pack",
        ))
        .unwrap();
        let (raw_pack,raw_data) =  Pack::decode_raw_data(&mut pack_file);
        assert_eq!(
            "1d0e6c14760c956c173ede71cb28f33d921e232f",
            raw_pack.signature.to_plain_str()
        );
    }
    ///Test the pack decode by the Idx File
    #[test]
    fn test_pack_idx_decode() {
        let mut pack_file = File::open(&Path::new(
            "./resources/data/test/pack-8d36a6464e1f284e5e9d06683689ee751d4b2687.pack",
        ))
        .unwrap();
        let idx_file = File::open(&Path::new(
            "./resources/data/test/pack-8d36a6464e1f284e5e9d06683689ee751d4b2687.idx",
        ))
        .unwrap();
        let mut reader = BufReader::new(idx_file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).ok();

        let mut idx = Idx::default();
        idx.decode(buffer).unwrap();
        let decoded_pack = Pack::decode_by_idx(&mut idx, &mut pack_file).unwrap();
        assert_eq!(*b"PACK", decoded_pack.head);
        assert_eq!(2, decoded_pack.version);
        assert_eq!(
            "8d36a6464e1f284e5e9d06683689ee751d4b2687",
            decoded_pack.signature.to_plain_str()
        );
    }
}
