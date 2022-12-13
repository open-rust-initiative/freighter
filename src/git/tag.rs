//!	Tag  对象结构体
//!
//!
//!


use bstr:: ByteSlice;
use crate::errors::GitError;
use crate::git::hash::{HashType,Hash};
use crate::git::object::types::ObjectType;
use crate::git::sign::AuthorSign;
use crate::git::Metadata;
use std::fmt::{Display};
use super::object::Object;

/// Git Object: tag
#[allow(unused)]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Tag {
    pub meta: Metadata,
    pub object: Hash,
    pub t: ObjectType,
    pub tag: String,
    pub tagger: AuthorSign,
    pub message: String,
}

///
impl Tag {
    /// Tag 的构造函数 接收一个@param meta::Metadata
    /// 同时执行tag解码 -> `fn decode_metadata`
    pub fn new(meta: Metadata) -> Self {
        
        let mut a = Self {
            meta: meta.clone(),
            object: meta.id.clone(),
            t: ObjectType::Commit,
            tag: "".to_string(),
            tagger: AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "".to_string(),
            },
            message: "".to_string(),
        };
        a.decode_metadata().unwrap();
        a
    }

    ///
    #[allow(unused)]
    fn decode_metadata(&mut self) -> Result<(), GitError> {
        let mut data = self.meta.data.clone();

        let object_begin = data.find_byte(0x20).unwrap();
        let object_end = data.find_byte(0x0a).unwrap();
        self.object = Hash::from_bytes(&data[object_begin + 1..object_end].to_vec()).unwrap();
        data = data[object_end + 1..].to_vec();

        let type_begin = data.find_byte(0x20).unwrap();
        let type_end = data.find_byte(0x0a).unwrap();
        self.t = ObjectType::from_string(data[type_begin + 1..type_end].to_str().unwrap()).unwrap();
        data = data[type_end + 1..].to_vec();

        let tag_begin = data.find_byte(0x20).unwrap();
        let tag_end = data.find_byte(0x0a).unwrap();
        self.tag = data[tag_begin + 1..tag_end]
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        data = data[tag_end + 1 ..].to_vec();//Fixed Bug: bug type_end to tag_end 
       

        let tagger_begin = data.find("tagger").unwrap();
        let tagger_end = data.find_byte(0x0a).unwrap();
        let tagger_data = data[tagger_begin..tagger_end].to_vec();
        self.tagger.decode_from_data(tagger_data)?;
        data = data[data.find_byte(0x0a).unwrap() + 1..].to_vec();

        self.message = data[data.find_byte(0x0a).unwrap()..]
            .to_vec()
            .to_str()
            .unwrap()
            .to_string();

        Ok(())
    }

    ///
    #[allow(unused)]
    fn encode_metadata(&self) -> Result<Metadata, ()> {
        let mut data = Vec::new();

        data.extend_from_slice("object".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.object.to_plain_str().as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice("type".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.t.to_string().as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice("tag".as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.tag.as_bytes());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());

        data.extend_from_slice(self.tagger.encode_to_data().unwrap().as_ref());
        data.extend_from_slice(0x0au8.to_be_bytes().as_ref());
        data.extend_from_slice(self.message.as_bytes());

        Ok(Metadata {
            t: ObjectType::Tag,
            h: HashType::Sha1,
            
            id: Object{
                object_type: ObjectType::Tag,
                contents:data.clone()
            }.hash(),
        
            size: data.len(),
            data,
        })
    }

    ///
    #[allow(unused)]
    fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
        self.meta.write_to_file(root_path)
    }
}
impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Type: Tag").unwrap();
        writeln!(f, "Tag : {}", self.tag).unwrap();
        self.tagger.fmt(f).unwrap();
        writeln!(f,"{}",self.message)
    }
}
///
#[cfg(test)]
mod tests {
    use std::env;
    use std::path::Path;
    use std::path::PathBuf;
    use std::str::FromStr;
    use crate::git::hash::Hash;
    use crate::git::sign::AuthorSign;
    use crate::git::Metadata;
    use crate::git::ObjectType;

    use super::Tag;

    ///
    #[test]
    fn test_tag_read_from_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/tag-e5c324b03b72b26f11557c4955c6d17c68dc8595");

        let meta = Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        assert_eq!(ObjectType::Tag, meta.t);
        assert_eq!(976, meta.size);
        assert_eq!(
            "e5c324b03b72b26f11557c4955c6d17c68dc8595",
            meta.id.to_plain_str()
        );

        let mut tag = Tag {
            meta,
            object: Hash::default(),
            t: ObjectType::Commit,
            tag: "".to_string(),
            tagger: AuthorSign {
                t: "".to_string(),
                name: "".to_string(),
                email: "".to_string(),
                timestamp: 0,
                timezone: "+0000".to_string(),
            },
            message: "".to_string(),
        };

        tag.decode_metadata().unwrap();
        
        assert_eq!(
            "6414e45babf0bdd043ba40d31123053cfebef26c",
            tag.object.to_plain_str()
        );
        assert_eq!("commit", tag.t.to_string());
        assert_eq!("v1.1.0", tag.tag);
        assert_eq!(1653037847, tag.tagger.timestamp);
        println!("{}",tag);
    }

    #[test]
    fn test_output_meat() {
        let meta = Metadata {
            t: ObjectType::Tag,
            h: crate::git::HashType::Sha1,
            id: Hash::from_str("df1087c478c8d337cb587b897e86f2455e2687ed").unwrap() ,
            size: 155,
            data: vec![
                111, 98, 106, 101, 99, 116, 32, 51, 55, 50, 49, 51, 101, 55, 98, 98, 51, 99, 51,
                51, 52, 97, 48, 102, 55, 55, 48, 56, 99, 55, 97, 102, 99, 97, 98, 53, 98, 97, 98,
                98, 51, 102, 57, 53, 52, 51, 52, 10, 116, 121, 112, 101, 32, 99, 111, 109, 109,
                105, 116, 10, 116, 97, 103, 32, 48, 46, 49, 10, 116, 97, 103, 103, 101, 114, 32,
                97, 100, 105, 116, 121, 97, 32, 60, 100, 101, 118, 64, 99, 104, 105, 109, 101, 114,
                97, 99, 111, 100, 101, 114, 46, 110, 101, 116, 62, 32, 49, 52, 50, 56, 54, 49, 50,
                48, 48, 55, 32, 45, 48, 52, 48, 48, 10, 10, 70, 105, 114, 115, 116, 32, 105, 109,
                112, 108, 101, 109, 101, 110, 116, 97, 116, 105, 111, 110, 32, 111, 102, 32, 116,
                104, 101, 32, 99, 108, 105, 10,
            ],
        };

        let tag = Tag::new(meta);
        
        
        println!("{}", tag);
    }
    ///
    #[test]
    fn test_tag_write_to_file() {
        use super::HashType;
        let meta = Metadata {
            t: ObjectType::Tag,
            h: HashType::Sha1,
            size: 0,
            id: Hash::default(),
            data: vec![],
        };

        let tagger = AuthorSign {
            t: "tagger".to_string(),
            name: "Quanyi Ma".to_string(),
            email: "eli@patch.sh".to_string(),
            timestamp: 1653037847,
            timezone: "+0800".to_string(),
        };

        let mut tag = Tag {
            meta,
            object: Hash::from_str("6414e45babf0bdd043ba40d31123053cfebef26c").unwrap(),
            t: ObjectType::Commit,
            tag: "v1.1.0".to_string(),
            tagger,
            message: "\nIt's a lastest object\n-----BEGIN PGP SIGNATURE-----\n\niQIzBAABCAAdFiEEanuf5/5ADLU2lvsCZL9E4tsHuXIFAmKHWxcACgkQZL9E4tsH\nuXIeFhAAtX+foSvc7/1lb98+QfRjHcpO+LX+LroTaq/QGOTX/2gE+tHD2TJAga1I\nVqDEz8fh8AE366FC7UCjCb5nvsCCox2htzbIxAjsc9L/JckWtxl6WOa/5OZssrDQ\nFtX39BNNl+4TfNn/z1XV+28c9yB1N5HSoP2gzdLoASw3y9n6E0FyzLdoXPILgmJI\nL4DAG/OFkixK+I+TsK+6995497h9BCi3x30dOjfxZS9ptiKhqWulbkflvvM9Cnie\n7obXYmnoe0jBjSfO5GgJlOYcLzE9MMYYzIx47/4lcrCbQXnojkW3KV03PEXGfRCL\nw/y8oBHVvNVRF0Jn+o7F+mzIrbF6Ufku63MfRf7WmbbS3B63CILEjNyuOFoe8mDb\nrmAUffzQSrgnvBk+g01slb6Q+q7Urw6wqHtBPn3ums/inHE9ymTqS7ffmRifUfR8\nD8LvhwpSUI7BdiN6HznRFPxMXzohYIqAJbUltjr4Q7qw/kJI+305Xcs1U5AUIaOp\n77p2UFHRVoMM5mpPOCSwsVJ6cSuOjWXf9afcNMrhgclKefM0aXXnd2p5zTUEe99T\nlAtXHuprRwxtSQUzHxJCdGlUGRGRR2aS9W984SNDVmcegnOIrZD2pVm/tjDwVex5\nMuAuKHr8et1EKyvKCnta6USq7WC2l6RdsCaAYzSTQ7ljEi9A+6Q=\n=/9g0\n-----END PGP SIGNATURE-----\n".to_string(),
        };

        tag.meta = tag.encode_metadata().unwrap();
        assert_eq!(
            "e5c324b03b72b26f11557c4955c6d17c68dc8595",
            tag.meta.id.to_plain_str()
        );

        tag.write_to_file("/tmp".to_string()).expect("Write error!");
        assert!(Path::new("/tmp/e5/c324b03b72b26f11557c4955c6d17c68dc8595").exists());
    }
}
