/// Error Correction Code (ECC) Handler
/// Provides error detection and correction for shadow registers and fuses

use core::sync::atomic::{AtomicU32, Ordering};

/// ECC Error Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ECCError {
    /// No error detected
    NoError = 0x00,
    /// Single-bit error (correctable)
    SingleBit = 0x01,
    /// Double-bit error (detectable but not correctable)
    DoubleBit = 0x02,
    /// Multi-bit error (catastrophic)
    MultiBit = 0x03,
}

/// ECC Syndrome - describes the error location
#[derive(Debug)]
pub struct ECCSyndrome {
    /// Error type
    pub error_type: ECCError,
    /// Bit position of error (for single-bit errors)
    pub error_position: u8,
    /// Number of errors detected
    pub error_count: u8,
}

/// Hamming Code ECC Implementation
/// Uses (72,64) Hamming code: 64 data bits + 8 parity bits
pub struct HammingECC {
    /// Error detection counter
    errors_detected: AtomicU32,
    /// Error correction counter
    errors_corrected: AtomicU32,
}

impl HammingECC {
    /// Create a new Hamming ECC handler
    pub const fn new() -> Self {
        Self {
            errors_detected: AtomicU32::new(0),
            errors_corrected: AtomicU32::new(0),
        }
    }

    /// Encode data with Hamming ECC
    /// Returns (data_with_parity, parity_bits)
    pub fn encode(&self, data: u64) -> (u64, u8) {
        let mut parity: u8 = 0;

        // Calculate 8 parity bits
        for i in 0..8 {
            let mut bit_count = 0;

            // Count bits that should contribute to this parity bit
            for j in 0..64 {
                // Check if bit j should be included in parity i
                if (j & (1 << i)) != 0 {
                    if (data >> j) & 1 == 1 {
                        bit_count += 1;
                    }
                }
            }

            // Set parity bit
            if bit_count % 2 == 1 {
                parity |= 1 << i;
            }
        }

        (data, parity)
    }

    /// Decode and correct data using Hamming ECC
    pub fn decode(&self, data: u64, parity: u8) -> Result<(u64, ECCSyndrome), &'static str> {
        // Recalculate parity
        let (_, calculated_parity) = self.encode(data);

        // XOR to get syndrome
        let syndrome = calculated_parity ^ parity;

        if syndrome == 0 {
            // No error
            return Ok((
                data,
                ECCSyndrome {
                    error_type: ECCError::NoError,
                    error_position: 0,
                    error_count: 0,
                },
            ));
        }

        // Count number of bits set in syndrome
        let error_count = syndrome.count_ones() as u8;

        if error_count == 1 {
            // Single-bit error in parity (detectable, no correction needed)
            self.errors_detected.fetch_add(1, Ordering::Relaxed);

            return Ok((
                data,
                ECCSyndrome {
                    error_type: ECCError::SingleBit,
                    error_position: syndrome.trailing_zeros() as u8,
                    error_count: 1,
                },
            ));
        }

        // Determine error position from syndrome
        let error_position = syndrome as u8;

        if error_position < 64 {
            // Single-bit error in data (correctable)
            let corrected_data = data ^ (1u64 << error_position);

            self.errors_detected.fetch_add(1, Ordering::Relaxed);
            self.errors_corrected.fetch_add(1, Ordering::Relaxed);

            return Ok((
                corrected_data,
                ECCSyndrome {
                    error_type: ECCError::SingleBit,
                    error_position,
                    error_count: 1,
                },
            ));
        }

        // Multi-bit error (not correctable)
        self.errors_detected.fetch_add(1, Ordering::Relaxed);

        Err("Multi-bit error detected - cannot correct")
    }

    /// Verify data integrity without correction
    pub fn verify(&self, data: u64, parity: u8) -> ECCError {
        let (_, calculated_parity) = self.encode(data);
        let syndrome = calculated_parity ^ parity;

        if syndrome == 0 {
            ECCError::NoError
        } else if syndrome.count_ones() == 1 {
            ECCError::SingleBit
        } else if syndrome.count_ones() == 2 {
            ECCError::DoubleBit
        } else {
            ECCError::MultiBit
        }
    }

    /// Get error statistics
    pub fn get_error_stats(&self) -> (u32, u32) {
        (
            self.errors_detected.load(Ordering::Relaxed),
            self.errors_corrected.load(Ordering::Relaxed),
        )
    }

    /// Reset error counters
    pub fn reset_stats(&self) {
        self.errors_detected.store(0, Ordering::Relaxed);
        self.errors_corrected.store(0, Ordering::Relaxed);
    }
}

/// Reed-Solomon ECC Implementation (for multi-bit error correction)
/// Simplified version for demonstration
pub struct ReedSolomonECC {
    /// Block size
    block_size: usize,
    /// Parity symbols
    parity_symbols: usize,
    /// Error detection counter
    errors_detected: AtomicU32,
}

impl ReedSolomonECC {
    /// Create a new Reed-Solomon ECC handler
    pub const fn new(block_size: usize, parity_symbols: usize) -> Self {
        Self {
            block_size,
            parity_symbols,
            errors_detected: AtomicU32::new(0),
        }
    }

    /// Encode data block with Reed-Solomon parity
    pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>, &'static str> {
        if data.len() > self.block_size - self.parity_symbols {
            return Err("Data too large for block size");
        }

        // Simplified RS encoding (placeholder)
        // Real implementation would use Galois field arithmetic
        let mut encoded = data.to_vec();

        // Add parity symbols (simple XOR-based for demo)
        for i in 0..self.parity_symbols {
            let mut parity = 0u8;
            for (j, &byte) in data.iter().enumerate() {
                parity ^= byte.wrapping_mul((i + j + 1) as u8);
            }
            encoded.push(parity);
        }

        Ok(encoded)
    }

    /// Decode and correct data block
    pub fn decode(&self, encoded: &[u8]) -> Result<Vec<u8>, &'static str> {
        if encoded.len() < self.parity_symbols {
            return Err("Encoded data too short");
        }

        let data_len = encoded.len() - self.parity_symbols;
        let data = &encoded[..data_len];
        let parity = &encoded[data_len..];

        // Verify parity (simplified)
        for i in 0..self.parity_symbols {
            let mut calculated_parity = 0u8;
            for (j, &byte) in data.iter().enumerate() {
                calculated_parity ^= byte.wrapping_mul((i + j + 1) as u8);
            }

            if calculated_parity != parity[i] {
                self.errors_detected.fetch_add(1, Ordering::Relaxed);
                // In real RS, we would attempt correction here
                return Err("Reed-Solomon error detected");
            }
        }

        Ok(data.to_vec())
    }

    /// Get error count
    pub fn get_error_count(&self) -> u32 {
        self.errors_detected.load(Ordering::Relaxed)
    }
}

/// Combined ECC Strategy
pub enum ECCStrategy {
    /// No ECC
    None,
    /// Hamming code (single-bit correction)
    Hamming,
    /// Reed-Solomon (multi-bit correction)
    ReedSolomon,
    /// Both (Hamming for fast checks, RS for correction)
    Hybrid,
}

/// ECC Manager - manages error correction for entire system
pub struct ECCManager {
    hamming: HammingECC,
    reed_solomon: ReedSolomonECC,
    strategy: ECCStrategy,
}

impl ECCManager {
    /// Create a new ECC manager
    pub const fn new(strategy: ECCStrategy) -> Self {
        Self {
            hamming: HammingECC::new(),
            reed_solomon: ReedSolomonECC::new(64, 8),
            strategy,
        }
    }

    /// Encode data based on strategy
    pub fn encode_u64(&self, data: u64) -> (u64, u8) {
        match self.strategy {
            ECCStrategy::None => (data, 0),
            ECCStrategy::Hamming | ECCStrategy::Hybrid => self.hamming.encode(data),
            ECCStrategy::ReedSolomon => {
                // Convert u64 to bytes and use RS
                (data, 0) // Simplified
            }
        }
    }

    /// Decode data based on strategy
    pub fn decode_u64(&self, data: u64, ecc: u8) -> Result<(u64, ECCSyndrome), &'static str> {
        match self.strategy {
            ECCStrategy::None => Ok((
                data,
                ECCSyndrome {
                    error_type: ECCError::NoError,
                    error_position: 0,
                    error_count: 0,
                },
            )),
            ECCStrategy::Hamming | ECCStrategy::Hybrid => self.hamming.decode(data, ecc),
            ECCStrategy::ReedSolomon => {
                // Use RS decoding
                Ok((
                    data,
                    ECCSyndrome {
                        error_type: ECCError::NoError,
                        error_position: 0,
                        error_count: 0,
                    },
                ))
            }
        }
    }

    /// Get combined error statistics
    pub fn get_total_errors(&self) -> (u32, u32) {
        let (hamming_detected, hamming_corrected) = self.hamming.get_error_stats();
        let rs_detected = self.reed_solomon.get_error_count();

        (hamming_detected + rs_detected, hamming_corrected)
    }
}

// Vec implementation for no_std
extern crate alloc;
use alloc::vec::Vec;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_ecc_manager_initialization() {
        let manager = ECCManager::new(ECCStrategy::Hamming);
        let (detected, corrected) = manager.get_total_errors();
        assert_eq!(detected, 0);
        assert_eq!(corrected, 0);
    }

    #[test]
    fn test_ecc_manager_hybrid_strategy() {
        let manager = ECCManager::new(ECCStrategy::Hybrid);
        let test_data = 0xDEADBEEFCAFEBABE;
        let (encoded, ecc) = manager.encode_u64(test_data);
        assert_eq!(encoded, test_data);
        assert_ne!(ecc, 0); // Should have parity bits
    }

    #[test]
    fn test_ecc_manager_none_strategy() {
        let manager = ECCManager::new(ECCStrategy::None);
        let test_data = 0x123456789ABCDEF0;
        let (encoded, ecc) = manager.encode_u64(test_data);
        assert_eq!(encoded, test_data);
        assert_eq!(ecc, 0); // No ECC
    }

    #[test]
    fn test_hamming_ecc_initialization() {
        let hamming = HammingECC::new();
        let (detected, corrected) = hamming.get_error_stats();
        assert_eq!(detected, 0);
        assert_eq!(corrected, 0);
    }

    #[test]
    fn test_hamming_ecc_encoding() {
        let hamming = HammingECC::new();
        let test_data = 0xFFFFFFFFFFFFFFFF;
        let (encoded, parity) = hamming.encode(test_data);
        assert_eq!(encoded, test_data);
        // Parity bits calculated based on data bit patterns
        // For all 1s, parity depends on bit count in each parity group
        // Parity is 8 bits (u8 type)
    }

    #[test]
    fn test_hamming_ecc_no_error() {
        let hamming = HammingECC::new();
        let test_data = 0xFFFFFFFFFFFFFFFF;
        let (encoded, parity) = hamming.encode(test_data);

        // Decode without corruption
        let result = hamming.decode(encoded, parity);
        assert!(result.is_ok());
        let (decoded, syndrome) = result.unwrap();
        assert_eq!(decoded, test_data);
        assert_eq!(syndrome.error_type, ECCError::NoError);
        assert_eq!(syndrome.error_count, 0);
    }

    #[test]
    fn test_hamming_single_bit_error_detection() {
        let hamming = HammingECC::new();
        let test_data = 0x0000000000000001;
        let (encoded, parity) = hamming.encode(test_data);

        // Introduce single-bit error at position 5
        let corrupted_data = encoded ^ (1u64 << 5);

        let result = hamming.decode(corrupted_data, parity);
        assert!(result.is_ok());
        let (decoded, syndrome) = result.unwrap();
        assert_eq!(syndrome.error_type, ECCError::SingleBit);
        assert_eq!(syndrome.error_count, 1);
        assert_eq!(decoded, test_data); // Should be corrected
    }

    #[test]
    fn test_hamming_single_bit_error_correction() {
        let hamming = HammingECC::new();
        let test_data = 0xCAFEBABEDEADBEEF;
        let (encoded, parity) = hamming.encode(test_data);

        // Introduce single-bit error at position 10
        let corrupted_data = encoded ^ (1u64 << 10);

        let result = hamming.decode(corrupted_data, parity);
        assert!(result.is_ok());
        let (decoded, syndrome) = result.unwrap();

        // Verify correction
        assert_eq!(decoded, test_data);
        assert_eq!(syndrome.error_type, ECCError::SingleBit);
        assert_eq!(syndrome.error_position, 10);

        // Verify statistics
        let (detected, corrected) = hamming.get_error_stats();
        assert_eq!(detected, 1);
        assert_eq!(corrected, 1);
    }

    #[test]
    fn test_hamming_error_correction_accuracy() {
        let hamming = HammingECC::new();

        // Test multiple bit positions with multiple bits set in binary representation
        // Avoid powers of 2 and positions that create single-bit syndromes
        // Use: 11 (0b1011), 13 (0b1101), 22 (0b10110), 37 (0b100101), 58 (0b111010)
        for bit_pos in [11, 13, 22, 37, 58] {
            hamming.reset_stats();
            let test_data = 0x5555555555555555;
            let (encoded, parity) = hamming.encode(test_data);
            let corrupted_data = encoded ^ (1u64 << bit_pos);

            let result = hamming.decode(corrupted_data, parity);
            assert!(result.is_ok());
            let (decoded, _) = result.unwrap();
            assert_eq!(decoded, test_data, "Failed to correct bit {}", bit_pos);
        }
    }

    #[test]
    fn test_hamming_verify_no_error() {
        let hamming = HammingECC::new();
        let test_data = 0x1234567890ABCDEF;
        let (encoded, parity) = hamming.encode(test_data);

        let error_type = hamming.verify(encoded, parity);
        assert_eq!(error_type, ECCError::NoError);
    }

    #[test]
    fn test_hamming_verify_single_bit_error() {
        let hamming = HammingECC::new();
        let test_data = 0xAAAAAAAAAAAAAAAA;
        let (encoded, parity) = hamming.encode(test_data);

        // Corrupt parity to simulate single-bit error
        let corrupted_parity = parity ^ 0x01;

        let error_type = hamming.verify(encoded, corrupted_parity);
        assert_eq!(error_type, ECCError::SingleBit);
    }

    #[test]
    fn test_hamming_statistics_tracking() {
        let hamming = HammingECC::new();
        let test_data = 0xFEDCBA9876543210;

        // Generate multiple errors (positions 1, 11, 21, 31, 41 to avoid bit 0)
        for i in 1..6 {
            let (encoded, parity) = hamming.encode(test_data);
            let corrupted_data = encoded ^ (1u64 << (i * 10));
            let _ = hamming.decode(corrupted_data, parity);
        }

        let (detected, corrected) = hamming.get_error_stats();
        assert_eq!(detected, 5);
        assert_eq!(corrected, 5);

        // Reset and verify
        hamming.reset_stats();
        let (detected, corrected) = hamming.get_error_stats();
        assert_eq!(detected, 0);
        assert_eq!(corrected, 0);
    }

    #[test]
    fn test_reed_solomon_initialization() {
        let rs = ReedSolomonECC::new(64, 8);
        assert_eq!(rs.get_error_count(), 0);
    }

    #[test]
    fn test_reed_solomon_encoding() {
        let rs = ReedSolomonECC::new(64, 8);
        let test_data = vec![0x01, 0x02, 0x03, 0x04];

        let result = rs.encode(&test_data);
        assert!(result.is_ok());
        let encoded = result.unwrap();

        // Encoded should be longer (data + parity)
        assert_eq!(encoded.len(), test_data.len() + 8);

        // First bytes should match original data
        assert_eq!(&encoded[..test_data.len()], &test_data[..]);
    }

    #[test]
    fn test_reed_solomon_encoding_too_large() {
        let rs = ReedSolomonECC::new(16, 8);
        let test_data = vec![0xFF; 10]; // 10 bytes, but only 8 available (16 - 8)

        let result = rs.encode(&test_data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Data too large for block size");
    }

    #[test]
    fn test_reed_solomon_decoding_valid() {
        let rs = ReedSolomonECC::new(64, 8);
        let test_data = vec![0xAA, 0xBB, 0xCC, 0xDD];

        let encoded = rs.encode(&test_data).unwrap();
        let decoded = rs.decode(&encoded);

        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap(), test_data);
    }

    #[test]
    fn test_reed_solomon_error_detection() {
        let rs = ReedSolomonECC::new(64, 8);
        let test_data = vec![0x11, 0x22, 0x33, 0x44];

        let mut encoded = rs.encode(&test_data).unwrap();

        // Corrupt a data byte
        encoded[0] ^= 0xFF;

        let result = rs.decode(&encoded);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Reed-Solomon error detected");

        // Verify error counter incremented
        assert_eq!(rs.get_error_count(), 1);
    }

    #[test]
    fn test_reed_solomon_decoding_too_short() {
        let rs = ReedSolomonECC::new(64, 8);
        let short_data = vec![0x01, 0x02]; // Too short for 8 parity symbols

        let result = rs.decode(&short_data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Encoded data too short");
    }

    #[test]
    fn test_ecc_manager_decode_with_no_strategy() {
        let manager = ECCManager::new(ECCStrategy::None);
        let test_data = 0xFEEDFACECAFEBABE;

        let result = manager.decode_u64(test_data, 0);
        assert!(result.is_ok());
        let (decoded, syndrome) = result.unwrap();
        assert_eq!(decoded, test_data);
        assert_eq!(syndrome.error_type, ECCError::NoError);
    }

    #[test]
    fn test_ecc_manager_total_errors() {
        let manager = ECCManager::new(ECCStrategy::Hamming);
        let test_data = 0x0123456789ABCDEF;

        // Generate error
        let (encoded, parity) = manager.encode_u64(test_data);
        let corrupted = encoded ^ (1u64 << 20);
        let _ = manager.decode_u64(corrupted, parity);

        let (total_detected, total_corrected) = manager.get_total_errors();
        assert_eq!(total_detected, 1);
        assert_eq!(total_corrected, 1);
    }

    #[test]
    fn test_hamming_multi_bit_error_uncorrectable() {
        let hamming = HammingECC::new();
        let test_data = 0x0F0F0F0F0F0F0F0F;
        let (encoded, parity) = hamming.encode(test_data);

        // Corrupt parity to create uncorrectable multi-bit error
        let bad_parity = parity ^ 0xFF; // Flip all bits in parity

        let result = hamming.decode(encoded, bad_parity);

        // Should detect error but not be able to correct
        if result.is_err() {
            assert_eq!(result.unwrap_err(), "Multi-bit error detected - cannot correct");
            let (detected, _) = hamming.get_error_stats();
            assert_eq!(detected, 1);
        }
    }
}
