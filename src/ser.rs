//! Содержит тип, реализующий простую сериализацию данных, как POD типов.

use std::io::Write;
use std::marker::PhantomData;
use byteorder::{ByteOrder, WriteBytesExt};
use serde::ser::{self, Serialize};

use error::{Error, Result};

/// Структура для сериализации значений Rust в простой поток байт. Внедрение разделителей
/// и любой другой метаинформации для десериализации остается заботой вызывающего кода.
///
/// # Правила сериализации
/// Все типы сериализуются максимально близко к тому, как они лежат в памяти: все целые
/// типы записываются согласно их разрядности (отраженной в названии типа), используя порядок
/// байт `BO`.
///
/// Для структур и кортежей рекурсивно сериализуются их поля, без разделителей между ними.
/// Если такие разделители требуются, они должны быть внедрены непосредственно в структуру
/// или кортеж.
///
/// Тип [`()`], [`None`]-вариант [`Option`], а также unit-вариант перечисления никак не записываются
/// в поток, писатель должен самостоятельно позаботится о сохранении информации об их наличии.
/// Например, для записи С-like перечислений он может использовать вместо enum-поля в структуре
/// поле одного из примитивных типов.
///
/// `bool`-значения сериализуются, как 1 байт со значением `0` или `1`.
///
/// [Newtype] типы сериализуются, как оборачиваемое ими значение. При необходимости сохранить маркер
/// типа вызывающий код должен сделать это самостоятельно, например, сериализуя вместо [Newtype][doc]
/// типа структуру с двумя полями -- маркером типа и значением.
///
/// Сериализация [строковых срезов][str] выполняется записью в поток UTF-8 кодированного значения,
/// которая является нативной для Rust и таким образом ведет за собой нулевые накладные расходы на
/// сериализацию. Записываются только байты самой строки, нулевого байта или длины строки никуда не
/// добавляется. В случае, если требуется записывать строки в других кодировках, оберните их в
/// структуры, для которых будет реализован типаж [`Serialize`], выполняющий сохранение данных в
/// требуемой кодировке, например, с помощью крейта [encoding].
///
/// Отдельные символы записываются, как строки из одного символа, в UTF-8. Также как и для строк, нулевой
/// байт в конце символа не записывается.
///
/// Сериализация последовательностей и их срезов осуществляется простой последовательной сериализацией
/// их элементов. Ни количество, ни разделители между элементами, ни какой-либо маркер конца
/// последовательности не записываются. В случае, если они требуются для корректной десериализации,
/// они должны быть добавлены в сериализуемые структуры вручную.
///
/// Key-value типы сериализуются, как последовательность структур ключ-значение по уже описанным выше
/// правилам. Порядок таких пар определяется сериализуемой структурой.
///
/// # Параметры типа
/// - `BO`: определяет порядок байт, в котором будут записаны примитивные числовые типы:
///         `u16`, `u32`, `u64`, `u128`, `i16`, `i32`, `i64`, `i128`, `f32` и `f64`.
/// - `W`: определяет тип, обеспечивающих сохранение сериализуемых данных в хранилище
///
/// [`()`]: https://doc.rust-lang.org/std/primitive.unit.html
/// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
/// [Newtype]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
/// [doc]: https://doc.rust-lang.org/book/ch19-04-advanced-types.html#using-the-newtype-pattern-for-type-safety-and-abstraction
/// [str]: https://doc.rust-lang.org/std/primitive.str.html
/// [`Serialize`]: https://docs.serde.rs/serde/trait.Serialize.html
/// [encoding]: https://docs.rs/encoding/
pub struct Serializer<BO, W> {
  /// Приемник сериализованных данных
  writer: W,
  /// Порядок байт, используемый при записи чисел
  _byteorder: PhantomData<BO>,
}

impl<BO, W> Serializer<BO, W>
  where W: Write,
        BO: ByteOrder,
{
  /// Создает сериализатор с настройками по умолчанию. Строки кодируются в UTF-8,
  /// если встречается непредставимый символ, кодирование прерывается и возвращается ошибка
  ///
  /// # Параметры
  /// - `writer`: Поток, в который записывать сериализуемые данные
  ///
  /// # Возвращаемое значение
  /// Сериализатор для записи данных в указанный поток и кодированием строк в UTF-8
  pub fn new(writer: W) -> Self {
    Serializer { writer, _byteorder: PhantomData }
  }
}

impl<'a, BO, W> ser::Serializer for &'a mut Serializer<BO, W>
  where W: Write,
        BO: ByteOrder,
{
  type Ok = ();
  type Error = Error;

  type SerializeSeq = Self;
  type SerializeTuple = Self;
  type SerializeTupleStruct = Self;
  type SerializeTupleVariant = Self;
  type SerializeMap = Self;
  type SerializeStruct = Self;
  type SerializeStructVariant = Self;

  /// Записывает в выходной поток 1 байт
  fn serialize_i8 (self, v: i8 ) -> Result<Self::Ok> { self.writer.write_i8(v).map_err(Into::into) }
  /// Записывает в выходной поток 1 байт
  fn serialize_u8 (self, v: u8 ) -> Result<Self::Ok> { self.writer.write_u8(v).map_err(Into::into) }
  /// Записывает в выходной поток 2 байта в указанном в сериализаторе порядке байт
  fn serialize_i16(self, v: i16) -> Result<Self::Ok> { self.writer.write_i16::<BO>(v).map_err(Into::into) }
  /// Записывает в выходной поток 2 байта в указанном в сериализаторе порядке байт
  fn serialize_u16(self, v: u16) -> Result<Self::Ok> { self.writer.write_u16::<BO>(v).map_err(Into::into) }
  /// Записывает в выходной поток 4 байта в указанном в сериализаторе порядке байт
  fn serialize_i32(self, v: i32) -> Result<Self::Ok> { self.writer.write_i32::<BO>(v).map_err(Into::into) }
  /// Записывает в выходной поток 4 байта в указанном в сериализаторе порядке байт
  fn serialize_u32(self, v: u32) -> Result<Self::Ok> { self.writer.write_u32::<BO>(v).map_err(Into::into) }
  /// Записывает в выходной поток 8 байт в указанном в сериализаторе порядке байт
  fn serialize_i64(self, v: i64) -> Result<Self::Ok> { self.writer.write_i64::<BO>(v).map_err(Into::into) }
  /// Записывает в выходной поток 8 байт в указанном в сериализаторе порядке байт
  fn serialize_u64(self, v: u64) -> Result<Self::Ok> { self.writer.write_u64::<BO>(v).map_err(Into::into) }
  /// Записывает в выходной поток 16 байт в указанном в сериализаторе порядке байт
  fn serialize_i128(self, v: i128) -> Result<Self::Ok> { self.writer.write_i128::<BO>(v).map_err(Into::into) }
  /// Записывает в выходной поток 16 байт в указанном в сериализаторе порядке байт
  fn serialize_u128(self, v: u128) -> Result<Self::Ok> { self.writer.write_u128::<BO>(v).map_err(Into::into) }
  /// Записывает в выходной поток 4 байта в указанном в сериализаторе порядке байт
  fn serialize_f32(self, v: f32) -> Result<Self::Ok> { self.writer.write_f32::<BO>(v).map_err(Into::into) }
  /// Записывает в выходной поток 8 байт в указанном в сериализаторе порядке байт
  fn serialize_f64(self, v: f64) -> Result<Self::Ok> { self.writer.write_f64::<BO>(v).map_err(Into::into) }

  /// Записывает в выходной поток 1 байт: `0x00` для `false` и `0x01` для `true`
  fn serialize_bool(self, v: bool) -> Result<Self::Ok> { self.serialize_u8(if v { 1 } else { 0 }) }
  /// Записывает в выходной поток UTF-8 байты представления указанного символа
  #[inline]
  fn serialize_char(self, v: char) -> Result<Self::Ok> {
    let mut buf = [0u8; 4];// Символ в UTF-8 может занимать максимум 4 байта
    self.serialize_str(v.encode_utf8(&mut buf))
  }

  /// Записывает в выходной поток UTF-8 байты представления указанной строки
  #[inline]
  fn serialize_str(self, v: &str) -> Result<Self::Ok> {
    self.serialize_bytes(v.as_bytes())
  }
  /// Записывает в выходной поток байты указанного массива как есть
  fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> { self.writer.write_all(v).map_err(Into::into) }

  /// Ничего не записывает в поток
  fn serialize_none(self) -> Result<Self::Ok> { Ok(()) }
  /// Записывает в выходной поток представление `value` с помощью данного сериализатора
  fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where T: ?Sized + Serialize,
  {
    value.serialize(self)
  }
  /// Ничего не записывает в поток
  fn serialize_unit(self) -> Result<Self::Ok> { Ok(()) }
  /// Ничего не записывает в поток
  fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> { Ok(()) }
  /// Ничего не записывает в поток
  fn serialize_unit_variant(
    self, _name: &'static str, _variant_index: u32, _variant: &'static str
  ) -> Result<Self::Ok> { Ok(()) }

  /// Записывает в выходной поток представление `value` с помощью данного сериализатора
  fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where T: ?Sized + Serialize,
  {
    value.serialize(self)
  }
  /// Записывает в выходной поток представление `value` с помощью данного сериализатора.
  /// Остальные параметры игнорируются
  fn serialize_newtype_variant<T>(
    self, _name: &'static str, _variant_index: u32, _variant: &'static str, value: &T
  ) -> Result<Self::Ok>
    where T: ?Sized + Serialize,
  {
    value.serialize(self)
  }

//-------------------------------------------------------------------------------------------------
  /// Просто возвращает данный сериализатор. Параметр `_len` игнорируется
  fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> { Ok(self) }
  /// Просто возвращает данный сериализатор. Параметр `_len` игнорируется
  fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> { Ok(self) }
  /// Просто возвращает данный сериализатор. Все параметры игнорируются
  fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct> { Ok(self) }
  /// Просто возвращает данный сериализатор. Все параметры игнорируются
  fn serialize_tuple_variant(
    self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize
  ) -> Result<Self::SerializeTupleVariant> { Ok(self) }
  /// Просто возвращает данный сериализатор. Параметр `_len` игнорируется
  fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> { Ok(self) }
  /// Просто возвращает данный сериализатор. Все параметры игнорируются
  fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> { Ok(self) }
  /// Просто возвращает данный сериализатор. Все параметры игнорируются
  fn serialize_struct_variant(
    self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize
  ) -> Result<Self::SerializeStructVariant> { Ok(self) }

  /// Возвращает `false`
  fn is_human_readable(&self) -> bool { false }
}

impl<'a, BO, W> ser::SerializeSeq for &'a mut Serializer<BO, W>
  where W: Write,
        BO: ByteOrder,
{
  type Ok = ();
  type Error = Error;

  /// Записывает в выходной поток представление `value` с помощью данного сериализатора
  fn serialize_element<T>(&mut self, value: &T) -> Result<Self::Ok>
    where T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }
  /// Ничего не записывает в поток
  fn end(self) -> Result<Self::Ok> { Ok(()) }
}

impl<'a, BO, W> ser::SerializeTuple for &'a mut Serializer<BO, W>
  where W: Write,
        BO: ByteOrder,
{
  type Ok = ();
  type Error = Error;

  /// Записывает в выходной поток представление `value` с помощью данного сериализатора
  fn serialize_element<T>(&mut self, value: &T) -> Result<Self::Ok>
    where T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }
  /// Ничего не записывает в поток
  fn end(self) -> Result<Self::Ok> { Ok(()) }
}

impl<'a, BO, W> ser::SerializeTupleStruct for &'a mut Serializer<BO, W>
  where W: Write,
        BO: ByteOrder,
{
  type Ok = ();
  type Error = Error;

  /// Записывает в выходной поток представление `value` с помощью данного сериализатора
  fn serialize_field<T>(&mut self, value: &T) -> Result<Self::Ok>
    where T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }
  /// Ничего не записывает в поток
  fn end(self) -> Result<Self::Ok> { Ok(()) }
}

impl<'a, BO, W> ser::SerializeTupleVariant for &'a mut Serializer<BO, W>
  where W: Write,
        BO: ByteOrder,
{
  type Ok = ();
  type Error = Error;

  /// Записывает в выходной поток представление `value` с помощью данного сериализатора
  fn serialize_field<T>(&mut self, value: &T) -> Result<Self::Ok>
    where T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }
  /// Ничего не записывает в поток
  fn end(self) -> Result<Self::Ok> { Ok(()) }
}

impl<'a, BO, W> ser::SerializeMap for &'a mut Serializer<BO, W>
  where W: Write,
        BO: ByteOrder,
{
  type Ok = ();
  type Error = Error;

  /// Записывает в выходной поток представление `key` с помощью данного сериализатора
  fn serialize_key<T>(&mut self, key: &T) -> Result<Self::Ok>
    where T: ?Sized + Serialize,
  {
    key.serialize(&mut **self)
  }
  /// Записывает в выходной поток представление `value` с помощью данного сериализатора
  fn serialize_value<T>(&mut self, value: &T) -> Result<Self::Ok>
    where T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }
  /// Ничего не записывает в поток
  fn end(self) -> Result<Self::Ok> { Ok(()) }
}

impl<'a, BO, W> ser::SerializeStruct for &'a mut Serializer<BO, W>
  where W: Write,
        BO: ByteOrder,
{
  type Ok = ();
  type Error = Error;

  /// Записывает в выходной поток представление `value` с помощью данного сериализатора
  fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<Self::Ok>
    where T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }
  /// Ничего не записывает в поток
  fn end(self) -> Result<Self::Ok> { Ok(()) }
}

impl<'a, BO, W> ser::SerializeStructVariant for &'a mut Serializer<BO, W>
  where W: Write,
        BO: ByteOrder,
{
  type Ok = ();
  type Error = Error;

  /// Записывает в выходной поток представление `value` с помощью данного сериализатора
  fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<Self::Ok>
    where T: ?Sized + Serialize,
  {
    value.serialize(&mut **self)
  }
  /// Ничего не записывает в поток
  fn end(self) -> Result<Self::Ok> { Ok(()) }
}

/// Сериализует указанное значение в поток.
///
/// # Параметры
/// - `writer`: Поток, в который необходимо записать сериализованное значение
/// - `value`: Значение для сериализации
///
/// # Параметры типа
/// - `BO`: Порядок байт, в котором записывать сериализуемые данные в поток
/// - `W`: Тип потока для записи в него значения
/// - `T`: Сериализуемый тип
///
/// # Ошибки
/// Возможны 3 причины, по которым данный метод вернет ошибку:
/// - Реализация `Serialize` для типа `T` вернет ошибку
/// - [`Error::Encoding`]: Сериализуемое значение содержит строки, которые не могут
///   быть представлены с использованием кодировки сериализатора и установленная ловушка
///   для таких случаев выдает ошибку
/// - [`Error::Io`]: `writer` выдал ошибку при записи в него значения
///
/// [`Error::Encoding`]: ../error/enum.Error.html#variant.Encoding
/// [`Error::Io`]: ../error/enum.Error.html#variant.Io
#[inline]
pub fn to_writer<BO, W, T>(writer: W, value: &T) -> Result<()>
  where BO: ByteOrder,
        W: Write,
        T: ?Sized + Serialize,
{
  let mut ser: Serializer<BO, W> = Serializer::new(writer);
  value.serialize(&mut ser)
}

/// Сериализует указанное значение в массив байт.
///
/// # Параметры
/// - `value`: Значение для сериализации
///
/// # Параметры типа
/// - `BO`: Порядок байт, в котором записывать сериализуемые данные в поток
/// - `T`: Сериализуемый тип
///
/// # Возвращаемое значение
/// Массив байт с сериализованным значением
///
/// # Ошибки
/// Возможны 2 причины, по которым данный метод вернет ошибку:
/// - Реализация `Serialize` для типа `T` вернет ошибку
/// - [`Error::Encoding`]: Сериализуемое значение содержит строки, которые не могут
///   быть представлены с использованием кодировки сериализатора и установленная ловушка
///   для таких случаев выдает ошибку
///
/// [`Error::Encoding`]: ../error/enum.Error.html#variant.Encoding
#[inline]
pub fn to_vec<BO, T>(value: &T) -> Result<Vec<u8>>
  where BO: ByteOrder,
        T: ?Sized + Serialize,
{
  let mut vec = Vec::new();
  to_writer::<BO, _, _>(&mut vec, value)?;
  Ok(vec)
}
////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod integers {
  use super::to_vec;
  use byteorder::{BE, LE};

  #[test]
  fn test_u8() {
    let test: u8 = 0x12;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), vec![0x12]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), vec![0x12]);
  }
  #[test]
  fn test_i8() {
    let test: i8 = 0x12;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), vec![0x12]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), vec![0x12]);
  }

  #[test]
  fn test_u16() {
    let test: u16 = 0x1234;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), vec![0x12, 0x34]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), vec![0x34, 0x12]);
  }
  #[test]
  fn test_i16() {
    let test: i16 = 0x1234;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), vec![0x12, 0x34]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), vec![0x34, 0x12]);
  }

  #[test]
  fn test_u32() {
    let test: u32 = 0x12345678;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), vec![0x12, 0x34, 0x56, 0x78]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), vec![0x78, 0x56, 0x34, 0x12]);
  }
  #[test]
  fn test_i32() {
    let test: i32 = 0x12345678;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), vec![0x12, 0x34, 0x56, 0x78]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), vec![0x78, 0x56, 0x34, 0x12]);
  }

  #[test]
  fn test_u64() {
    let test: u64 = 0x12345678_90ABCDEF;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), vec![0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), vec![0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12]);
  }
  #[test]
  fn test_i64() {
    let test: i64 = 0x12345678_90ABCDEF;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), vec![0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), vec![0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12]);
  }

  #[test]
  fn test_u128() {
    let test: u128 = 0x12345678_90ABCDEF_12345678_90ABCDEF;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), vec![0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), vec![0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12, 0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12]);
  }
  #[test]
  fn test_i128() {
    let test: i128 = 0x12345678_90ABCDEF_12345678_90ABCDEF;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), vec![0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), vec![0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12, 0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12]);
  }
}
#[cfg(test)]
mod floats {
  use super::to_vec;
  use byteorder::{ByteOrder, BE, LE};

  macro_rules! float_test {
    ($name:ident, $BO:ident :: $write:ident, $type:ty) => (
      quickcheck! {
        fn $name(test: $type) -> bool {
          let mut buf = [0; std::mem::size_of::<$type>()];
          $BO::$write(&mut buf, test);
          to_vec::<$BO,_>(&test).unwrap() == buf
        }
      }
    );
  }

  float_test!(test_f32_be, BE::write_f32, f32);
  float_test!(test_f32_le, LE::write_f32, f32);

  float_test!(test_f64_be, BE::write_f64, f64);
  float_test!(test_f64_le, LE::write_f64, f64);
}
#[cfg(test)]
mod complex {
  use super::to_vec;
  use byteorder::{BE, LE};

  quickcheck! {
    fn test_bool_be(test: bool) -> bool {
      let result = [if test { 1u8 } else { 0u8 }];
      to_vec::<BE, _>(&test).unwrap() == result
    }
    fn test_bool_le(test: bool) -> bool {
      let result = [if test { 1u8 } else { 0u8 }];
      to_vec::<LE, _>(&test).unwrap() == result
    }
  }
  /// При сериализации ничего не записывает в поток
  #[test]
  fn test_unit() {
    #[derive(Serialize)]
    struct Test;

    let test = Test;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), []);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), []);
  }

  /// При сериализации представляется своим нижележащим типом
  #[test]
  fn test_newtype() {
    #[derive(Serialize)]
    struct Test(u32);

    let test = Test(0x12345678);
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), [0x12, 0x34, 0x56, 0x78]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), [0x78, 0x56, 0x34, 0x12]);
  }

  /// Поля в кортеже записываются подряд, в порядке следования, без пробелов и дополнительных данных.
  /// Порядок байт переворачивается для каждого поля независимо.
  #[test]
  fn test_tuple() {
    #[derive(Serialize)]
    struct Test(u32, u16);

    let test = Test(0x12345678, 0xABCD);
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), [0x12, 0x34, 0x56, 0x78,   0xAB, 0xCD]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), [0x78, 0x56, 0x34, 0x12,   0xCD, 0xAB]);
  }

  /// Поля в структуре записываются подряд, в порядке следования, без пробелов и дополнительных данных.
  /// Порядок байт переворачивается для каждого поля независимо.
  #[test]
  fn test_struct() {
    #[derive(Serialize)]
    struct Test {
      int1: u32,
      int2: u16,
    }

    let test = Test { int1: 0x12345678, int2: 0xABCD };
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), [0x12, 0x34, 0x56, 0x78,   0xAB, 0xCD]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), [0x78, 0x56, 0x34, 0x12,   0xCD, 0xAB]);
  }

  /// Записывает значение элемента в опции
  #[test]
  fn test_option_some() {
    let test = Some(0x12345678_u32);
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), [0x12, 0x34, 0x56, 0x78]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), [0x78, 0x56, 0x34, 0x12]);
  }

  /// Ничего не записывает
  #[test]
  fn test_option_none() {
    let test: Option<u32> = None;
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), []);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), []);
  }

  /// Записывает все элементы последовательности подряд, без разделителей, заголовочной или
  /// конечной информации, либо какой-либо информации о количестве элементов.
  /// Порядок байт переворачивается для каждого поля независимо.
  #[test]
  fn test_seq() {
    let test: Vec<u16> = vec![0x1234, 0x5678, 0xABCD];
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), [0x12, 0x34,   0x56, 0x78,   0xAB, 0xCD]);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), [0x34, 0x12,   0x78, 0x56,   0xCD, 0xAB]);
  }

  #[test]
  fn test_str() {
    let test = "тест";
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), test.as_bytes());
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), test.as_bytes());
    assert_eq!(to_vec::<BE,_>(&test.to_owned()).unwrap(), test.as_bytes());
    assert_eq!(to_vec::<LE,_>(&test.to_owned()).unwrap(), test.as_bytes());
  }

  #[test]
  fn test_array_empty() {
    let test: [u8; 0] = [];
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), []);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), []);
  }
  #[test]
  fn test_array() {
    let test: [u8; 6] = [0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD];
    assert_eq!(to_vec::<BE,_>(&test).unwrap(), test);
    assert_eq!(to_vec::<LE,_>(&test).unwrap(), test);
  }
}

#[cfg(test)]
mod enums {
  use super::to_vec;
  use byteorder::{BE, LE};

  #[derive(Serialize)]
  enum E {
    /// При сериализации ничего не записывает в поток
    Unit,
    /// При сериализации представляется своим нижележащим типом
    Newtype(u32),
    /// Последовательно записывает в поток свои элементы. Порядок байт меняется
    /// в каждом поле независимо
    Tuple(u32, u16),
    /// Последовательно записывает в поток свои элементы. Порядок байт меняется
    /// в каждом поле независимо
    Struct { int1: u32, int2: u16 },
  }

  #[test]
  fn test_enum_unit() {
    let u = E::Unit;
    assert_eq!(to_vec::<BE,_>(&u).unwrap(), []);
    assert_eq!(to_vec::<LE,_>(&u).unwrap(), []);
  }

  #[test]
  fn test_enum_newtype() {
    let n = E::Newtype(0x12345678);
    assert_eq!(to_vec::<BE,_>(&n).unwrap(), [0x12, 0x34, 0x56, 0x78]);
    assert_eq!(to_vec::<LE,_>(&n).unwrap(), [0x78, 0x56, 0x34, 0x12]);
  }

  #[test]
  fn test_enum_tuple() {
    let t = E::Tuple(0x12345678, 0xABCD);
    assert_eq!(to_vec::<BE,_>(&t).unwrap(), [0x12, 0x34, 0x56, 0x78,   0xAB, 0xCD]);
    assert_eq!(to_vec::<LE,_>(&t).unwrap(), [0x78, 0x56, 0x34, 0x12,   0xCD, 0xAB]);
  }

  #[test]
  fn test_enum_struct() {
    let s = E::Struct { int1: 0x12345678, int2: 0xABCD };
    assert_eq!(to_vec::<BE,_>(&s).unwrap(), [0x12, 0x34, 0x56, 0x78,   0xAB, 0xCD]);
    assert_eq!(to_vec::<LE,_>(&s).unwrap(), [0x78, 0x56, 0x34, 0x12,   0xCD, 0xAB]);
  }
}
