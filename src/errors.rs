//!
//!
//!
//!
//!
//!

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
        FreighterError { error: None, code }
    }

    pub fn print(&self) {
        tracing::info!("{}", self.error.as_ref().unwrap());
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
