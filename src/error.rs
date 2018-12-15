//! Содержит тип ошибки и результата, описывающие неуспешный результат сериализации
//! или десериализации.
use std::error;
use std::fmt;
use std::io;
use std::result;
use serde::ser;

/// Варианты ошибок, которые могут возникнуть при сериализации или десериализации
#[derive(Debug)]
pub enum Error {
  /// Ошибка при записи сформированных байт в поток во время сериализации или при
  /// чтении из потока во время десериализации.
  Io(io::Error),
  /// Ошибка сериализации стороннего типа
  Unknown(String),
}
/// Результат операции сериализации или десериализации
pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
  fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      Error::Io(ref err) => err.fmt(fmt),
      Error::Unknown(ref msg) => msg.fmt(fmt),
    }
  }
}

impl error::Error for Error {
  fn description(&self) -> &str {
    match *self {
      Error::Io(ref err) => error::Error::description(err),
      Error::Unknown(ref msg) => msg,
    }
  }

  fn cause(&self) -> Option<&error::Error> {
    match *self {
      Error::Io(ref err) => Some(err),
      Error::Unknown(_) => None,
    }
  }
}
// Конвертация из ошибок сериализации сторонних типов
impl ser::Error for Error {
  fn custom<T: fmt::Display>(msg: T) -> Self {
    Error::Unknown(msg.to_string())
  }
}
// Конвертация из ошибок, связанных с чтением/записью из потока
impl From<io::Error> for Error {
  fn from(err: io::Error) -> Self {
    Error::Io(err)
  }
}
