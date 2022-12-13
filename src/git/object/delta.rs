use crate::errors::GitError;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::Path;
use std::str::FromStr;
use flate2::read::ZlibDecoder;
use super::Hash;
use super::Object;
use crate::utils;

const INDEX_FILE_SUFFIX: &str = ".idx";
const COPY_INSTRUCTION_FLAG: u8 = 1 << 7;
const COPY_OFFSET_BYTES: u8 = 4;
const COPY_SIZE_BYTES: u8 = 3;
const COPY_ZERO_SIZE: usize = 0x10000;

///使用delta指令
pub fn apply_delta(pack_file: &mut File, base: &Object) -> Result<Object, GitError> {
    let Object {
        object_type,
        contents: ref base,
    } = *base;
    utils::read_zlib_stream_exact(pack_file, |delta| {
        let base_size = utils::read_size_encoding(delta)?;
        if base.len() != base_size {
            return Err(GitError::DeltaObjError(
                String::from_str("Incorrect base object length").unwrap(),
            ));
        }

        let result_size = utils::read_size_encoding(delta)?;
        let mut result = Vec::with_capacity(result_size);
        while apply_delta_instruction(delta, base, &mut result)? {}
        if result.len() != result_size {
            return Err(GitError::DeltaObjError(
                String::from_str("Incorrect object length").unwrap(),
            ));
        }

        // The object type is the same as the base object
        Ok(Object {
            object_type,
            contents: result,
        })
    })
}

///执行单个delta指令
fn apply_delta_instruction<R: Read>(
    stream: &mut R,
    base: &[u8],
    result: &mut Vec<u8>,
) -> Result<bool, GitError> {
    // Check if the stream has ended, meaning the new object is done
    let instruction = match utils::read_bytes(stream) {
        Ok([instruction]) => instruction,
        Err(err) if err.kind() == ErrorKind::UnexpectedEof => return Ok(false),
        Err(err) => {
            return Err(GitError::DeltaObjError(format!(
                "Wrong instruction in delta :{}",err.to_string()
            )))
        }
    };
    if instruction & COPY_INSTRUCTION_FLAG == 0 {
        // Data instruction; the instruction byte specifies the number of data bytes
        if instruction == 0 {
            // Appending 0 bytes doesn't make sense, so git disallows it
            return Err(GitError::DeltaObjError(
                String::from_str("Invalid data instruction").unwrap(),
            ));
        }

        // Append the provided bytes
        let mut data = vec![0; instruction as usize];
        stream.read_exact(&mut data)?;
        result.extend_from_slice(&data);
    } else {
        // Copy instruction
        let mut nonzero_bytes = instruction;
        let offset = utils::read_partial_int(stream, COPY_OFFSET_BYTES, &mut nonzero_bytes)?;
        let mut size = utils::read_partial_int(stream, COPY_SIZE_BYTES, &mut nonzero_bytes)?;
        if size == 0 {
            // Copying 0 bytes doesn't make sense, so git assumes a different size
            size = COPY_ZERO_SIZE;
        }
        // Copy bytes from the base object
        let base_data = base
            .get(offset..(offset + size))
            .ok_or_else(|| GitError::DeltaObjError(format!("Invalid copy instruction")));

        match base_data {
            Ok(data) => result.extend_from_slice(data),
            Err(e) => return Err(e),
          }
        }
        
    Ok(true)
}

pub fn read_object(hash: Hash) -> Result<Object, GitError> {
    let object = match read_unpacked_object(hash) {
        // Found in objects directory
        Ok(object) => object,
        // Not found in objects directory; look in packfiles
        Err(_err) => panic!("not found object"),
    };

    let object_hash = object.hash();
    if object_hash != hash {
        return Err(GitError::DeltaObjError(format!(
            "Object {} has wrong hash {}",
            hash, object_hash
        )));
    }

    Ok(object)
}

const OBJECTS_DIRECTORY: &str = ".git/objects";

///读出unpack 的Object
#[allow(unused)]
fn read_unpacked_object(hash: Hash) -> Result<Object, GitError> {
    use super::ObjectType::*;

    let hex_hash = hash.to_string();
    let (directory_name, file_name) = hex_hash.split_at(2);
    let object_file = Path::new(OBJECTS_DIRECTORY)
        .join(directory_name)
        .join(file_name);
    let object_file = File::open(object_file)?;
    let mut object_stream = ZlibDecoder::new(object_file);
    let object_type = utils::read_until_delimiter(&mut object_stream, b' ')?;
    let object_type = match &object_type[..] {
        _commit_object_type => Commit,
        _tree_object_type => Tree,
        _blob_object_type => Blob,
        _tag_object_type => Tag,
        _ => {
            return Err(GitError::DeltaObjError(format!(
                "Invalid object type: {:?}",
                object_type
            )))
        }
    };
    let size = utils::read_until_delimiter(&mut object_stream, b'\0')?;
    let size = match parse_decimal(&size){
      Some(a) => a,
      None => return Err(GitError::DeltaObjError(format!("Invalid object size: {:?}", size)))
    };

    
    let mut contents = Vec::with_capacity(size);
    object_stream.read_to_end(&mut contents)?;
    if contents.len() != size {
        return Err(GitError::DeltaObjError(format!("Incorrect object size")));
    }

    Ok(Object {
        object_type,
        contents,
    })
}

///解析u8数组的十进制
fn parse_decimal(decimal_str: &[u8]) -> Option<usize> {
    let mut value = 0usize;
    for &decimal_char in decimal_str {
        let char_value = decimal_char_value(decimal_char)?;
        value = value.checked_mul(10)?;
        value = value.checked_add(char_value as usize)?;
    }
    Some(value)
}

///从u8转为单个10进制数
fn decimal_char_value(decimal_char: u8) -> Option<u8> {
    match decimal_char {
        b'0'..=b'9' => Some(decimal_char - b'0'),
        _ => None,
    }
}

///获取idx文件的文件名
#[allow(unused)]
fn strip_index_file_name(file_name: &OsStr) -> Option<&str> {
    let file_name = file_name.to_str()?;
    file_name.strip_suffix(INDEX_FILE_SUFFIX)
}
