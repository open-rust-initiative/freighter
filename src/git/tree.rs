//!	Tree 对象结构体
//!
//!
//!
//!

use std::fmt::Display;

use bstr::ByteSlice;

use crate::errors::GitError;
use crate::git::Metadata;
use crate::git::object::types::ObjectType;
use crate::git::hash::HashType;
use crate::git::Hash;
///
#[derive(PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Clone, Copy)]
pub enum TreeItemType {
    Blob,
    BlobExecutable,
    Tree,
    Commit,
    Link,
}

use colored::Colorize;

use super::object::Object;
impl Display for TreeItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _print = match *self {
            TreeItemType::Blob => "blob",
            TreeItemType::BlobExecutable => "blob executable",
            TreeItemType::Tree => "tree",
            TreeItemType::Commit => "commit",
            TreeItemType::Link => "link",
        };
        write!(f, "{}",String::from(_print).blue())
    }
}

///
impl TreeItemType {
    ///
    #[allow(unused)]
    pub(crate) fn to_bytes(self) -> &'static [u8] {
        match self {
            TreeItemType::Blob => b"100644",
            TreeItemType::BlobExecutable => b"100755",
            TreeItemType::Tree => b"40000",
            TreeItemType::Link => b"120000",
            TreeItemType::Commit => b"160000",
        }
    }

    ///
    #[allow(unused)]
    pub(crate) fn tree_item_type_from(mode: &[u8]) -> Result<TreeItemType, GitError> {
        Ok(match mode {
            b"40000" => TreeItemType::Tree,
            b"100644" => TreeItemType::Blob,
            b"100755" => TreeItemType::BlobExecutable,
            b"120000" => TreeItemType::Link,
            b"160000" => TreeItemType::Commit,
            b"100664" => TreeItemType::Blob,
            b"100640" => TreeItemType::Blob,
            _ => return Err(GitError::InvalidTreeItem(String::from_utf8(mode.to_vec()).unwrap())),
        })
    }
}

/// Git Object: tree item
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct TreeItem {
    pub mode: Vec<u8>,
    pub item_type: TreeItemType,
    pub id: Hash,
    pub filename: String,
}

/// Git Object: tree
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Tree {
    pub meta: Metadata,
    pub tree_items: Vec<TreeItem>,
}


impl Display for Tree {
    #[allow(unused)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f,"Type: Tree");
        for item in &self.tree_items {
            writeln!(f, "{:6} {} {} {}",
                     String::from_utf8(item.mode.to_vec()).unwrap(),
                     item.item_type, item.id, item.filename);
        }

        Ok(())
    }
}

///
impl Tree {

    pub fn new (metadata:Metadata)-> Self {
        let mut  a = Self{
            meta:metadata,
            tree_items:vec![],
        };
        a.decode_metadata().unwrap();
        a
    }

    pub(crate) fn decode_metadata(&mut self) -> Result<(), GitError> {
        let mut index = 0;
        while index < self.meta.data.len() {
            let mode_index = &self.meta.data[index..].find_byte(0x20).unwrap();
            let mode = &self.meta.data[index..index + *mode_index];
            let item_type = TreeItemType::tree_item_type_from(mode).unwrap();

            let filename_index = &self.meta.data[index..].find_byte(0x00).unwrap();
            let filename = String::from_utf8(self.meta.data[index + mode_index + 1.. index + *filename_index]
                .to_vec())
                .unwrap();

            let id = Hash::from_row(&self.meta.data[index + filename_index + 1..index + filename_index + 21].to_vec());

            self.tree_items.push(TreeItem {
                mode: mode.to_vec(),
                item_type,
                id,
                filename,
            });

            index = index + filename_index + 21;
        }

        Ok(())
    }

    ///
    #[allow(unused)]
    pub(crate) fn encode_metadata(&self) -> Result<Metadata, ()> {
        let mut data = Vec::new();
        for item in &self.tree_items {
            data.extend_from_slice(&item.mode);
            data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
            data.extend_from_slice(item.filename.as_bytes());
            data.extend_from_slice(0x00u8.to_be_bytes().as_ref());
            data.extend_from_slice(&item.id.0.to_vec());
        }

        Ok(
            Metadata {
                t: ObjectType::Tree,
                h: HashType::Sha1,
                id:Object{
                    object_type: ObjectType::Tree,
                    contents:data.clone()
                }.hash(),
                size: data.len(),
                data,
            },
        )
    }

    ///
    #[allow(unused)]
    pub(crate) fn write_to_file(&self, root_path: String) -> Result<String, GitError> {
        self.meta.write_to_file(root_path)
    }
}

///
#[cfg(test)]
mod tests {
    use std::env;
    use std::path::Path;
    use std::path::PathBuf;

    use super::ObjectType;
    use crate::git::Metadata;
    use crate::git::blob::Blob;
    use crate::git::hash::Hash;
    use crate::git::hash::HashType;

    use super::Tree;
    use super::TreeItemType;

    ///
    #[test]
    fn test_tree_write_to_file() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/blob-82352c3a6a7a8bd32011751699c7a3648d1b5d3c-gitmega.md");

        let meta =
            Metadata::read_object_from_file(path.to_str().unwrap().to_string())
                .expect("Read error!");

        assert_eq!(meta.t, ObjectType::Blob);
        assert_eq!("82352c3a6a7a8bd32011751699c7a3648d1b5d3c", meta.id.to_plain_str());
        assert_eq!(16, meta.size);

        let blob = Blob {
            meta: meta.clone(),
            data: meta.data,
        };

        assert_eq!(
            "# Hello Gitmega\n",
            String::from_utf8(blob.clone().data).unwrap().as_str()
        );

        let item = blob
            .to_tree_item(String::from("gitmega.md")).unwrap();

        let mut tree = Tree {
            meta: Metadata {
                t: ObjectType::Tree,
                h: HashType::Sha1,
                id: Hash::default(),
                size: 0,
                data: vec![]
            },
            tree_items: vec![item],
        };

        tree.meta = tree.encode_metadata().unwrap();
        tree.write_to_file("/tmp".to_string()).expect("Write error!");

        assert!(Path::new("/tmp/1b/dbc1e723aa199e83e33ecf1bb19f874a56ebc3").exists());
    }

    ///
    #[test]
    fn test_tree_write_to_file_2_blob() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/blob-fc1a505ac94f98cc5f29100a2d9aef97027a32fb-gitmega.md");

        let meta_gitmega =
            Metadata::read_object_from_file(path.to_str().unwrap().to_string())
                .expect("Read error!");

        let blob_gitmega = Blob {
            meta: meta_gitmega.clone(),
            data: meta_gitmega.data,
        };

        let item_gitmega = blob_gitmega
            .to_tree_item(String::from("gitmega.md")).unwrap();

        path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/blob-a3b55a2ce16d2429dae2d690d2c15bcf26fbe33c-gust.md");

        let meta_gust =
            Metadata::read_object_from_file(path.to_str().unwrap().to_string())
                .expect("Read error!");

        let blob_gust = Blob {
            meta: meta_gust.clone(),
            data: meta_gust.data,
        };

        let item_gust = blob_gust
            .to_tree_item(String::from("gust.md")).unwrap();


        let mut tree = Tree {
            meta: Metadata {
                t: ObjectType::Tree,
                h: HashType::Sha1,
                id: Hash::default(),
                size: 0,
                data: vec![]
            },
            tree_items: vec![item_gitmega, item_gust],
        };

        tree.meta = tree.encode_metadata().unwrap();
        tree.write_to_file("/tmp".to_string()).expect("Write error!");

        assert!(Path::new("/tmp/9b/be4087bedef91e50dc0c1a930c1d3e86fd5f20").exists());
    }

    ///
    #[test]
    fn test_tree_read_from_file() {
        // 100644 blob 82352c3a6a7a8bd32011751699c7a3648d1b5d3c	gitmega.md
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/tree-1bdbc1e723aa199e83e33ecf1bb19f874a56ebc3");

        let meta = Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        assert_eq!(ObjectType::Tree, meta.t);
        assert_eq!(38, meta.size);

        let mut tree = Tree {
            meta,
            tree_items: Vec::new(),
        };

        tree.decode_metadata().unwrap();

        assert_eq!(1, tree.tree_items.len());
        assert_eq!(
            "gitmega.md",
            tree.tree_items[0].filename.as_str()
        );
        assert_eq!(
            "82352c3a6a7a8bd32011751699c7a3648d1b5d3c",
            tree.tree_items[0].id.to_plain_str()
        );
        assert_eq!(
            "100644",
            String::from_utf8(tree.tree_items[0].mode.to_vec()).unwrap().as_str()
        );
        assert_eq!(TreeItemType::Blob, tree.tree_items[0].item_type);
    }

    ///
    #[test]
    fn test_tree_read_from_file_2_items() {
        // 100644 blob fc1a505ac94f98cc5f29100a2d9aef97027a32fb	gitmega.md
        // 100644 blob a3b55a2ce16d2429dae2d690d2c15bcf26fbe33c	gust.md
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/tree-9bbe4087bedef91e50dc0c1a930c1d3e86fd5f20");

        let meta = Metadata::read_object_from_file(path.to_str().unwrap().to_string())
            .expect("Read error!");

        assert_eq!(ObjectType::Tree, meta.t);
        assert_eq!(73, meta.size);

        let mut tree = Tree {
            meta,
            tree_items: Vec::new(),
        };

        tree.decode_metadata().unwrap();

        assert_eq!(2, tree.tree_items.len());

        assert_eq!(
            "gitmega.md",
            tree.tree_items[0].filename.as_str()
        );

        assert_eq!(
            "fc1a505ac94f98cc5f29100a2d9aef97027a32fb",
            tree.tree_items[0].id.to_plain_str()
        );

        assert_eq!(
            "100644",
            String::from_utf8(tree.tree_items[0].mode.to_vec()).unwrap().as_str()
        );

        assert_eq!(TreeItemType::Blob, tree.tree_items[0].item_type);

        assert_eq!(
            "gust.md",
            tree.tree_items[1].filename.as_str()
        );

        assert_eq!(
            "a3b55a2ce16d2429dae2d690d2c15bcf26fbe33c",
            tree.tree_items[1].id.to_plain_str()
        );

        assert_eq!(
            "100644",
            String::from_utf8(tree.tree_items[1].mode.to_vec()).unwrap().as_str()
        );

        assert_eq!(TreeItemType::Blob, tree.tree_items[1].item_type);
    }
}