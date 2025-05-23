use std::collections::HashMap;
use std::io::{self, Read, Write, Seek, SeekFrom};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Error types mirroring `construct.core` exceptions.
#[derive(Debug)]
pub enum ConstructError {
    SizeofError,
    AdaptationError,
    ValidationError,
    StreamError,
    FormatFieldError,
    IntegerError,
    StringError,
    MappingError,
    RangeError,
    RepeatError,
    ConstError,
    IndexFieldError,
    CheckError,
    ExplicitError,
    NamedTupleError,
    TimestampError,
    UnionError,
    SelectError,
    SwitchError,
    StopFieldError,
    PaddingError,
    TerminatedError,
    RawCopyError,
    RotationError,
    ChecksumError,
    CancelParsing,
    Other(String),
}

impl std::fmt::Display for ConstructError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ConstructError {}

/// Read exactly `length` bytes from a stream.
pub fn stream_read(stream: &mut impl Read, length: usize) -> Result<Vec<u8>, ConstructError> {
    let mut buf = vec![0u8; length];
    stream.read_exact(&mut buf).map_err(|_| ConstructError::StreamError)?;
    Ok(buf)
}

/// Read all remaining bytes from a stream.
pub fn stream_read_entire(stream: &mut impl Read) -> Result<Vec<u8>, ConstructError> {
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).map_err(|_| ConstructError::StreamError)?;
    Ok(buf)
}

/// Write data into a stream.
pub fn stream_write(stream: &mut impl Write, data: &[u8]) -> Result<(), ConstructError> {
    stream.write_all(data).map_err(|_| ConstructError::StreamError)
}

/// Seek a stream to `offset` according to `whence`.
pub fn stream_seek(stream: &mut impl Seek, offset: i64, whence: SeekFrom) -> Result<u64, ConstructError> {
    stream.seek(match whence {
        SeekFrom::Start(_) | SeekFrom::End(_) | SeekFrom::Current(_) => whence,
    }).map_err(|_| ConstructError::StreamError)
}

/// Get current position of a stream.
pub fn stream_tell(stream: &mut impl Seek) -> Result<u64, ConstructError> {
    stream.seek(SeekFrom::Current(0)).map_err(|_| ConstructError::StreamError)
}

/// Return size of stream without changing position.
pub fn stream_size(stream: &mut impl Seek) -> Result<u64, ConstructError> {
    let pos = stream.seek(SeekFrom::Current(0)).map_err(|_| ConstructError::StreamError)?;
    let end = stream.seek(SeekFrom::End(0)).map_err(|_| ConstructError::StreamError)?;
    stream.seek(SeekFrom::Start(pos)).map_err(|_| ConstructError::StreamError)?;
    Ok(end)
}

/// Check if end of file has been reached without consuming data.
pub fn stream_iseof(stream: &mut (impl Read + Seek)) -> Result<bool, ConstructError> {
    let pos = stream.seek(SeekFrom::Current(0)).map_err(|_| ConstructError::StreamError)?;
    let mut buf = [0u8; 1];
    let read = stream.read(&mut buf).map_err(|_| ConstructError::StreamError)?;
    stream.seek(SeekFrom::Start(pos)).map_err(|_| ConstructError::StreamError)?;
    Ok(read == 0)
}

/// Replace underscores with hyphens in keys of the map.
pub fn hyphenatedict(input: &HashMap<String, String>) -> HashMap<String, String> {
    input.iter().map(|(k, v)| {
        let key = k.replace('_', "-").trim_end_matches('-').to_string();
        (key, v.clone())
    }).collect()
}

/// Apply [`hyphenatedict`] to all dictionaries in the slice.
pub fn hyphenatelist(list: &[HashMap<String, String>]) -> Vec<HashMap<String, String>> {
    list.iter().map(hyphenatedict).collect()
}

// ========================= BitsInteger ================================

#[pyclass(extends=Construct)]
pub struct BitsInteger {
    length: usize,
    signed: bool,
    swapped: bool,
}

#[pymethods]
impl BitsInteger {
    #[new]
    fn new(length: usize, signed: Option<bool>, swapped: Option<bool>) -> (Self, Construct) {
        (
            BitsInteger {
                length,
                signed: signed.unwrap_or(false),
                swapped: swapped.unwrap_or(false),
            },
            Construct {},
        )
    }

    fn parse<'py>(&self, py: Python<'py>, data: &PyBytes) -> PyResult<PyObject> {
        let mut bits = data.as_bytes().to_vec();
        if bits.len() != self.length {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "input length mismatch",
            ));
        }
        if self.swapped {
            if self.length % 8 != 0 {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "little-endianness is only defined for multiples of 8 bits",
                ));
            }
            bits.reverse();
        }
        let val = bits2integer(&bits, self.signed);
        Ok(val.into_py(py))
    }

    fn build<'py>(&self, py: Python<'py>, obj: &PyAny) -> PyResult<&'py PyBytes> {
        let mut val: i128 = obj.extract()?;
        if val < 0 && !self.signed {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "value is negative, but field is not signed",
            ));
        }
        let mut bits = integer2bits(val, self.length)
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("integer error"))?;
        if self.swapped {
            if self.length % 8 != 0 {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "little-endianness is only defined for multiples of 8 bits",
                ));
            }
            bits.reverse();
        }
        Ok(PyBytes::new(py, &bits))
    }

    fn sizeof(&self) -> PyResult<usize> {
        Ok(self.length)
    }
}

/// Convert an integer into a bit string using big-endian bit order.
pub fn integer2bits(mut number: i128, width: usize) -> Result<Vec<u8>, ConstructError> {
    if width > 128 {
        return Err(ConstructError::IntegerError);
    }
    if width == 0 {
        return Ok(Vec::new());
    }
    if number < 0 {
        number += 1i128.checked_shl(width as u32).ok_or(ConstructError::IntegerError)?;
    }
    let mut bits = vec![0u8; width];
    for i in (0..width).rev() {
        bits[i] = (number & 1) as u8;
        number >>= 1;
    }
    Ok(bits)
}

/// Convert a big-endian bit string into an integer.
pub fn bits2integer(data: &[u8], signed: bool) -> i128 {
    let mut number: i128 = 0;
    for &b in data {
        number = (number << 1) | if b != 0 { 1 } else { 0 };
    }
    if signed && !data.is_empty() && data[0] != 0 {
        let bias = 1i128 << data.len();
        number - bias
    } else {
        number
    }
}

/// Convert an integer into a big-endian byte string.
pub fn integer2bytes(mut number: i128, width: usize) -> Result<Vec<u8>, ConstructError> {
    if width > 16 {
        return Err(ConstructError::IntegerError);
    }
    if number < 0 {
        number += 1i128.checked_shl((width * 8) as u32).ok_or(ConstructError::IntegerError)?;
    }
    let mut acc = vec![0u8; width];
    for i in (0..width).rev() {
        acc[i] = (number & 0xff) as u8;
        number >>= 8;
    }
    Ok(acc)
}

/// Convert a big-endian byte string into an integer.
pub fn bytes2integer(data: &[u8], signed: bool) -> i128 {
    let mut number: i128 = 0;
    for &b in data {
        number = (number << 8) | (b as i128);
    }
    if signed && !data.is_empty() && data[0] & 0x80 != 0 {
        let bias = 1i128 << (data.len() * 8);
        number - bias
    } else {
        number
    }
}

/// Reverse byte order of a bit string.
pub fn swapbytes(mut data: Vec<u8>) -> Vec<u8> {
    data.reverse();
    data
}

// ========================= BytesInteger ================================

#[pyclass(extends=Construct)]
pub struct BytesInteger {
    length: usize,
    signed: bool,
    swapped: bool,
}

#[pymethods]
impl BytesInteger {
    #[new]
    fn new(length: usize, signed: Option<bool>, swapped: Option<bool>) -> (Self, Construct) {
        (
            BytesInteger {
                length,
                signed: signed.unwrap_or(false),
                swapped: swapped.unwrap_or(false),
            },
            Construct {},
        )
    }

    fn parse<'py>(&self, py: Python<'py>, data: &PyBytes) -> PyResult<PyObject> {
        let mut bytes = data.as_bytes().to_vec();
        if bytes.len() != self.length {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("input length mismatch"));
        }
        if self.swapped {
            bytes.reverse();
        }
        let val = bytes2integer(&bytes, self.signed);
        Ok(val.into_py(py))
    }

    fn build<'py>(&self, py: Python<'py>, obj: &PyAny) -> PyResult<&'py PyBytes> {
        let mut val: i128 = obj.extract()?;
        if val < 0 && !self.signed {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "value is negative, but field is not signed",
            ));
        }
        let mut data = integer2bytes(val, self.length)
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyValueError, _>("integer error"))?;
        if self.swapped {
            data.reverse();
        }
        Ok(PyBytes::new(py, &data))
    }

    fn sizeof(&self) -> PyResult<usize> {
        Ok(self.length)
    }
}

// ========================= FormatField ================================

#[pyclass(extends=Construct)]
pub struct FormatField {
    endian: char,
    format: char,
    length: usize,
}

#[pymethods]
impl FormatField {
    #[new]
    fn new(endian: &str, format: &str) -> PyResult<(Self, Construct)> {
        let e = endian.chars().next().unwrap_or('>');
        let f = format.chars().next().unwrap_or('B');
        let length = match f {
            'b' | 'B' => 1,
            'h' | 'H' => 2,
            'l' | 'L' | 'f' => 4,
            'q' | 'Q' | 'd' => 8,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("bad format")),
        };
        Ok((FormatField { endian: e, format: f, length }, Construct {}))
    }

    fn parse<'py>(&self, py: Python<'py>, data: &PyBytes) -> PyResult<PyObject> {
        let buf = data.as_bytes();
        if buf.len() != self.length {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("input length mismatch"));
        }
        let val = match self.format {
            'B' => buf[0] as i128,
            'b' => (buf[0] as i8) as i128,
            'H' => {
                let mut arr = [0u8; 2];
                arr.copy_from_slice(buf);
                let v = match self.endian {
                    '>' => u16::from_be_bytes(arr),
                    '<' => u16::from_le_bytes(arr),
                    '=' => u16::from_ne_bytes(arr),
                    _ => u16::from_be_bytes(arr),
                };
                v as i128
            }
            'h' => {
                let mut arr = [0u8; 2];
                arr.copy_from_slice(buf);
                let v = match self.endian {
                    '>' => i16::from_be_bytes(arr),
                    '<' => i16::from_le_bytes(arr),
                    '=' => i16::from_ne_bytes(arr),
                    _ => i16::from_be_bytes(arr),
                };
                v as i128
            }
            'L' | 'l' => {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(buf);
                if self.format == 'L' {
                    let v = match self.endian {
                        '>' => u32::from_be_bytes(arr),
                        '<' => u32::from_le_bytes(arr),
                        '=' => u32::from_ne_bytes(arr),
                        _ => u32::from_be_bytes(arr),
                    };
                    v as i128
                } else {
                    let v = match self.endian {
                        '>' => i32::from_be_bytes(arr),
                        '<' => i32::from_le_bytes(arr),
                        '=' => i32::from_ne_bytes(arr),
                        _ => i32::from_be_bytes(arr),
                    };
                    v as i128
                }
            }
            'Q' | 'q' => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(buf);
                if self.format == 'Q' {
                    let v = match self.endian {
                        '>' => u64::from_be_bytes(arr),
                        '<' => u64::from_le_bytes(arr),
                        '=' => u64::from_ne_bytes(arr),
                        _ => u64::from_be_bytes(arr),
                    };
                    v as i128
                } else {
                    let v = match self.endian {
                        '>' => i64::from_be_bytes(arr),
                        '<' => i64::from_le_bytes(arr),
                        '=' => i64::from_ne_bytes(arr),
                        _ => i64::from_be_bytes(arr),
                    };
                    v as i128
                }
            }
            'f' => {
                let mut arr = [0u8; 4];
                arr.copy_from_slice(buf);
                let v = match self.endian {
                    '>' => f32::from_be_bytes(arr),
                    '<' => f32::from_le_bytes(arr),
                    '=' => f32::from_ne_bytes(arr),
                    _ => f32::from_be_bytes(arr),
                };
                return Ok((v as f64).into_py(py));
            }
            'd' => {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(buf);
                let v = match self.endian {
                    '>' => f64::from_be_bytes(arr),
                    '<' => f64::from_le_bytes(arr),
                    '=' => f64::from_ne_bytes(arr),
                    _ => f64::from_be_bytes(arr),
                };
                return Ok(v.into_py(py));
            }
            _ => 0,
        };
        Ok(val.into_py(py))
    }

    fn build<'py>(&self, py: Python<'py>, obj: &PyAny) -> PyResult<&'py PyBytes> {
        let bytes = match self.format {
            'B' => {
                let v: u8 = obj.extract()?;
                vec![v]
            }
            'b' => {
                let v: i8 = obj.extract()?;
                vec![v as u8]
            }
            'H' => {
                let v: u16 = obj.extract()?;
                match self.endian {
                    '>' => v.to_be_bytes().to_vec(),
                    '<' => v.to_le_bytes().to_vec(),
                    '=' => v.to_ne_bytes().to_vec(),
                    _ => v.to_be_bytes().to_vec(),
                }
            }
            'h' => {
                let v: i16 = obj.extract()?;
                match self.endian {
                    '>' => v.to_be_bytes().to_vec(),
                    '<' => v.to_le_bytes().to_vec(),
                    '=' => v.to_ne_bytes().to_vec(),
                    _ => v.to_be_bytes().to_vec(),
                }
            }
            'L' | 'l' => {
                if self.format == 'L' {
                    let v: u32 = obj.extract()?;
                    match self.endian {
                        '>' => v.to_be_bytes().to_vec(),
                        '<' => v.to_le_bytes().to_vec(),
                        '=' => v.to_ne_bytes().to_vec(),
                        _ => v.to_be_bytes().to_vec(),
                    }
                } else {
                    let v: i32 = obj.extract()?;
                    match self.endian {
                        '>' => v.to_be_bytes().to_vec(),
                        '<' => v.to_le_bytes().to_vec(),
                        '=' => v.to_ne_bytes().to_vec(),
                        _ => v.to_be_bytes().to_vec(),
                    }
                }
            }
            'Q' | 'q' => {
                if self.format == 'Q' {
                    let v: u64 = obj.extract()?;
                    match self.endian {
                        '>' => v.to_be_bytes().to_vec(),
                        '<' => v.to_le_bytes().to_vec(),
                        '=' => v.to_ne_bytes().to_vec(),
                        _ => v.to_be_bytes().to_vec(),
                    }
                } else {
                    let v: i64 = obj.extract()?;
                    match self.endian {
                        '>' => v.to_be_bytes().to_vec(),
                        '<' => v.to_le_bytes().to_vec(),
                        '=' => v.to_ne_bytes().to_vec(),
                        _ => v.to_be_bytes().to_vec(),
                    }
                }
            }
            'f' => {
                let v: f64 = obj.extract()?;
                let v = v as f32;
                match self.endian {
                    '>' => v.to_be_bytes().to_vec(),
                    '<' => v.to_le_bytes().to_vec(),
                    '=' => v.to_ne_bytes().to_vec(),
                    _ => v.to_be_bytes().to_vec(),
                }
            }
            'd' => {
                let v: f64 = obj.extract()?;
                match self.endian {
                    '>' => v.to_be_bytes().to_vec(),
                    '<' => v.to_le_bytes().to_vec(),
                    '=' => v.to_ne_bytes().to_vec(),
                    _ => v.to_be_bytes().to_vec(),
                }
            }
            _ => Vec::new(),
        };
        Ok(PyBytes::new(py, &bytes))
    }

    fn sizeof(&self) -> PyResult<usize> {
        Ok(self.length)
    }
}

// ========================= Python bindings ==============================

#[pyclass(subclass)]
pub struct Construct {}

#[pymethods]
impl Construct {
    #[new]
    fn new() -> Self {
        Construct {}
    }

    /// Parse bytes from memory. Currently returns the data unchanged.
    fn parse<'py>(&self, py: Python<'py>, data: &PyBytes) -> PyResult<&'py PyBytes> {
        Ok(PyBytes::new(py, data.as_bytes()))
    }

    /// Build an object into bytes. Returns the input bytes.
    fn build<'py>(&self, py: Python<'py>, obj: &PyBytes) -> PyResult<&'py PyBytes> {
        Ok(PyBytes::new(py, obj.as_bytes()))
    }

    /// Parse entire contents of a file.
    fn parse_file<'py>(&self, py: Python<'py>, filename: &str) -> PyResult<&'py PyBytes> {
        let data = std::fs::read(filename).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;
        Ok(PyBytes::new(py, &data))
    }

    /// Build bytes into a file.
    fn build_file(&self, filename: &str, data: &PyBytes) -> PyResult<()> {
        std::fs::write(filename, data.as_bytes()).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
    }
}

/// A wrapper around another `Construct`-like object.
#[pyclass(extends=Construct)]
pub struct Subconstruct {
    subcon: Py<PyAny>,
}

#[pymethods]
impl Subconstruct {
    #[new]
    fn new(subcon: Py<PyAny>) -> (Self, Construct) {
        (Subconstruct { subcon }, Construct {})
    }

    /// Delegate parsing to the wrapped construct.
    fn parse<'py>(&self, py: Python<'py>, data: &PyBytes) -> PyResult<&'py PyBytes> {
        let res = self.subcon.as_ref(py).call_method1("parse", (data,))?;
        res.extract()
    }

    /// Delegate building to the wrapped construct.
    fn build<'py>(&self, py: Python<'py>, obj: &PyBytes) -> PyResult<&'py PyBytes> {
        let res = self.subcon.as_ref(py).call_method1("build", (obj,))?;
        res.extract()
    }

    /// Delegate file parsing to the wrapped construct.
    fn parse_file<'py>(&self, py: Python<'py>, filename: &str) -> PyResult<&'py PyBytes> {
        let res = self.subcon.as_ref(py).call_method1("parse_file", (filename,))?;
        res.extract()
    }

    /// Delegate file building to the wrapped construct.
    fn build_file(&self, py: Python, filename: &str, data: &PyBytes) -> PyResult<()> {
        self.subcon.as_ref(py).call_method1("build_file", (filename, data))?;
        Ok(())
    }
}

#[pymodule]
fn construct_rs(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Construct>()?;
    m.add_class::<Subconstruct>()?;
    m.add_class::<BitsInteger>()?;
    m.add_class::<BytesInteger>()?;
    m.add_class::<FormatField>()?;

    let bit = Py::new(py, (BitsInteger { length: 1, signed: false, swapped: false }, Construct {}))?;
    m.add("Bit", bit)?;
    let nibble = Py::new(py, (BitsInteger { length: 4, signed: false, swapped: false }, Construct {}))?;
    m.add("Nibble", nibble)?;
    let octet = Py::new(py, (BitsInteger { length: 8, signed: false, swapped: false }, Construct {}))?;
    m.add("Octet", octet)?;

    m.add("Int8ub", Py::new(py, (FormatField { endian: '>', format: 'B', length: 1 }, Construct {}))?)?;
    m.add("Int16ub", Py::new(py, (FormatField { endian: '>', format: 'H', length: 2 }, Construct {}))?)?;
    m.add("Int32ub", Py::new(py, (FormatField { endian: '>', format: 'L', length: 4 }, Construct {}))?)?;
    m.add("Int64ub", Py::new(py, (FormatField { endian: '>', format: 'Q', length: 8 }, Construct {}))?)?;
    m.add("Int8sb", Py::new(py, (FormatField { endian: '>', format: 'b', length: 1 }, Construct {}))?)?;
    m.add("Int16sb", Py::new(py, (FormatField { endian: '>', format: 'h', length: 2 }, Construct {}))?)?;
    m.add("Int32sb", Py::new(py, (FormatField { endian: '>', format: 'l', length: 4 }, Construct {}))?)?;
    m.add("Int64sb", Py::new(py, (FormatField { endian: '>', format: 'q', length: 8 }, Construct {}))?)?;
    m.add("Int8ul", Py::new(py, (FormatField { endian: '<', format: 'B', length: 1 }, Construct {}))?)?;
    m.add("Int16ul", Py::new(py, (FormatField { endian: '<', format: 'H', length: 2 }, Construct {}))?)?;
    m.add("Int32ul", Py::new(py, (FormatField { endian: '<', format: 'L', length: 4 }, Construct {}))?)?;
    m.add("Int64ul", Py::new(py, (FormatField { endian: '<', format: 'Q', length: 8 }, Construct {}))?)?;
    m.add("Int8sl", Py::new(py, (FormatField { endian: '<', format: 'b', length: 1 }, Construct {}))?)?;
    m.add("Int16sl", Py::new(py, (FormatField { endian: '<', format: 'h', length: 2 }, Construct {}))?)?;
    m.add("Int32sl", Py::new(py, (FormatField { endian: '<', format: 'l', length: 4 }, Construct {}))?)?;
    m.add("Int64sl", Py::new(py, (FormatField { endian: '<', format: 'q', length: 8 }, Construct {}))?)?;
    m.add("Int8un", Py::new(py, (FormatField { endian: '=', format: 'B', length: 1 }, Construct {}))?)?;
    m.add("Int16un", Py::new(py, (FormatField { endian: '=', format: 'H', length: 2 }, Construct {}))?)?;
    m.add("Int32un", Py::new(py, (FormatField { endian: '=', format: 'L', length: 4 }, Construct {}))?)?;
    m.add("Int64un", Py::new(py, (FormatField { endian: '=', format: 'Q', length: 8 }, Construct {}))?)?;
    m.add("Int8sn", Py::new(py, (FormatField { endian: '=', format: 'b', length: 1 }, Construct {}))?)?;
    m.add("Int16sn", Py::new(py, (FormatField { endian: '=', format: 'h', length: 2 }, Construct {}))?)?;
    m.add("Int32sn", Py::new(py, (FormatField { endian: '=', format: 'l', length: 4 }, Construct {}))?)?;
    m.add("Int64sn", Py::new(py, (FormatField { endian: '=', format: 'q', length: 8 }, Construct {}))?)?;

    m.add("Byte", m.getattr("Int8ub")?)?;
    m.add("Short", m.getattr("Int16ub")?)?;
    m.add("Int", m.getattr("Int32ub")?)?;
    m.add("Long", m.getattr("Int64ub")?)?;

    m.add("Float32b", Py::new(py, (FormatField { endian: '>', format: 'f', length: 4 }, Construct {}))?)?;
    m.add("Float32l", Py::new(py, (FormatField { endian: '<', format: 'f', length: 4 }, Construct {}))?)?;
    m.add("Float32n", Py::new(py, (FormatField { endian: '=', format: 'f', length: 4 }, Construct {}))?)?;
    m.add("Float64b", Py::new(py, (FormatField { endian: '>', format: 'd', length: 8 }, Construct {}))?)?;
    m.add("Float64l", Py::new(py, (FormatField { endian: '<', format: 'd', length: 8 }, Construct {}))?)?;
    m.add("Float64n", Py::new(py, (FormatField { endian: '=', format: 'd', length: 8 }, Construct {}))?)?;

    m.add("Single", m.getattr("Float32b")?)?;
    m.add("Double", m.getattr("Float64b")?)?;

    let native_le = cfg!(target_endian = "little");
    m.add("Int24ub", Py::new(py, (BytesInteger { length: 3, signed: false, swapped: false }, Construct {}))?)?;
    m.add("Int24ul", Py::new(py, (BytesInteger { length: 3, signed: false, swapped: true }, Construct {}))?)?;
    m.add("Int24un", Py::new(py, (BytesInteger { length: 3, signed: false, swapped: native_le }, Construct {}))?)?;
    m.add("Int24sb", Py::new(py, (BytesInteger { length: 3, signed: true, swapped: false }, Construct {}))?)?;
    m.add("Int24sl", Py::new(py, (BytesInteger { length: 3, signed: true, swapped: true }, Construct {}))?)?;
    m.add("Int24sn", Py::new(py, (BytesInteger { length: 3, signed: true, swapped: native_le }, Construct {}))?)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use pyo3::Python;
    use pyo3::types::{PyBytes, PyModule};
    use pyo3::PyAny;

    #[test]
    fn test_stream_helpers() {
        let data = b"abcdef".to_vec();
        let mut cur = Cursor::new(data.clone());
        assert_eq!(stream_size(&mut cur).unwrap(), 6);
        assert!(!stream_iseof(&mut cur).unwrap());
        assert_eq!(stream_read(&mut cur, 3).unwrap(), b"abc");
        let mut out = Cursor::new(Vec::new());
        stream_write(&mut out, b"xyz").unwrap();
        assert_eq!(out.into_inner(), b"xyz".to_vec());

        let mut cur = Cursor::new(data);
        let buf = stream_read_entire(&mut cur).unwrap();
        assert_eq!(buf, b"abcdef");
    }

    #[test]
    fn test_subconstruct_delegation() {
        Python::with_gil(|py| {
            let inner = Py::new(py, Construct {}).unwrap();
            let sub = Py::new(py, (Subconstruct { subcon: inner.clone_ref(py) }, Construct {})).unwrap();
            let data = PyBytes::new(py, b"abc");
            let res: &PyBytes = sub.call_method1(py, "parse", (data,)).unwrap().extract(py).unwrap();
            assert_eq!(res.as_bytes(), b"abc");
            let built: &PyBytes = sub.call_method1(py, "build", (data,)).unwrap().extract(py).unwrap();
            assert_eq!(built.as_bytes(), b"abc");
        });
    }

    #[test]
    fn test_bitsinteger() {
        Python::with_gil(|py| {
            let obj = Py::new(py, (BitsInteger { length: 8, signed: false, swapped: false }, Construct {})).unwrap();
            let data = PyBytes::new(py, &[1u8; 8]);
            let val: i128 = obj.call_method1(py, "parse", (data,)).unwrap().extract(py).unwrap();
            assert_eq!(val, 255);

            let built: &PyBytes = obj.call_method1(py, "build", (255i128,)).unwrap().extract(py).unwrap();
            assert_eq!(built.as_bytes(), &[1u8; 8]);
        });
    }

    #[test]
    fn test_singleton_bits() {
        Python::with_gil(|py| {
            let m = PyModule::new(py, "test").unwrap();
            construct_rs(py, m).unwrap();
            let bit: &PyAny = m.getattr("Bit").unwrap();
            let data = PyBytes::new(py, &[1u8]);
            let val: i128 = bit.call_method1("parse", (data,)).unwrap().extract().unwrap();
            assert_eq!(val, 1);

            let built: &PyBytes = bit.call_method1("build", (1i128,)).unwrap().extract().unwrap();
            assert_eq!(built.as_bytes(), &[1u8]);
        });
    }

    #[test]
    fn test_singleton_ints() {
        Python::with_gil(|py| {
            let m = PyModule::new(py, "test").unwrap();
            construct_rs(py, m).unwrap();
            let int16: &PyAny = m.getattr("Int16ub").unwrap();
            let data = PyBytes::new(py, &[0x01, 0x02]);
            let val: i128 = int16.call_method1("parse", (data,)).unwrap().extract().unwrap();
            assert_eq!(val, 0x0102);
            let built: &PyBytes = int16.call_method1("build", (0x0102i128,)).unwrap().extract().unwrap();
            assert_eq!(built.as_bytes(), &[0x01, 0x02]);
        });
    }

    #[test]
    fn test_singleton_bytesinteger() {
        Python::with_gil(|py| {
            let m = PyModule::new(py, "test").unwrap();
            construct_rs(py, m).unwrap();
            let int24: &PyAny = m.getattr("Int24ub").unwrap();
            let data = PyBytes::new(py, &[0x01, 0x02, 0x03]);
            let val: i128 = int24.call_method1("parse", (data,)).unwrap().extract().unwrap();
            assert_eq!(val, 0x010203);
            let built: &PyBytes = int24.call_method1("build", (0x010203i128,)).unwrap().extract().unwrap();
            assert_eq!(built.as_bytes(), &[0x01, 0x02, 0x03]);
        });
    }
}
