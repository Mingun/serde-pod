//! Реализует простую сериализацию и десериализацию структур, наиболее близкую к их
//! представлению в памяти.
//!
//! # Пример
//! Читаем заголовок GFF файла (формат Bioware, используемый для хранения данных в
//! таких играх, как Neverwinter Nights, Neverwinter Nights 2 и Ведьмак):
//! ```rust
//! # extern crate byteorder;
//! # #[macro_use]
//! # extern crate serde_derive;
//! # extern crate serde_pod;
//! # use serde_pod::{from_bytes, Result};
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Signature([u8; 4]);
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Version([u8; 4]);
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct Section {
//!   offset: u32,
//!   count: u32,
//! }
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct GffHeader {
//!   signature:     Signature,
//!   version:       Version,
//!   structs:       Section,
//!   fields:        Section,
//!   labels:        Section,
//!   field_data:    Section,
//!   field_indices: Section,
//!   list_indices:  Section,
//! }
//!
//! # fn main() -> Result<()> {
//! let header: GffHeader = from_bytes::<byteorder::LE, _>(&[
//!   // Signature
//!   0x47, 0x55, 0x49, 0x20,
//!   // Version
//!   0x56, 0x33, 0x2E, 0x32,
//!   // structs
//!   0x38, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00,
//!   // fields
//!   0xEC, 0x00, 0x00, 0x00, 0x93, 0x00, 0x00, 0x00,
//!   // labels
//!   0xD0, 0x07, 0x00, 0x00, 0x1A, 0x00, 0x00, 0x00,
//!   // field_data
//!   0x70, 0x09, 0x00, 0x00, 0x1D, 0x02, 0x00, 0x00,
//!   // field_indices
//!   0x8D, 0x0B, 0x00, 0x00, 0x4C, 0x02, 0x00, 0x00,
//!   // list_indices
//!   0xD9, 0x0D, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00,
//! ])?;
//!
//! assert_eq!(header, GffHeader {
//!   signature:     Signature(*b"GUI "),
//!   version:       Version(*b"V3.2"),
//!   structs:       Section { offset:   0x38, count:  15 },
//!   fields:        Section { offset:   0xEC, count: 147 },
//!   labels:        Section { offset: 0x07D0, count:  26 },
//!   field_data:    Section { offset: 0x0970, count: 541 },
//!   field_indices: Section { offset: 0x0B8D, count: 588 },
//!   list_indices:  Section { offset: 0x0DD9, count:  36 },
//! });
//! # Ok(())
//! # }
//! ```
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
pub mod de;

/// Сериализатор, записывающий числа в поток в порядке `Big-Endian`
pub type BESerializer<W> = ser::Serializer<BE, W>;
/// Сериализатор, записывающий числа в поток в порядке `Little-Endian`
pub type LESerializer<W> = ser::Serializer<LE, W>;

/// Десериализатор, читающий числа из потока в порядке `Big-Endian`
pub type BEDeserializer<R> = de::Deserializer<BE, R>;
/// Десериализатор, читающий числа из потока в порядке `Little-Endian`
pub type LEDeserializer<R> = de::Deserializer<LE, R>;

pub use error::{Error, Result};
pub use ser::{to_vec, to_writer};
pub use de::from_bytes;
