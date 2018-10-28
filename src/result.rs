extern crate protobuf;
extern crate reqwest;
extern crate rppal;
extern crate std;

pub type TTDashResult<T> = std::result::Result<T, TTDashError>;

#[derive(Debug)]
pub enum TTDashError {
    GpioError(rppal::gpio::Error),
    HttpError(reqwest::Error),
    IoError(std::io::Error),
    ProtobufError(protobuf::ProtobufError),
    SpiError(rppal::spi::Error),
}

impl std::fmt::Display for TTDashError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            TTDashError::GpioError(ref err) => {
                return write!(f, "GPIO Error: {}", err);
            },
            TTDashError::HttpError(ref err) => {
                return write!(f, "HTTP Error: {}", err);
            },
            TTDashError::IoError(ref err) => {
                return write!(f, "IO Error: {}", err);
            },
            TTDashError::ProtobufError(ref err) => {
                return write!(f, "Protobuf Error: {}", err);
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
            TTDashError::GpioError(_) => "GpioError",
            TTDashError::HttpError(_) => "HttpError",
            TTDashError::IoError(_) => "IoError",
            TTDashError::ProtobufError(_) => "ProtobufError",
            TTDashError::SpiError(_) => "SpiError",
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        return None
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

impl From<protobuf::ProtobufError> for TTDashError {
    fn from(err: protobuf::ProtobufError) -> TTDashError {
        return TTDashError::ProtobufError(err);
    }
}

impl From<rppal::spi::Error> for TTDashError {
    fn from(err: rppal::spi::Error) -> TTDashError {
        return TTDashError::SpiError(err);
    }
}
