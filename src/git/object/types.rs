//! ### Types enums for object types
//! There are ObjectType
//! PackObjectType
//! 
//! 
use std:: fmt::Display;
use crate::errors::GitError;
/// Four abstract Object Types:
/// - Blob
/// - Tree
/// - Commit 
/// - Tag
#[derive(PartialEq, Eq, Hash, Ord, PartialOrd, Debug, Clone, Copy)]
pub enum ObjectType {
    Commit,
    Tree,
    Blob,
    Tag,
}


/// Display trait for Git objects type
impl Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ObjectType::Blob => write!(f, "blob"),
            ObjectType::Tree => write!(f, "tree"),
            ObjectType::Commit => write!(f, "commit"),
            ObjectType::Tag => write!(f, "tag"),
        }
    }
}

///
impl ObjectType {
    ///
    #[allow(unused)]
    pub fn to_bytes(self) -> Vec<u8> {
        match self {
            ObjectType::Blob => vec![0x62, 0x6c, 0x6f, 0x62],
            ObjectType::Tree => vec![0x74, 0x72, 0x65, 0x65],
            ObjectType::Commit => vec![0x63, 0x6f, 0x6d, 0x6d, 0x69, 0x74],
            ObjectType::Tag => vec![0x74, 0x61, 0x67],
        }
    }

    ///
    #[allow(unused)]
    pub fn from_string(s: &str) -> Result<ObjectType, GitError> {
        match s {
            "blob" => Ok(ObjectType::Blob),
            "tree" => Ok(ObjectType::Tree),
            "commit" => Ok(ObjectType::Commit),
            "tag" => Ok(ObjectType::Tag),
            _ => Err(GitError::InvalidObjectType(s.to_string())),
        }
    }
    pub fn type2_number(&self)->u8{
        match self {
            ObjectType::Commit=> 1,
            ObjectType::Tree=> 2,
            ObjectType::Blob=> 3,
            ObjectType::Tag=> 4,
        }
    }
}


///Six Storage Object Type in the Pack File
/// - Blob(1)
/// - Tree(2)
/// - Commit(3) 
/// - Tag(4)
/// -  OffsetDelta(6)
/// -  HashDelta(7)
#[derive(Debug)]
pub enum PackObjectType {
    Base(ObjectType),
    OffsetDelta,
    HashDelta,
}
impl PackObjectType {
    pub fn type_number2_type(type_number: u8) -> Option<Self> {
        use ObjectType::*;
        match type_number {
            1 => Some(Self::Base(Commit)),
            2 => Some(Self::Base(Tree)),
            3 => Some(Self::Base(Blob)),
            4 => Some(Self::Base(Tag)),
            6 => Some(Self::OffsetDelta),
            7 => Some(Self::HashDelta),
            _ => None,
        }
    }
    #[allow(unused)]
    pub fn type2_number(&self) -> u8{
        use ObjectType::*;
        match self {
            Self::Base(Commit) => 1,
            Self::Base(Tree) => 2,
            Self::Base(Blob) => 3,
            Self::Base(Tag) => 4,
            Self::OffsetDelta => 6,
            Self::HashDelta => 7,
        }
    }

}



