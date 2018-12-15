//! Реализует простую сериализацию и десериализацию структур, наиболее близкую к их
//! представлению в памяти.
#![deny(missing_docs)]
extern crate serde;
extern crate byteorder;

#[cfg(test)]
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
#[macro_use]
extern crate quickcheck;

use byteorder::{BE, LE};

pub mod error;
pub mod ser;

/// Сериализатор, записывающий числа в поток в порядке `Big-Endian`
pub type BESerializer<W> = ser::Serializer<BE, W>;
/// Сериализатор, записывающий числа в поток в порядке `Little-Endian`
pub type LESerializer<W> = ser::Serializer<LE, W>;

pub use error::{Error, Result};
pub use ser::{to_vec, to_writer};
