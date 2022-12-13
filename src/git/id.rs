//!  # Deprecated 
//! It was used to the Hash value
//! ID .


use std::fmt::Display;

use crypto::digest::Digest;
use crypto::sha1::Sha1;

use crate::git::{NL, SPACE};

use super::object::types::ObjectType;

/// Git Object ID: a SHA-1 hash for now, and we will support multiple hash algorithms later.
/// The SHA-1 Hax ID is a 40-byte hexadecimal string.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone,Default)]
pub struct ID {
    pub bytes: Vec<u8>,
    pub hash: String,
}

///
impl ID {
    /// Return the first and second alphanumeric characters of the ID.
    /// In the git object store format, the first two characters is the folder for save the object.
    #[allow(unused)]
    pub(crate) fn to_folder(&self) -> String { //to_folder
        self.hash.as_str()[0..2].to_string()
    }

    /// Return the last 18 characters of the ID for the object name.
    #[allow(unused)]
    pub(crate) fn to_filename(&self) -> String {//TODO 
        self.hash.as_str()[2..].to_string()
    }

    /// Return the ID in the git object store format form a hex string.
    #[allow(unused)]
    pub(crate) fn from_string(s: &str) -> Self {//from_str
        ID {
            bytes: hex::decode(s).unwrap(),
            hash: s.to_string(),
        }
    }

    /// Return the ID in the git object store format from a byte array.
    #[allow(unused)]
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {//hex_to_hash
        ID {
            bytes: bytes.to_vec(),
            hash: hex::encode(bytes),
        }
    }

    #[allow(unused)]
    pub(crate) fn from_vec(t: ObjectType, data: &mut [u8]) -> Self {//new
        let mut hash = Sha1::new();

        let object: &[u8] = &[
            t.to_string().as_bytes(),
            SPACE,
            data.len().to_string().as_bytes(),
            NL,
            (data),
        ].concat();

        hash.input(object);
        let mut id = [0u8; 20];
        hash.result(&mut id);

        ID::from_bytes(id.as_ref())
    }
}

/// Display ObjectID hash data to hex string
impl Display for ID {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.hash)
    }
}

///
#[cfg(test)]
mod tests {
    use std::env;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    use std::path::PathBuf;

    use bstr::ByteSlice;

    use super::ID;

    /// There is a bug need to be resolve:
    ///     The `\r\n` is a Windows Style, but the `\n` is a POSIX Style.
    ///     The file will be different both length and content between Windows and Mac.
    ///     So there is different SHA-1 value.
    ///
    ///     Temporarily, just replace the `\r\n` to `\n` in the test.
    ///
    ///     Same as the another test case: [test_blob_write_to_file]
    ///
    ///     References:
    ///         [1] https://docs.github.com/cn/get-started/getting-started-with-git/configuring-git-to-handle-line-endings
    ///
    #[test]
    fn test_object_id_new() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/data/test/gitmega.md");

        let f = File::open(path).ok();
        let mut reader = BufReader::new(f.unwrap());
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).ok();

        if env::consts::OS == "windows" {
            buffer = buffer.replace(b"\r\n", b"\n");
        }

        let id = ID::from_vec(super::ObjectType::Blob, &mut buffer);
        assert_eq!("82352c3a6a7a8bd32011751699c7a3648d1b5d3c", id.to_string());
    }

}