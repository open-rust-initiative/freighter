//!	Sign 签名 对象结构体
//!
//!
//!
//!
//!
//!

use std::fmt::Display;

use bstr::ByteSlice;

use crate::errors::GitError;

///
#[allow(unused)]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct AuthorSign {
    pub t: String,
    pub name: String,
    pub email: String,
    pub timestamp: usize,
    pub timezone: String,
}

///
impl Display for AuthorSign {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} \n Email:<{}> \n timestamp:{}\n timezone:{}", self.name, self.email, self.timestamp, self.timezone)
    }
}

///
impl AuthorSign {
    ///
    #[allow(unused)]
    pub(crate) fn decode_from_data(&mut self, data: Vec<u8>) -> Result<(), GitError> {
        let mut data = data;

        let name_start = data.find_byte(0x20).unwrap();

        self.t = String::from_utf8(data[..name_start].to_vec()).unwrap();

        let email_start = data.find_byte(0x3C).unwrap();
        let email_end = data.find_byte(0x3E).unwrap();

        self.name = data[name_start + 1..email_start - 1].to_str().unwrap().to_string();
        self.email = data[email_start + 1..email_end].to_str().unwrap().to_string();
        data = data[email_end + 2..].to_vec();

        let timestamp_split = data.find_byte(0x20).unwrap();
        self.timestamp = data[0..timestamp_split].to_str().unwrap().parse::<usize>().unwrap();
        self.timezone = data[timestamp_split + 1..].to_str().unwrap().to_string();

        Ok(())
    }

    ///
    #[allow(unused)]
    pub(crate) fn encode_to_data(&self) -> Result<Vec<u8>, GitError> {
        let mut data = Vec::new();

        data.extend_from_slice(self.t.as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.name.as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(0x3Cu8.to_be_bytes().as_ref());
        data.extend_from_slice(self.email.as_bytes());
        data.extend_from_slice(0x3Eu8.to_be_bytes().as_ref());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.timestamp.to_string().as_bytes());
        data.extend_from_slice(0x20u8.to_be_bytes().as_ref());
        data.extend_from_slice(self.timezone.as_bytes());

        Ok(data)
    }
}

mod tests {
    #[test]
    fn test_author_sign_encode() {
        let author = super::AuthorSign {
            t: "author".to_string(),
            name: "Quanyi Ma".to_string(),
            email: "eli@patch.sh".to_string(),
            timestamp: 1649521615,
            timezone: "+0800".to_string()
        };

        let data = author.encode_to_data().unwrap();

        let author_data = [97, 117, 116, 104, 111, 114, 32, 81, 117, 97, 110, 121, 105, 32, 77, 97, 32, 60, 101, 108, 105, 64, 112, 97, 116, 99, 104, 46, 115, 104, 62, 32, 49, 54, 52, 57, 53, 50, 49, 54, 49, 53, 32, 43, 48, 56, 48, 48]
            .to_vec();

        assert_eq!(data, author_data);
    }

    #[test]
    fn test_author_sign_decode() {
        let author_data = [97, 117, 116, 104, 111, 114, 32, 81, 117, 97, 110, 121, 105, 32, 77, 97, 32, 60, 101, 108, 105, 64, 112, 97, 116, 99, 104, 46, 115, 104, 62, 32, 49, 54, 52, 57, 53, 50, 49, 54, 49, 53, 32, 43, 48, 56, 48, 48]
            .to_vec();

        let mut author = super::AuthorSign {
            t: "".to_string(),
            name: "".to_string(),
            email: "".to_string(),
            timestamp: 0,
            timezone: "".to_string()
        };

        author.decode_from_data(author_data).unwrap();

        assert_eq!(author.t, "author");
        assert_eq!(author.name, "Quanyi Ma");
        assert_eq!(author.email, "eli@patch.sh");
        assert_eq!(author.timestamp, 1649521615);
        assert_eq!(author.timezone, "+0800");
    }
}