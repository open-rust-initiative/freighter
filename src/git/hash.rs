//!Hash值结构体 20位u8数组
//! > Attention to the Display function

use std::fmt::Display;
use crate::errors::GitError;
use std::convert::TryFrom;
use std::str::FromStr;
use sha1::{Digest, Sha1};

///Hash值的位数 - sha1
pub const HASH_BYTES: usize = 20;
const COMMIT_OBJECT_TYPE: &[u8] = b"commit";
const TREE_OBJECT_TYPE: &[u8] = b"tree";
const BLOB_OBJECT_TYPE: &[u8] = b"blob";
const TAG_OBJECT_TYPE: &[u8] = b"tag";

/// Git Object hash type. only support SHA1 for now.
#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HashType {
    Sha1,
}
/// Hash struct ,only contain the u8 array :`[u8;20]`
#[allow(unused)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,Default)]
pub struct Hash(pub [u8; HASH_BYTES]);

/// Display trait for Hash type
use colored::Colorize;
use super::object::{Object, types::ObjectType};
impl Display for Hash {
    // Hash 值打印的彩色与16进制格式
    /// # !Attention 
    /// cause of the color chars for ,if you want to use the string with out color ,
    /// please call the func:`to_plain_str()` rather than the func:`to_string()`
    /// ### For example :
    ///  the hash value `18fd2deaaf152c7f1222c52fb2673f6192b375f0`<br>
    ///  will be the `1;31m8d2deaaf152c7f1222c52fb2673f6192b375f00m`
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        //
        write!(f, "{}", self.to_plain_str().red().bold())
    }
}

impl Hash {
    /// Create Hash by the long information , the all data .
    pub fn new(data:&Vec<u8>) -> Hash{
        let mut new_hash = Sha1::new();
        new_hash.update(data);
        let hash_re = new_hash.finalize();
        let result = <[u8; 20]>::from(hash_re);
      Hash(result)
    }

    /// Create Hash from the Object 
    pub fn from_obj(obj:&Object) -> Hash{
        let mut h = Sha1::new();
        h.update(match obj.object_type {
          ObjectType::Commit => COMMIT_OBJECT_TYPE,
          ObjectType::Tree => TREE_OBJECT_TYPE,
          ObjectType::Blob => BLOB_OBJECT_TYPE,
          ObjectType::Tag => TAG_OBJECT_TYPE,
        });
        h.update(b" ");
        h.update(obj.contents.len().to_string());
        h.update(b"\0");
        h.update(&obj.contents);
        let hash_re = h.finalize();
        let result = <[u8; HASH_BYTES]>::from(hash_re);
      Hash(result)
    }
    
    ///解析出16进制数字0-f
    fn hex_char_value(hex_char: u8) -> Option<u8> {
        match hex_char {
            b'0'..=b'9' => Some(hex_char - b'0'),
            b'a'..=b'f' => Some(hex_char - b'a' + 10),
            b'A'..=b'F' => Some(hex_char - b'A' + 10),//Add The Support for the Big Char 
            _ => None ,
        }
    }
    
    ///Change the u8 array to the Hash ,which should be the 40 length,
    /// every bit is a char value of the string 
    pub fn from_bytes(hex_hash: &[u8]) -> Option<Hash> {
        const BITS_PER_CHAR: usize = 4;
        const CHARS_PER_BYTE: usize = 8 / BITS_PER_CHAR;
        // 将切片以chunks_size的切片
        let byte_chunks = hex_hash.chunks_exact(CHARS_PER_BYTE);
        if !byte_chunks.remainder().is_empty() {
            return None;
        }
        let bytes = byte_chunks
            .map(|hex_digits| {
                hex_digits.iter().try_fold(0, |value, &byte| {
                    let char_value = Hash::hex_char_value(byte)?;
                    Some(value << BITS_PER_CHAR | char_value)
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let bytes = <[u8; HASH_BYTES]>::try_from(bytes).ok()?;
        Some(Hash(bytes))
    }

    //Create a Hash value by the row value 
    // It's shout be a `&[u8;20]`
    pub fn from_row(hex_hash: &[u8]) -> Hash{
        Hash(<[u8; HASH_BYTES]>::try_from(hex_hash).unwrap())
    }
    // Get tht first u8 (0x00~0xff) from the Hash
    pub fn get_first(&self)->u8{
        return self.0[0];
    }
    /// Create plain String without the color chars 
    pub fn to_plain_str(&self) -> String{      
        hex::encode(self.0)
    }

    #[allow(unused)]
    pub(crate) fn to_folder(&self) -> String {
        
        let  str = self.to_plain_str();
        let str = str[0..2].to_string().clone();
        str
    }

    
    #[allow(unused)]
    pub(crate) fn to_filename(&self) -> String {
        
        let  str = self.to_plain_str();
        let str = str[2..].to_string().clone();
        str
    }


}

impl FromStr for Hash {
    type Err = GitError;
    fn from_str(hex_hash: &str) -> Result<Self, GitError> {
        Hash::from_bytes(hex_hash.as_bytes())
            .ok_or_else(|| GitError::InvalidHashValue(hex_hash.to_string()))
    }
}

mod tests {


   
    /// The Right Hash decode
    #[test]
    fn test_hash() {
        use super::Hash;
        use std::str::FromStr;
        let test_hash = Hash::from_str("18fd2deaaf152c7f1222c52fb2673f6192b375f0").unwrap();
        let result_hash: [u8; 20] = [
            24, 253, 45, 234, 175, 21, 44, 127, 18, 34, 197, 47, 178, 103, 63, 97, 146, 179, 117,
            240,
        ];
        assert_eq!(test_hash.0, result_hash);
        
        println!("{}",test_hash.to_string());
        println!("{}",test_hash.to_folder());
        assert_eq!(String::from("18"),test_hash.to_folder());
        assert_eq!(String::from("fd2deaaf152c7f1222c52fb2673f6192b375f0"),test_hash.to_filename());
    }


        /// The Right Hash decode
        #[test]
        fn test_hash_with_zero() {
            use super::Hash;
            use std::str::FromStr;
            let test_hash = Hash::from_str("08fd2deaaf152c7f1222c52fb2673f6192b37500").unwrap();
            let result_hash: [u8; 20] = [
                8, 253, 45, 234, 175, 21, 44, 127, 18, 34, 197, 47, 178, 103, 63, 97, 146, 179, 117, 0
            ];
             assert_eq!(test_hash.0, result_hash);
            println!("{}",test_hash);
        }
    /// The Wrong Hash decode
    #[test]
    fn test_error_hash() {
        use super::Hash;
        use std::str::FromStr;
        let test_str = "18fd2deaaf152c7f1222c52fb2673f6192z375f0";
        let test_hash = Hash::from_str(test_str).unwrap_err();
        print!("{:?}", test_hash);
        assert_eq!(
            format!("The {} is not a valid Hash value ", test_str),
            test_hash.to_string()
        );

    }
    #[test]
    fn test_btree_map(){
        use std::collections::BTreeMap;
        use super::Hash;
        use std::str::FromStr;
        let mut map = BTreeMap::new();
        map.insert(Hash::from_str("cd64b12b3949483d42d34979a3f89589aad804c2").unwrap(), 1);
        map.insert(Hash::from_str("1c6ec4271e3e75b585e8d150f9758e4ee4890dd5").unwrap(), 2);
        map.insert(Hash::from_str("f4010b9167a3c7d81bc81bfbffbeac0c9e95052f").unwrap(), 3);
        map.insert(Hash::from_str("aa36c1e0d709f96d7b356967e16766bafdf63a75").unwrap(), 4);
       for (key ,value) in map.iter() {
           println!("key: {} \t value :{}",key,value );
       }

    }

}
