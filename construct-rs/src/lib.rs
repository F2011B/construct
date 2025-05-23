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

fn construct_error(py: Python<'_>, name: &str, msg: &str) -> PyErr {
    let module = py.import("construct.core").expect("construct.core must be importable");
    let exc = module.getattr(name).expect("exception not found");
    PyErr::from_type(exc, msg.to_string())
}

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

// ========================= Integer helpers ==============================

pub fn swapbytes(data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    out.reverse();
    out
}

// ========================= Python bindings ==============================

#[pyclass]
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

#[pyclass]
pub struct BitsInteger {
    length: Option<usize>,
    signed: bool,
    swapped: bool,
}

#[pymethods]
impl BitsInteger {
    #[new]
    #[pyo3(signature=(length, signed=false, swapped=false))]
    fn new(length: &PyAny, signed: bool, swapped: bool) -> Self {
        let len = length.extract::<usize>().ok();
        BitsInteger { length: len, signed, swapped }
    }

    fn parse<'py>(&self, py: Python<'py>, data: &PyBytes) -> PyResult<PyObject> {
        let length = match self.length {
            Some(l) => l,
            None => return Err(construct_error(py, "SizeofError", "cannot calculate size, key not found in context")),
        };
        let mut bits = data.as_bytes();
        if bits.len() != length {
            return Err(construct_error(py, "StreamError", "stream read less than specified amount"));
        }
        let mut vec = bits.to_vec();
        if self.swapped {
            if length % 8 != 0 {
                return Err(construct_error(py, "IntegerError", "little-endianness is only defined for multiples of 8 bits"));
            }
            vec = swapbytes(&vec);
        }
        let module = py.import("construct.lib.binary")?;
        let func = module.getattr("bits2integer")?;
        let obj = func.call1((PyBytes::new(py, &vec), self.signed))?;
        Ok(obj.into())
    }

    fn build<'py>(&self, py: Python<'py>, obj: &PyAny) -> PyResult<&'py PyBytes> {
        let length = match self.length {
            Some(l) => l,
            None => return Err(construct_error(py, "SizeofError", "cannot calculate size, key not found in context")),
        };
        let text = obj.str()?.to_str()?;
        if text.starts_with('-') && !self.signed {
            return Err(construct_error(py, "IntegerError", "value is negative, but field is not signed"));
        }
        let module = py.import("construct.lib.binary")?;
        let func = module.getattr("integer2bits")?;
        let bits_py = func.call1((obj, length))?;
        let mut bits: Vec<u8> = bits_py.extract()?;
        if self.swapped {
            if length % 8 != 0 {
                return Err(construct_error(py, "IntegerError", "little-endianness is only defined for multiples of 8 bits"));
            }
            bits = swapbytes(&bits);
        }
        Ok(PyBytes::new(py, &bits))
    }

    fn sizeof(&self, py: Python) -> PyResult<usize> {
        match self.length {
            Some(l) => Ok(l),
            None => Err(construct_error(py, "SizeofError", "cannot calculate size, key not found in context")),
        }
    }
}

#[pymodule]
fn construct_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Construct>()?;
    m.add_class::<BitsInteger>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
}
