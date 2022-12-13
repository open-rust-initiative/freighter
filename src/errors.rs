//!
//!
//!
//!
//!
//!
use thiserror::Error;
use log::info;

pub type FreightResult = Result<(), FreighterError>;

///
///
#[derive(Debug)]
pub struct FreighterError {
    pub error: Option<anyhow::Error>,
    pub code: i32,
}

/// The Freighter error is the error type used at Freight's CLI and others.
///
impl FreighterError {
    pub fn new(error: anyhow::Error, code: i32) -> FreighterError {
        FreighterError {
            error: Some(error),
            code,
        }
    }

    pub fn unknown_command(cmd: String) -> FreighterError {
        FreighterError {
            error: anyhow::anyhow!("Unknown command: {}", cmd).into(),
            code: 1,
        }
    }

    pub fn code(code: i32) -> FreighterError {
        FreighterError {
            error: None,
            code,
        }
    }

    pub fn print(&self) {
        info!("{}", self.error.as_ref().unwrap());
    }
}

///
///
impl From<anyhow::Error> for FreighterError {
    fn from(err: anyhow::Error) -> FreighterError {
        FreighterError::new(err, 101)
    }
}

///
///
impl From<clap::Error> for FreighterError {
    fn from(err: clap::Error) -> FreighterError {
        let code = i32::from(err.use_stderr());
        FreighterError::new(err.into(), code)
    }
}

///
///
impl From<std::io::Error> for FreighterError {
    fn from(err: std::io::Error) -> FreighterError {
        FreighterError::new(err.into(), 1)
    }
}

///
///
impl From<git2::Error> for FreighterError {
    fn from(err: git2::Error) -> FreighterError {
        FreighterError::new(err.into(), 1)
    }
}

///
///
impl From<walkdir::Error> for FreighterError {
    fn from(err: walkdir::Error) -> FreighterError {
        FreighterError::new(err.into(), 1)
    }
}

///
///
impl From<reqwest::Error> for FreighterError {
    fn from(err: reqwest::Error) -> FreighterError {
        FreighterError::new(err.into(), 1)
    }
}


#[derive(Error, Debug)]
pub enum GitError {
    #[error("The `{0}` is not a valid git object type.")]
    InvalidObjectType(String),

    #[error("The `{0}` is not a valid idx file.")]
    InvalidIdxFile(String),

    #[error("The `{0}` is not a valid pack file.")]
    InvalidPackFile(String),

    #[error("The `{0}` is not a valid pack header.")]
    InvalidPackHeader(String),

    #[error("The `{0}` is not a valid git tree type.")]
    InvalidTreeItem(String),

    #[error("The {0} is not a valid Hash value ")]
    InvalidHashValue(String),

    #[error("Delta Object Error Info:{0}")]
    DeltaObjError(String),

    #[error("Error decode in the Object ,info:{0}")]
    InvalidObjectInfo(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

}