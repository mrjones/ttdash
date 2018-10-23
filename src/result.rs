extern crate protobuf;
extern crate reqwest;
extern crate std;

pub type TTDashResult<T> = std::result::Result<T, TTDashError>;

#[derive(Debug)]
pub enum TTDashError {
    GenericError(String),
    HttpError(reqwest::Error),
    IoError(std::io::Error),
    ProtobufError(protobuf::ProtobufError),
}

impl std::fmt::Display for TTDashError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            TTDashError::HttpError(ref err) => {
                return write!(f, "HTTP Error: {}", err);
            },
            TTDashError::IoError(ref err) => {
                return write!(f, "IO Error: {}", err);
            },
            TTDashError::ProtobufError(ref err) => {
                return write!(f, "Protobuf Error: {}", err);
            },
            TTDashError::GenericError(ref e) => std::fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for TTDashError {
    fn description(&self) -> &str {
        match *self {
            TTDashError::HttpError(_) => "HttpError",
            TTDashError::IoError(_) => "IoError",
            TTDashError::ProtobufError(_) => "ProtobufError",
            TTDashError::GenericError(ref str) => str,
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        return None
    }
}

impl From<reqwest::Error> for TTDashError {
    fn from(err: reqwest::Error) -> TTDashError {
        return TTDashError::HttpError(err);
    }
}

impl From<std::io::Error> for TTDashError {
    fn from(err: std::io::Error) -> TTDashError {
        return TTDashError::IoError(err);
    }
}

impl From<protobuf::ProtobufError> for TTDashError {
    fn from(err: protobuf::ProtobufError) -> TTDashError {
        return TTDashError::ProtobufError(err);
    }
}
