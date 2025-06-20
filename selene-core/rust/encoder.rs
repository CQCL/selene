use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutputStreamError {
    #[error("IO Error: {0}")]
    IoError(std::io::Error),
    #[error("Empty arrays are not allowed")]
    EmptyArrayError, // A zero-length array of non-string primatives is handled as a single
    // element.
    #[error("Array size {0} exceeds maximum size")]
    OversizeArrayError(usize),
    #[error("String size {0} exceeds maximum size")]
    OversizeStringError(usize),
    #[error("Tag pointer is null")]
    NullTagError,
    #[error("String contains null byte: {0:?}")]
    CorruptedStringError(Box<[u8]>),
    #[error("{0}")]
    OtherError(String),
}

pub trait StreamWritable {
    const TYPE_REPR: u16;
    fn write(&self, stream: &mut dyn Write) -> Result<(), OutputStreamError> {
        stream
            .write_all(&Self::TYPE_REPR.to_le_bytes())
            .map_err(OutputStreamError::IoError)?;
        stream
            .write_all(&self.get_length()?.to_le_bytes())
            .map_err(OutputStreamError::IoError)?;
        self.write_impl(stream)
    }
    fn get_length(&self) -> Result<u16, OutputStreamError>;
    fn write_impl(&self, stream: &mut dyn Write) -> Result<(), OutputStreamError>;
}
impl StreamWritable for &str {
    const TYPE_REPR: u16 = 3;
    fn get_length(&self) -> Result<u16, OutputStreamError> {
        if self.len() > u16::MAX as usize {
            return Err(OutputStreamError::OversizeStringError(self.len()));
        }
        Ok(self.len() as u16)
    }
    fn write_impl(&self, stream: &mut dyn Write) -> Result<(), OutputStreamError> {
        if self.contains('\0') {
            return Err(OutputStreamError::CorruptedStringError(
                self.as_bytes().into(),
            ));
        }
        stream
            .write_all(self.as_bytes())
            .map_err(OutputStreamError::IoError)
    }
}
pub trait StreamWritableSingle {
    const TYPE_REPR: u16;
    fn write_impl(&self, stream: &mut dyn Write) -> Result<(), OutputStreamError>;
}
impl<T: StreamWritableSingle> StreamWritable for T {
    const TYPE_REPR: u16 = T::TYPE_REPR;
    fn get_length(&self) -> Result<u16, OutputStreamError> {
        // by default, we're writing a single value.
        // The encoding we use defines a length of 0 as a single non-array value.
        Ok(0)
    }
    fn write_impl(&self, stream: &mut dyn Write) -> Result<(), OutputStreamError> {
        self.write_impl(stream)
    }
}
impl<T: StreamWritableSingle> StreamWritable for &[T] {
    const TYPE_REPR: u16 = T::TYPE_REPR;
    fn get_length(&self) -> Result<u16, OutputStreamError> {
        if self.is_empty() {
            return Err(OutputStreamError::EmptyArrayError);
        }
        if self.len() > u16::MAX as usize {
            return Err(OutputStreamError::OversizeArrayError(self.len()));
        }
        Ok(self.len() as u16)
    }
    fn write_impl(&self, stream: &mut dyn Write) -> Result<(), OutputStreamError> {
        for item in self.iter() {
            item.write_impl(stream)?;
        }
        Ok(())
    }
}
impl StreamWritableSingle for bool {
    const TYPE_REPR: u16 = 4;
    fn write_impl(&self, stream: &mut dyn Write) -> Result<(), OutputStreamError> {
        let byte = if *self { 1u8 } else { 0u8 };
        stream
            .write_all(&[byte])
            .map_err(OutputStreamError::IoError)
    }
}
impl StreamWritableSingle for u64 {
    const TYPE_REPR: u16 = 1;
    fn write_impl(&self, stream: &mut dyn Write) -> Result<(), OutputStreamError> {
        stream
            .write_all(&self.to_le_bytes())
            .map_err(OutputStreamError::IoError)
    }
}
impl StreamWritableSingle for i64 {
    const TYPE_REPR: u16 = 5;
    fn write_impl(&self, stream: &mut dyn Write) -> Result<(), OutputStreamError> {
        stream
            .write_all(&self.to_le_bytes())
            .map_err(OutputStreamError::IoError)
    }
}
impl StreamWritableSingle for f64 {
    const TYPE_REPR: u16 = 2;
    fn write_impl(&self, stream: &mut dyn Write) -> Result<(), OutputStreamError> {
        stream
            .write_all(&self.to_le_bytes())
            .map_err(OutputStreamError::IoError)
    }
}
impl StreamWritableSingle for u8 {
    const TYPE_REPR: u16 = 9116;
    fn write_impl(&self, stream: &mut dyn Write) -> Result<(), OutputStreamError> {
        stream
            .write_all(&self.to_le_bytes())
            .map_err(OutputStreamError::IoError)
    }
}
pub struct OutputStream {
    writer: Box<dyn Write>,
}
impl OutputStream {
    pub fn new(writer: Box<dyn Write>) -> Self {
        OutputStream { writer }
    }
    fn write_impl(&mut self, value: &[u8]) -> Result<(), OutputStreamError> {
        match self.writer.write_all(value) {
            Ok(_) => Ok(()),
            Err(e) => Err(OutputStreamError::IoError(e)),
        }
    }
    pub fn flush(&mut self) -> Result<(), OutputStreamError> {
        self.writer.flush().map_err(OutputStreamError::IoError)
    }

    pub fn begin_message(&mut self, time_cursor: u64) -> Result<(), OutputStreamError> {
        self.write_impl(&time_cursor.to_le_bytes())
    }
    pub fn end_message(&mut self) -> Result<(), OutputStreamError> {
        self.write_impl(&0u16.to_le_bytes())?;
        self.write_impl(&0u16.to_le_bytes())
    }
    pub fn end_of_stream(&mut self) -> Result<(), OutputStreamError> {
        self.write_impl(&u64::MAX.to_le_bytes())
    }
    // Single element writers
    pub fn write<T: StreamWritable>(&mut self, value: T) -> Result<(), OutputStreamError> {
        value.write(&mut self.writer)
    }
}
impl Drop for OutputStream {
    fn drop(&mut self) {
        if let Err(e) = self.end_of_stream() {
            eprintln!("Error closing output encoder: {}", e);
        }
        if let Err(e) = self.flush() {
            eprintln!("Error flushing output encoder: {}", e);
        }
    }
}
