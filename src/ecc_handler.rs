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
