//! Содержит тип ошибки и результата, описывающие неуспешный результат сериализации
//! или десериализации.
use std::error;
use std::fmt;
use std::io;
use std::result;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use serde::{de, ser};

/// Варианты ошибок, которые могут возникнуть при сериализации или десериализации
#[derive(Debug)]
pub enum Error {
  /// Ошибка при записи сформированных байт в поток во время сериализации или при
  /// чтении из потока во время десериализации.
  Io(io::Error),
  /// Ошибка декодирования строки или символа из массива байт
  Encoding(Utf8Error),
  /// Ошибка сериализации стороннего типа
  Unknown(String),
  /// Метод десериализации не поддерживается
  Unsupported(&'static str),
}
/// Результат операции сериализации или десериализации
pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
  fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      Error::Io(ref err) => err.fmt(fmt),
      Error::Encoding(ref err) => err.fmt(fmt),
      Error::Unknown(ref msg) => msg.fmt(fmt),
      Error::Unsupported(ref msg) => msg.fmt(fmt),
    }
  }
}

impl error::Error for Error {
  fn description(&self) -> &str {
    match *self {
      Error::Io(ref err) => error::Error::description(err),
      Error::Encoding(ref err) => error::Error::description(err),
      Error::Unknown(ref msg) => msg,
      Error::Unsupported(ref msg) => msg,
    }
  }

  fn cause(&self) -> Option<&error::Error> {
    match *self {
      Error::Io(ref err) => Some(err),
      Error::Encoding(ref err) => Some(err),
      Error::Unknown(_) => None,
      Error::Unsupported(_) => None,
    }
  }
}
// Конвертация из ошибок сериализации сторонних типов
impl ser::Error for Error {
  fn custom<T: fmt::Display>(msg: T) -> Self {
    Error::Unknown(msg.to_string())
  }
}
// Конвертация из ошибок десериализации сторонних типов
impl de::Error for Error {
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
// Конвертация из ошибок, связанных с кодированием строк в UTF-8
impl From<Utf8Error> for Error {
  fn from(err: Utf8Error) -> Self {
    Error::Encoding(err)
  }
}
impl From<FromUtf8Error> for Error {
  fn from(err: FromUtf8Error) -> Self {
    Error::Encoding(err.utf8_error())
  }
}
