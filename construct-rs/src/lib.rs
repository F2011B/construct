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

/// Reverse byte order of a bit string.
pub fn swapbytes(mut data: Vec<u8>) -> Vec<u8> {
    data.reverse();
    data
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
fn construct_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Construct>()?;
    m.add_class::<Subconstruct>()?;
    m.add_class::<BitsInteger>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use pyo3::Python;
    use pyo3::types::PyBytes;

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
}
