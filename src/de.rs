//! Содержит тип, реализующий простую десериализацию данных, как POD типов.

use std::io::BufRead;
use std::marker::PhantomData;
use std::str;
use std::string::String;
use byteorder::{ByteOrder, ReadBytesExt};
use serde::de::{self, Deserialize, DeserializeSeed, SeqAccess, Visitor};

use error::{Error, Result};

/// Структура для десериализации потока байт, практически из значений, как они хранятся
/// в памяти, в значения Rust.
///
/// # Правила десериализации
/// Все типы десериализуются максимально прямолинейно, так, как они лежат в памяти: для
/// всех целых типов читается соответствующее их разрядности количество байт и интерпретируется
/// в соответствии с порядком байт `BO`.
///
/// Для структур и кортежей рекурсивно десериализуются их поля, без разделителей между ними.
/// Если такие разделители требуются, они должны быть внедрены непосредственно в структуру
/// или кортеж.
///
/// Тип [`()`] при десериализации всегда считается присутствующим, когда запрашивается.
///
/// [Newtype] типы десериализуются, как оборачиваемое ими значение. Так как десериализация
/// произвольных данных все равно не поддерживается, вызывающий код всегда будет знать, в какой
/// именно тип он должен обернуть возвращенные данные, таким образом маркер типа даже не требуется.
///
/// При десериализации строк байты интерпретируются в кодировке UTF-8, являющейся нативной для Rust.
/// В случае, если требуется читать строки в других кодировках, оберните их в структуры, для которых
/// будет реализован типах [`Deserialize`], выполняющий чтение массива байт из потока и конвертирующий
/// его в строку с помощью требуемой кодировки, например, используя крейт [encoding]. Чтение строки
/// продолжается до конца потока, т.к. десериализатор не способен самостоятельно определить длину
/// строки. В случае, если поток содержит некорректные UTF-8 данные, то возвращается ошибка
/// [`Error::Encoding`].
///
/// При десериализации элемента типа `char` из потока читается требуемое количество байт (от 1 до 4-х)
/// его UTF-8 представления; если в процессе чтения выясняется, что байты не составляют корректно
/// кодированное значение символа в UTF-8, возвращается ошибка [`Error::Encoding`].
///
/// Десериализация последовательностей без определенной длины (таких, как [вектор]) осуществляется простой
/// последовательной десериализацией их элементов до тех пор, пока в потоке остаются данные. Ни количество,
/// ни разделители между элементами, ни какой-либо маркер конца последовательности не читаются. В случае,
/// если они требуются для корректной десериализации, они должны быть добавлены в сериализуемые структуры
/// вручную. Для последовательностей с известной длиной (например, массивы) читается запрошенное количество
/// данных.
///
/// # Неподдерживаемые методы
/// Для некоторых типов [модели serde] десериализация не поддержана, попытка их десериализации приводит
/// к возврату ошибки [`Error::Unsupported`]. Также это означает, что [сериализатор] несимметричен по отношению
/// к десериализатору: не все, что может быть закодировано, может быть раскодировано.
///
/// К неподдерживаемым типам модели относятся:
/// - Оба варианта [`Option`] -- десериализатор не способен самостоятельно их различить. При необходимости
///   десериализации типа [`Option`] можно реализовать собственную структуру, для которой реализовать
///   типаж [`Deserialize`] и выполнить чтение маркера типа и данных `Some` варианта, если в потоке записан
///   `Some` вариант
/// - Перечисления. Также как и в предыдущем случае, десериализатор не способен самостоятельно определить,
///   какой из вариантов записан в потоке. Стоит отметить, что данное ограничение применимо только к
///   [варианту десериализации][enum] перечислений в externally tagged виде (с внешней пометкой), который
///   является вариантом сериализации перечислений в serde по умолчанию. В остальных случаях serde десериализует
///   перечисления, как структуры, что уже поддерживается десериализатором.
/// - Тип `bool` также не поддерживается ввиду того, что десериализатор не знает, сколько байт читать и как
///   их интерпретировать. Так как обычно булевы значения записываются в виде числа, не должно возникнуть
///   проблем использовать вместо типа `bool` число, соответствующее его представлению в сериализованных данных.
/// - Десериализация произвольных данных и отображений (map) также не поддерживается. Отображения обычно будут
///   записаны в потоке, как список пар ключ-значение, поэтому не должно возникнуть проблем десериализовывать
///   именно такие структуры, а затем приводить их в требуемый вид.
///
/// # Параметры типа
/// - `BO`: определяет порядок байт, в котором будут записаны примитивные числовые типы:
///         `u16`, `u32`, `u64`, `u128`, `i16`, `i32`, `i64`, `i128`, `f32` и `f64`.
/// - `W`: определяет тип, обеспечивающих сохранение сериализуемых данных в хранилище
///
/// [`()`]: https://doc.rust-lang.org/std/primitive.unit.html
/// [Newtype]: https://doc.rust-lang.org/rust-by-example/generics/new_types.html
/// [`Deserialize`]: https://docs.serde.rs/serde/trait.Deserialize.html
/// [encoding]: https://docs.rs/encoding/
/// [`Error::Encoding`]: ../error/enum.Error.html#variant.Encoding
/// [вектор]: https://doc.rust-lang.org/std/vec/struct.Vec.html
/// [модели serde]: https://serde.rs/data-model.html
/// [`Error::Unsupported`]: ../error/enum.Error.html#variant.Unsupported
/// [сериализатор]: ../ser/struct.Serializer.html
/// [`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
/// [enum]: https://serde.rs/enum-representations.html
pub struct Deserializer<BO, R> {
  /// Источник данных для десериализации
  reader: R,
  /// Порядок байт, используемый при чтении чисел
  _byteorder: PhantomData<BO>,
}

impl<BO, R> Deserializer<BO, R>
  where R: BufRead,
        BO: ByteOrder,
{
  /// Создает десериализатор с настройками по умолчанию. Строки кодируются в UTF-8,
  /// если встречается непредставимый символ, декодирование прерывается и возвращается ошибка
  ///
  /// # Параметры
  /// - `reader`: Поток, из которого будут читаться данные. Буферизация требуется для возможности
  ///   определения окончания последовательностей, т.к. последовательности читаются до конца потока
  ///   и требуется возможность определять, имеются ли в потоке еще данные или нет
  ///
  /// # Возвращаемое значение
  /// Десериализатор для чтения данных из указанного потока и кодированием строк в UTF-8
  pub fn new(reader: R) -> Self {
    Deserializer { reader, _byteorder: PhantomData }
  }
  /// Читает все данные из потока в вектор и возвращает его
  #[inline]
  fn read_to_end(&mut self) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    self.reader.read_to_end(&mut buf)?;
    Ok(buf)
  }
  /// Читает из потока один символ в кодировке UTF-8 (т.е. 1-4 байта для его формирования) и
  /// возвращает его, либо возвращает ошибку, если в потоке недостаточно байт для декодирования
  /// символа или они не представляют валидный символ в UTF-8
  fn read_char(&mut self) -> Result<char> {
    // Скопировано из реализации нестабильной функции core::str::utf8_char_width
    // https://tools.ietf.org/html/rfc3629
    static UTF8_CHAR_WIDTH: [u8; 256] = [
      1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
      1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x1F
      1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
      1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x3F
      1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
      1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x5F
      1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
      1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x7F
      0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
      0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 0x9F
      0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
      0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 0xBF
      0,0,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
      2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2, // 0xDF
      3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3, // 0xEF
      4,4,4,4,4,0,0,0,0,0,0,0,0,0,0,0, // 0xFF
    ];

    let mut buf = [0u8; 4];
    self.reader.read_exact(&mut buf[..1])?;// читаем 1 символ
    let width = UTF8_CHAR_WIDTH[buf[0] as usize] as usize;
    if width == 1 {
      return Ok(buf[0] as char);
    }
    self.reader.read_exact(&mut buf[1..width])?;
    let s = str::from_utf8(&buf[..width])?;
    s.chars().next().ok_or_else(|| Error::Unknown("UTF-8 bytes decoded as empty string".into()))
  }
}

/// Макрос, генерирующий код десериализации числовых типов
macro_rules! impl_numbers {
  ($dser_method:ident, $visitor_method:ident, $reader_method:ident) => {
    fn $dser_method<V>(self, visitor: V) -> Result<V::Value>
      where V: de::Visitor<'de>,
    {
      visitor.$visitor_method(self.reader.$reader_method::<BO>()?)
    }
  }
}
/// Макрос, генерирующий метод, возвращающий ошибку [`Error::Unsupported`]
///
/// [`Error::Unsupported`]: ../error/enum.Error.html#variant.Unsupported
macro_rules! unsupported {
  ($dser_method:ident) => {
    /// Всегда возвращает ошибку [`Error::Unsupported`]
    ///
    /// [`Error::Unsupported`]: ../error/enum.Error.html#variant.Unsupported
    fn $dser_method<V>(self, _visitor: V) -> Result<V::Value>
      where V: Visitor<'de>,
    {
      Err(Error::Unsupported(concat!('`', stringify!($dser_method), "` is not supported")))
    }
  }
}

impl<'de, 'a, BO, R> de::Deserializer<'de> for &'a mut Deserializer<BO, R>
  where R: BufRead,
        BO: ByteOrder,
{
  type Error = Error;

  /// Читает из потока 1 байт, интерпретируя его, как число со знаком
  fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    visitor.visit_i8(self.reader.read_i8()?)
  }
  /// Читает из потока 1 байт, интерпретируя его, как беззнаковое число
  fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    visitor.visit_u8(self.reader.read_u8()?)
  }
  impl_numbers!(deserialize_i16, visit_i16, read_i16);
  impl_numbers!(deserialize_u16, visit_u16, read_u16);
  impl_numbers!(deserialize_i32, visit_i32, read_i32);
  impl_numbers!(deserialize_u32, visit_u32, read_u32);
  impl_numbers!(deserialize_i64, visit_i64, read_i64);
  impl_numbers!(deserialize_u64, visit_u64, read_u64);
  impl_numbers!(deserialize_i128, visit_i128, read_i128);
  impl_numbers!(deserialize_u128, visit_u128, read_u128);
  impl_numbers!(deserialize_f32, visit_f32, read_f32);
  impl_numbers!(deserialize_f64, visit_f64, read_f64);

  fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    visitor.visit_char(self.read_char()?)
  }
  #[inline]
  fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    self.deserialize_string(visitor)
  }
  /// Читает байты до конца потока, возвращает их посетителю в виде владеющего буфера.
  /// Так как десериализатор сам не может определить, где заканчиваются данные, то для
  /// десериализации сложных структур внешний код должен ограничить размер буфера концом
  /// строки.
  ///
  /// Прочитанные байт интерпретируются, как строка в кодировке UTF-8, в случае, если это не так,
  /// возвращается ошибка [`Error::Encoding`]
  ///
  /// [`Error::Encoding`]: ../error/enum.Error.html#variant.Encoding
  fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    let buf = self.read_to_end()?;
    visitor.visit_string(String::from_utf8(buf)?)
  }
  #[inline]
  fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    self.deserialize_byte_buf(visitor)
  }
  fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    visitor.visit_byte_buf(self.read_to_end()?)
  }
  /// Безусловно вызывает [`Visitor::visit_unit`]
  ///
  /// [`Visitor::visit_unit`]: https://docs.serde.rs/serde/de/trait.Visitor.html#method.visit_unit
  fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    visitor.visit_unit()
  }
  /// Безусловно вызывает [`Visitor::visit_unit`]. Аргумент `_name` игнорируется
  ///
  /// [`Visitor::visit_unit`]: https://docs.serde.rs/serde/de/trait.Visitor.html#method.visit_unit
  fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    visitor.visit_unit()
  }
  /// Безусловно вызывает [`Visitor::visit_newtype_struct`]. Аргумент `_name` игнорируется
  ///
  /// [`Visitor::visit_newtype_struct`]: https://docs.serde.rs/serde/de/trait.Visitor.html#method.visit_newtype_struct
  fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    visitor.visit_newtype_struct(self)
  }
  /// Десериализует последовательность, последовательно вычитывая ее элементы, пока не кончатся
  /// данные в потоке. Элементы ничем не разделяются, никакого начального или конечного разделителя
  /// не читается: если что-либо из этого требуется, они должны быть представлены, как читаемые
  /// данные. Безусловно вызывает [`Visitor::visit_seq`]
  ///
  /// [`Visitor::visit_seq`]: https://docs.serde.rs/serde/de/trait.Visitor.html#method.visit_seq
  fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    visitor.visit_seq(self)
  }
  /// Десериализует кортеж, как последовательность его полей: безусловно вызывает
  /// [`Visitor::visit_seq`].
  ///
  /// [`Visitor::visit_seq`]: https://docs.serde.rs/serde/de/trait.Visitor.html#method.visit_seq
  fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    visitor.visit_seq(Tuple { de: self, count: len })
  }
  /// Десериализует кортеж, как последовательность его полей: безусловно вызывает
  /// [`Visitor::visit_seq`]. Аргумент `_name` игнорируется
  ///
  /// [`Visitor::visit_seq`]: https://docs.serde.rs/serde/de/trait.Visitor.html#method.visit_seq
  #[inline]
  fn deserialize_tuple_struct<V>(self, _name: &'static str, len: usize, visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    self.deserialize_tuple(len, visitor)
  }
  /// Десериализует структуру, как последовательность ее полей: безусловно вызывает
  /// [`Visitor::visit_seq`]. Аргумент `_name` игнорируется, в аргументе `fields` важна только его длина
  ///
  /// [`Visitor::visit_seq`]: https://docs.serde.rs/serde/de/trait.Visitor.html#method.visit_seq
  #[inline]
  fn deserialize_struct<V>(self, _name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    self.deserialize_tuple(fields.len(), visitor)
  }

  unsupported!(deserialize_any);
  unsupported!(deserialize_map);
  unsupported!(deserialize_bool);
  unsupported!(deserialize_option);
  unsupported!(deserialize_identifier);
  unsupported!(deserialize_ignored_any);
  fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str], _visitor: V) -> Result<V::Value>
    where V: Visitor<'de>,
  {
    Err(Error::Unsupported("`deserialize_enum` is not supported"))
  }
}

/// Структура, используемая для чтения ограниченных по количеству последовательностей,
/// таких, как массивы, структуры и кортежи
struct Tuple<'a, BO, R> {
  /// Объект, используемый для чтения и десериализации элементов
  de: &'a mut Deserializer<BO, R>,
  /// Количество элементов, которое осталось прочитать
  count: usize,
}
impl<'a, 'de, BO, R> SeqAccess<'de> for Tuple<'a, BO, R>
  where R: BufRead,
        BO: ByteOrder,
{
  type Error = Error;

  fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where T: DeserializeSeed<'de>,
  {
    // Если еще есть элементы для чтения, вытаскиваем их
    if self.count > 0 {
      self.count -= 1;
      return seed.deserialize(&mut *self.de).map(Some);
    }
    return Ok(None);
  }

  fn size_hint(&self) -> Option<usize> { Some(self.count) }
}

impl<'a, 'de, BO, R> SeqAccess<'de> for &'a mut Deserializer<BO, R>
  where R: BufRead,
        BO: ByteOrder,
{
  type Error = Error;

  fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where T: DeserializeSeed<'de>,
  {
    // Если данные закончились, прекращаем итерации
    if self.reader.fill_buf()?.is_empty() {
      return Ok(None);
    }
    seed.deserialize(&mut **self).map(Some)
  }
}

/// Десериализует значение заданного типа из массива байт.
///
/// # Параметры
/// - `storage`: Массив байт, содержащий сериализованное значение
///
/// # Параметры типа
/// - `BO`: Порядок байт, в котором читать данные из потока
/// - `T`: Десериализуемый тип
///
/// # Возвращаемое значение
/// Прочитанное значение
///
/// # Ошибки
/// Возможны 2 причины, по которым данный метод вернет ошибку:
/// - Реализация `Deserialize` для типа `T` вернет ошибку
/// - [`Error::Encoding`]: Десериализуемый тип содержит [строки], и в десериализуемых
///   данных они не содержат корректных UTF-8 последовательностей
///
/// [`Error::Encoding`]: ../error/enum.Error.html#variant.Encoding
/// [строки]: https://doc.rust-lang.org/std/string/struct.String.html
pub fn from_bytes<'a, BO, T>(storage: &'a [u8]) -> Result<T>
  where T: Deserialize<'a>,
        BO: ByteOrder,
{
  let mut deserializer: Deserializer<BO, _> = Deserializer::new(storage);
  T::deserialize(&mut deserializer)
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod integers {
  use super::from_bytes;
  use byteorder::{BE, LE};

  #[test]
  fn test_u8() {
    let test: u8 = 0x12;
    assert_eq!(from_bytes::<BE, u8>(&[0x12]).unwrap(), test);
    assert_eq!(from_bytes::<LE, u8>(&[0x12]).unwrap(), test);
  }
  #[test]
  fn test_i8() {
    let test: i8 = 0x12;
    assert_eq!(from_bytes::<BE, i8>(&[0x12]).unwrap(), test);
    assert_eq!(from_bytes::<LE, i8>(&[0x12]).unwrap(), test);
  }

  #[test]
  fn test_u16() {
    let test: u16 = 0x1234;
    assert_eq!(from_bytes::<BE, u16>(&[0x12, 0x34]).unwrap(), test);
    assert_eq!(from_bytes::<LE, u16>(&[0x34, 0x12]).unwrap(), test);
  }
  #[test]
  fn test_i16() {
    let test: i16 = 0x1234;
    assert_eq!(from_bytes::<BE, i16>(&[0x12, 0x34]).unwrap(), test);
    assert_eq!(from_bytes::<LE, i16>(&[0x34, 0x12]).unwrap(), test);
  }

  #[test]
  fn test_u32() {
    let test: u32 = 0x12345678;
    assert_eq!(from_bytes::<BE, u32>(&[0x12, 0x34, 0x56, 0x78]).unwrap(), test);
    assert_eq!(from_bytes::<LE, u32>(&[0x78, 0x56, 0x34, 0x12]).unwrap(), test);
  }
  #[test]
  fn test_i32() {
    let test: i32 = 0x12345678;
    assert_eq!(from_bytes::<BE, i32>(&[0x12, 0x34, 0x56, 0x78]).unwrap(), test);
    assert_eq!(from_bytes::<LE, i32>(&[0x78, 0x56, 0x34, 0x12]).unwrap(), test);
  }

  #[test]
  fn test_u64() {
    let test: u64 = 0x12345678_90ABCDEF;
    assert_eq!(from_bytes::<BE, u64>(&[0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]).unwrap(), test);
    assert_eq!(from_bytes::<LE, u64>(&[0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12]).unwrap(), test);
  }
  #[test]
  fn test_i64() {
    let test: i64 = 0x12345678_90ABCDEF;
    assert_eq!(from_bytes::<BE, i64>(&[0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]).unwrap(), test);
    assert_eq!(from_bytes::<LE, i64>(&[0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12]).unwrap(), test);
  }

  #[test]
  fn test_u128() {
    let test: u128 = 0x12345678_90ABCDEF_12345678_90ABCDEF;
    assert_eq!(from_bytes::<BE, u128>(&[0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]).unwrap(), test);
    assert_eq!(from_bytes::<LE, u128>(&[0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12, 0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12]).unwrap(), test);
  }
  #[test]
  fn test_i128() {
    let test: i128 = 0x12345678_90ABCDEF_12345678_90ABCDEF;
    assert_eq!(from_bytes::<BE, i128>(&[0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF]).unwrap(), test);
    assert_eq!(from_bytes::<LE, i128>(&[0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12, 0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12]).unwrap(), test);
  }
}
#[cfg(test)]
mod floats {
  use super::from_bytes;
  use byteorder::{ByteOrder, BE, LE};

  macro_rules! float_test {
    ($name:ident, $BO:ident :: $write:ident, $type:ty) => (
      quickcheck! {
        fn $name(test: $type) -> bool {
          let mut buf = [0; std::mem::size_of::<$type>()];
          $BO::$write(&mut buf, test);
          from_bytes::<$BO, $type>(&buf).unwrap() == test
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
  use super::from_bytes;
  use byteorder::{BE, LE};

  quickcheck! {
    #[should_panic]
    fn test_bool(byte: u8) -> bool {
      from_bytes::<BE, bool>(&[byte]).unwrap()
    }
  }
  /// При десериализации ничего не читает из потока
  #[test]
  fn test_unit() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Test;

    let test = Test;
    assert_eq!(from_bytes::<BE, Test>(&[]).unwrap(), test);
    assert_eq!(from_bytes::<LE, Test>(&[]).unwrap(), test);
  }

  /// При десериализации читает из потока нижележащий тип
  #[test]
  fn test_newtype() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Test(u32);

    let test = Test(0x12345678);
    assert_eq!(from_bytes::<BE, Test>(&[0x12, 0x34, 0x56, 0x78]).unwrap(), test);
    assert_eq!(from_bytes::<LE, Test>(&[0x78, 0x56, 0x34, 0x12]).unwrap(), test);
  }

  /// Поля в кортеже записываются подряд, в порядке следования, без пробелов и дополнительных данных.
  /// Порядок байт переворачивается для каждого поля независимо.
  #[test]
  fn test_tuple() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Test(u32, u16);

    let test = Test(0x12345678, 0xABCD);
    assert_eq!(from_bytes::<BE, Test>(&[0x12, 0x34, 0x56, 0x78,   0xAB, 0xCD]).unwrap(), test);
    assert_eq!(from_bytes::<LE, Test>(&[0x78, 0x56, 0x34, 0x12,   0xCD, 0xAB]).unwrap(), test);
  }

  /// Поля в структуре записываются подряд, в порядке следования, без пробелов и дополнительных данных.
  /// Порядок байт переворачивается для каждого поля независимо.
  #[test]
  fn test_struct() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Test {
      int1: u32,
      int2: u16,
    }

    let test = Test { int1: 0x12345678, int2: 0xABCD };
    assert_eq!(from_bytes::<BE, Test>(&[0x12, 0x34, 0x56, 0x78,   0xAB, 0xCD]).unwrap(), test);
    assert_eq!(from_bytes::<LE, Test>(&[0x78, 0x56, 0x34, 0x12,   0xCD, 0xAB]).unwrap(), test);
  }

  /// Десериализатор не в состоянии различить `Some` и `None` -- для десериализации нужно
  /// вручную прочитать (или определить) маркер, и прочитать значение, если маркер говорит,
  /// что оно есть
  #[test]
  #[should_panic]
  fn test_option_be() {
    from_bytes::<BE, Option<u16>>(&[0x12, 0x34]).unwrap();
  }
  #[test]
  #[should_panic]
  fn test_option_le() {
    from_bytes::<LE, Option<u16>>(&[0x12, 0x34]).unwrap();
  }

  /// Записывает все элементы последовательности подряд, без разделителей, заголовочной или
  /// конечной информации, либо какой-либо информации о количестве элементов.
  /// Порядок байт переворачивается для каждого поля независимо.
  #[test]
  fn test_seq() {
    let test = [0x12, 0x34,   0x56, 0x78,   0xAB, 0xCD];
    assert_eq!(from_bytes::<BE, Vec<u16>>(&test).unwrap(), vec![0x1234, 0x5678, 0xABCD]);
    assert_eq!(from_bytes::<LE, Vec<u16>>(&test).unwrap(), vec![0x3412, 0x7856, 0xCDAB]);
  }

  /// Возврат срезов строки не поддерживается, т.к. десериализатор всегда выдает новую строку
  #[test]
  #[should_panic]
  fn test_str_be() {
    from_bytes::<BE, &str>("test".as_bytes()).unwrap();
  }
  #[test]
  #[should_panic]
  fn test_str_le() {
    from_bytes::<LE, &str>("test".as_bytes()).unwrap();
  }
  #[test]
  fn test_string() {
    let test = "тест";
    assert_eq!(from_bytes::<BE, String>(test.as_bytes()).unwrap(), test);
    assert_eq!(from_bytes::<LE, String>(test.as_bytes()).unwrap(), test);
  }

  #[test]
  fn test_array_empty() {
    assert_eq!(from_bytes::<BE, [u16; 0]>(&[]).unwrap(), []);
    assert_eq!(from_bytes::<LE, [u16; 0]>(&[]).unwrap(), []);
  }
  #[test]
  fn test_array() {
    let test = [0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD];
    assert_eq!(from_bytes::<BE, [u16; 3]>(&test).unwrap(), [0x1234, 0x5678, 0xABCD]);
    assert_eq!(from_bytes::<LE, [u16; 3]>(&test).unwrap(), [0x3412, 0x7856, 0xCDAB]);
  }
  #[test]
  #[should_panic]
  fn test_array_no_data_be() {
    let test = [0x12, 0x34, 0x56, 0x78, 0xAB];
    from_bytes::<BE, [u16; 3]>(&test).unwrap();
  }
  #[test]
  #[should_panic]
  fn test_array_no_data_le() {
    let test = [0x12, 0x34, 0x56, 0x78, 0xAB];
    from_bytes::<LE, [u16; 3]>(&test).unwrap();
  }
  #[test]
  fn test_vec() {
    let test = [0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD];
    assert_eq!(from_bytes::<BE, Vec<u16>>(&test).unwrap(), vec![0x1234, 0x5678, 0xABCD]);
    assert_eq!(from_bytes::<LE, Vec<u16>>(&test).unwrap(), vec![0x3412, 0x7856, 0xCDAB]);
  }
  #[test]
  #[should_panic]
  fn test_vec_no_data_be() {
    let test = [0x12, 0x34, 0x56, 0x78, 0xAB];
    from_bytes::<BE, Vec<u16>>(&test).unwrap();
  }
  #[test]
  #[should_panic]
  fn test_vec_no_data_le() {
    let test = [0x12, 0x34, 0x56, 0x78, 0xAB];
    from_bytes::<LE, Vec<u16>>(&test).unwrap();
  }
}
