//!
//!
//!
const TYPE_BITS: u8 = 3;
const VAR_INT_ENCODING_BITS: u8 = 7;
const TYPE_BYTE_SIZE_BITS: u8 = VAR_INT_ENCODING_BITS - TYPE_BITS;
const VAR_INT_CONTINUE_FLAG: u8 = 1 << VAR_INT_ENCODING_BITS;


use std::{io::{self,Read,SeekFrom,Seek}, fs::File, vec};
use flate2::read::ZlibDecoder;

use crate::errors::GitError;

use super::git::hash::Hash;
///保留value二进制的后bits位
fn keep_bits(value: usize, bits: u8) -> usize {
    value & ((1 << bits) - 1)
}

///从read中读取出N个u8
pub fn read_bytes<R: Read, const N: usize>(stream: &mut R) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    stream.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// 从Read中读取出一个u32
pub fn read_u32<R: Read>(stream: &mut R) -> io::Result<u32> {
    let bytes = read_bytes(stream)?;
    Ok(u32::from_be_bytes(bytes))
}

/// 从Read中读取一个hash
pub fn read_hash<R: Read>(stream: &mut R) -> io::Result<Hash> {
    let bytes = read_bytes(stream)?;
    Ok(Hash(bytes))
}
/// 读取vec直到读到delimiter
pub fn read_until_delimiter<R: Read>(stream: &mut R, delimiter: u8) -> io::Result<Vec<u8>> {
    let mut bytes = vec![];
    loop {
        let [byte] = read_bytes(stream)?;
        if byte == delimiter {
            break;
        }

        bytes.push(byte);
    }
    Ok(bytes)
}
/// 返回u8的第一位是否为1 并返回7位真值
pub fn read_var_int_byte<R: Read>(stream: &mut R) -> io::Result<(u8, bool)> {
    let [byte] = read_bytes(stream)?;
    let value = byte & !VAR_INT_CONTINUE_FLAG;
    let more_bytes = byte & VAR_INT_CONTINUE_FLAG != 0;
    Ok((value, more_bytes))
}
/// 读取信息位
pub fn read_size_encoding<R: Read>(stream: &mut R) -> io::Result<usize> {
    let mut value = 0;
    let mut length = 0;
    loop {
        let (byte_value, more_bytes) = read_var_int_byte(stream)?;
        value |= (byte_value as usize) << length;
        if !more_bytes {
            return Ok(value);
        }

        length += VAR_INT_ENCODING_BITS;
    }
}
///读取Object的前几个字段并解析出
pub fn read_type_and_size<R: Read>(stream: &mut R) -> io::Result<(u8, usize)> {
    // Object type and uncompressed pack data size
    // are stored in a "size-encoding" variable-length integer.
    // Bits 4 through 6 store the type and the remaining bits store the size.
    let value = read_size_encoding(stream)?;
    let object_type = keep_bits(value >> TYPE_BYTE_SIZE_BITS, TYPE_BITS) as u8;
    let size = keep_bits(value, TYPE_BYTE_SIZE_BITS)
        | (value >> VAR_INT_ENCODING_BITS << TYPE_BYTE_SIZE_BITS);
    Ok((object_type, size))
}
///The offset for an OffsetDelta object
pub fn read_offset_encoding<R: Read>(stream: &mut R) -> io::Result<u64> {
    // Like the object length, the offset for an OffsetDelta object
    // is stored in a variable number of bytes,
    // with the most significant bit of each byte indicating whether more bytes follow.
    // However, the object length encoding allows redundant values,
    // e.g. the 7-bit value [n] is the same as the 14- or 21-bit values [n, 0] or [n, 0, 0].
    // Instead, the offset encoding adds 1 to the value of each byte except the least significant one.
    // And just for kicks, the bytes are ordered from *most* to *least* significant.
    let mut value = 0;
    loop {
        let (byte_value, more_bytes) = read_var_int_byte(stream)?;
        value = (value << VAR_INT_ENCODING_BITS) | byte_value as u64;
        if !more_bytes {
            return Ok(value);
        }

        value += 1;
    }
}

pub fn read_partial_int<R: Read>(
    stream: &mut R,
    bytes: u8,
    present_bytes: &mut u8,
) -> io::Result<usize> {
    let mut value:usize = 0;
    for byte_index in 0..bytes {
        if *present_bytes & 1 != 0 {
            let [byte] = read_bytes(stream)?;
            value |= (byte as usize) << (byte_index * 8);
        }
        *present_bytes >>= 1;
    }
    Ok(value)
}

/// 返回文件偏移后的指针
pub fn seek(file: &mut File, offset: u64) -> io::Result<()> {
    file.seek(SeekFrom::Start(offset))?;
    Ok(())
}
/// 探测目前offset
pub fn get_offset(file: &mut File) -> io::Result<u64> {
    file.seek(SeekFrom::Current(0))
}

pub fn get_singnature(file: &mut File) -> io::Result<u64> {
    file.seek(SeekFrom::End(0))
}

// Call reader() to process a zlib stream from a file.
// Reset the file offset afterwards to the end of the zlib stream,
// since ZlibDecoder uses BufReader, which may consume extra bytes.
pub fn read_zlib_stream_exact<T, F>(file: &mut File, reader: F) -> Result<T,GitError>
  where F: FnOnce(&mut ZlibDecoder<&mut File>) -> Result<T,GitError>
{

  let offset = get_offset(file)?;
  let mut decompressed = ZlibDecoder::new(file);
  let result = reader(&mut decompressed);
  let zlib_end = offset + decompressed.total_in();
  seek(decompressed.into_inner(), zlib_end).unwrap();
  result
}

pub fn u32_vec(value: u32)->Vec<u8>{
    let mut result :Vec<u8> = vec![];
    result.push((value >> 24 & 0xff) as u8  );
    result.push((value >> 16 & 0xff) as u8  );
    result.push((value >> 8  & 0xff) as u8  );
    result.push((value       & 0xff) as u8  );
    result
}

pub fn get_pack_raw_data(data:Vec<u8>) -> Vec<u8>{

    let result = &data[12..data.len()-20];
    let result =  result.to_vec();
    result
}