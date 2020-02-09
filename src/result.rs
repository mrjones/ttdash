extern crate chrono;
extern crate prost;
extern crate reqwest;
extern crate rppal;
extern crate serde_json;
extern crate serde_xml_rs;
extern crate std;

pub type TTDashResult<T> = std::result::Result<T, TTDashError>;

#[derive(Debug)]
pub enum TTDashError {
    SimpleError(String),
    ChronoParseError(chrono::ParseError),
    GpioError(rppal::gpio::Error),
    HttpError(reqwest::Error),
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    XmlError(serde_xml_rs::Error),
    ProstDecodeError(prost::DecodeError),
    SpiError(rppal::spi::Error),
}

pub fn make_error(s: &str) -> TTDashError{
    return TTDashError::SimpleError(s.to_string());
}

impl std::fmt::Display for TTDashError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            TTDashError::SimpleError(ref s) => {
                return write!(f, "Simple Error: {}", s)
            }
            TTDashError::ChronoParseError(ref err) => {
                return write!(f, "Chrono Parse Error: {}", err);
            },
            TTDashError::GpioError(ref err) => {
                return write!(f, "GPIO Error: {}", err);
            },
            TTDashError::HttpError(ref err) => {
                return write!(f, "HTTP Error: {}", err);
            },
            TTDashError::IoError(ref err) => {
                return write!(f, "IO Error: {}", err);
            },
            TTDashError::JsonError(ref err) => {
                return write!(f, "JSON Error: {}", err);
            },
            TTDashError::XmlError(ref err) => {
                return write!(f, "XML Error: {}", err);
            },
            TTDashError::ProstDecodeError(ref err) => {
                return write!(f, "ProstDecodeError: {}", err);
            },
            TTDashError::SpiError(ref err) => {
                return write!(f, "SPI Error: {}", err);
            },
        }
    }
}

impl std::error::Error for TTDashError {
    fn description(&self) -> &str {
        match *self {
            TTDashError::SimpleError(_) => "SimpleError",
            TTDashError::ChronoParseError(_) => "ChronoParseError",
            TTDashError::GpioError(_) => "GpioError",
            TTDashError::HttpError(_) => "HttpError",
            TTDashError::IoError(_) => "IoError",
            TTDashError::JsonError(_) => "JsonError",
            TTDashError::XmlError(_) => "XmlError",
            TTDashError::ProstDecodeError(_) => "ProstDecodeError",
            TTDashError::SpiError(_) => "SpiError",
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        return None
    }
}

impl From<chrono::ParseError> for TTDashError {
    fn from(err: chrono::ParseError) -> TTDashError {
        return TTDashError::ChronoParseError(err);
    }
}

impl From<rppal::gpio::Error> for TTDashError {
    fn from(err: rppal::gpio::Error) -> TTDashError {
        return TTDashError::GpioError(err);
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

impl From<serde_json::Error> for TTDashError {
    fn from(err: serde_json::Error) -> TTDashError {
        return TTDashError::JsonError(err);
    }
}

impl From<serde_xml_rs::Error> for TTDashError {
    fn from(err: serde_xml_rs::Error) -> TTDashError {
        return TTDashError::XmlError(err);
    }
}

impl From<prost::DecodeError> for TTDashError {
    fn from(err: prost::DecodeError) -> TTDashError {
        return TTDashError::ProstDecodeError(err);
    }
}

impl From<rppal::spi::Error> for TTDashError {
    fn from(err: rppal::spi::Error) -> TTDashError {
        return TTDashError::SpiError(err);
    }
}
