//! Deterministic hash/size primitive over a reader.

use std::io::Read;

use sha2::{Digest, Sha256};

use crate::domain::ContentFingerprint;

/// Streams `reader` once and returns the lower-case 64-hex SHA-256 digest
/// together with the exact byte count. Performs no filesystem mutation
/// (design: hash.rs establishes future reconciliation semantics).
pub fn fingerprint<R: Read>(reader: R) -> std::io::Result<ContentFingerprint> {
    let mut hasher = Sha256::new();
    let mut size: u64 = 0;
    let mut buffer = [0u8; 8192];
    let mut reader = reader;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size += read as u64;
    }
    Ok(ContentFingerprint {
        sha256: hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    #[test]
    fn fingerprint_known_vector_abc() {
        let fp = fingerprint(Cursor::new(b"abc")).expect("fingerprint");
        assert_eq!(
            fp.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(fp.size, 3);
        assert_eq!(fp.sha256.len(), 64);
        assert!(
            fp.sha256
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "lowercase 64-hex"
        );
    }

    #[test]
    fn fingerprint_empty_input() {
        let fp = fingerprint(Cursor::new(b"")).expect("fingerprint");
        assert_eq!(
            fp.sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(fp.size, 0);
    }

    #[test]
    fn fingerprint_counts_exact_bytes_across_reads() {
        let data = vec![b'a'; 20_000];
        let fp = fingerprint(Cursor::new(data)).expect("fingerprint");
        assert_eq!(fp.size, 20_000);
        assert_eq!(fp.sha256.len(), 64);
    }

    #[test]
    fn fingerprint_propagates_reader_errors() {
        struct Failing;

        impl std::io::Read for Failing {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("boom"))
            }
        }

        assert!(fingerprint(Failing).is_err());
    }
}
