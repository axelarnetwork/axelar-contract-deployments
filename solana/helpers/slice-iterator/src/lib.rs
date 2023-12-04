use std::io::{self, Write};
use std::mem::size_of;

/// Iterates over a slice of bytes while yielding segments of variable length.
/// Each slice is prefixed with its length as a 16-bit unsigned integer.
#[derive(Debug)]
pub struct SliceIterator<'a> {
    input: &'a [u8],
    cursor: u16,
}
const STEP: usize = size_of::<u16>();

impl<'a> SliceIterator<'a> {
    /// Creates a new `SliceIterator` for the given byte slice.
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, cursor: 0 }
    }

    /// Generates an `IterationError` with specific `InputError` and current cursor position.
    fn error(&self, kind: InputError) -> IterationError {
        IterationError {
            kind,
            position: self.cursor,
        }
    }

    pub fn rest(self) -> &'a [u8] {
        &self.input[self.cursor as usize..]
    }
}

/// Error returnd by `SliceIterator`, detailing the error type and the position in the
/// byte slice where it occurred.
#[derive(Debug)]
#[allow(unused)]
pub struct IterationError {
    kind: InputError,
    position: u16,
}

#[derive(Debug)]
/// Possible types of error that can occur during iteration.
pub enum InputError {
    TooSmall,
    MissingSizePrefix,
    InvalidSizePrefix,
    ContentLenghtMissmatch,
}

impl<'a> Iterator for SliceIterator<'a> {
    type Item = Result<&'a [u8], IterationError>;

    /// Advances the iterator and returns the next result.
    ///
    /// Returns `Ok(&[u8])` where `slice` is the next segment of the input or `Err(IterationError)`
    /// if an error occurs.
    fn next(&mut self) -> Option<Self::Item> {
        use InputError::*;
        let rest = &self.input[self.cursor as usize..];
        if rest.is_empty() {
            return None;
        }
        if rest.len() < STEP {
            return Some(Err(self.error(MissingSizePrefix)));
        }
        let (chunk_size_bytes, rest) = rest.split_at(STEP);
        // Unwrap: We just checked that `rest` contains at least two bytes.
        let chunk_size_bytes = chunk_size_bytes.try_into().unwrap();
        let chunk_size = u16::from_be_bytes(chunk_size_bytes);

        let chunk = &rest[0..chunk_size as usize];
        if chunk.len() != chunk_size as usize {
            return Some(Err(self.error(ContentLenghtMissmatch)));
        }

        self.cursor += STEP as u16 + chunk_size;
        Some(Ok(chunk))
    }
}

/// Deserializes an encoded byte slice into a vector of sub-slices.
///
/// This function expects the input byte slice to be formatted as a sequence of sub-slices,
/// where each sub-slice is prefixed with its length encoded as a big-endian `u16`.
pub fn deserialize_slices(src: &[u8]) -> Result<Vec<&[u8]>, IterationError> {
    SliceIterator::new(src).collect()
}

/// Serializes slices into a writer.
///
/// Each slice is prefixed with its length encoded as a big-endian `u16`.
pub fn serialize_slices<W: Write>(src: &[&[u8]], writer: &mut W) -> io::Result<()> {
    for &value in src {
        writer.write_all(&(value.len() as u16).to_be_bytes())?;
        writer.write_all(value)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterator_round_trip() {
        let a = &[1, 2, 3, 4, 5];
        let b = &[5, 6, 7, 8, 9, 10];
        let c: &[u8] = &[];
        let d = &[11, 12, 13];
        let data: Vec<&[u8]> = vec![a, b, c, d];

        let mut buff = vec![];
        serialize_slices(&data, &mut buff).unwrap();
        let des = deserialize_slices(&buff).unwrap();

        assert_eq!(data, des);
    }

    #[test]
    fn serialize_slices_function() {
        let data: &[&[u8]] = &[&[12, 13]];
        let mut buffer = vec![];
        serialize_slices(data, &mut buffer).unwrap();
        assert_eq!(u16::from_be_bytes([buffer[0], buffer[1]]), 2); // sub-slice size
        assert_eq!(buffer[2], 12); // sub-slice element
        assert_eq!(buffer[3], 13); // sub-slice element
        assert!(buffer.get(4).is_none());
    }

    #[test]
    fn serialize_slices_function_two_subslices() {
        let a = &[20, 21];
        let b = &[30, 31, 32];
        let data: &[&[u8]] = &[a, b];
        let mut buffer = vec![];
        serialize_slices(data, &mut buffer).unwrap();
        assert_eq!(u16::from_be_bytes([buffer[0], buffer[1]]), 2); // first sub-slice size
        assert_eq!(buffer[2], 20); // first sub-slice, first element
        assert_eq!(buffer[3], 21); // second sub-slice, second element
        assert_eq!(u16::from_be_bytes([buffer[4], buffer[5]]), 3); // second sub-slice size
        assert_eq!(buffer[6], 30); // second sub-slice, first element
        assert_eq!(buffer[7], 31); // second sub-slice, second element
        assert_eq!(buffer[8], 32); // secont sub-slice, third element
        assert!(buffer.get(9).is_none());
        // round-trip
        assert_eq!(deserialize_slices(&buffer).unwrap(), data);
    }

    #[test]
    fn serialize_empty_sub_slice() {
        let data: &[&[u8]] = &[&[]];
        let mut buffer = vec![];
        serialize_slices(data, &mut buffer).unwrap();
        assert_eq!(u16::from_be_bytes([buffer[0], buffer[1]]), 0); // first sub-slice size
        assert!(buffer.get(3).is_none());
        // round-trip
        assert_eq!(deserialize_slices(&buffer).unwrap(), data);
    }

    #[test]
    fn serialize_empty_slice() {
        let data: &[&[u8]] = &[];
        let mut buffer = vec![];
        serialize_slices(data, &mut buffer).unwrap();
        assert!(buffer.is_empty());
        // round-trip
        assert_eq!(deserialize_slices(&buffer).unwrap(), data);
    }
}
