use std::collections::HashMap;
use std::io::{self, Read, Write, Seek, SeekFrom};

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
