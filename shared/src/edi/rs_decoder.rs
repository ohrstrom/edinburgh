use reed_solomon_erasure::ReedSolomon;

pub struct RSDecoder {
    rs: ReedSolomon<u8>,
    // The Dablin C++ code uses a shortening of 135 bytes for RS(255, 245) to get RS(120, 110).
    // This means the effective message length is 110, and the total codeword length is 120.
    // The shortening offset is 255 - 120 = 135.
    shortening_offset: usize,
}

impl RSDecoder {
    pub fn new() -> Self {
        // Parameters from Dablin C++: RS(120, 110)
        // n = 120 (codeword length)
        // k = 110 (message length)
        // npar = 10 (parity symbols)
        // t = 5 (error correction capability)
        // The generator polynomial is 0x11D for GF(2^8)
        // The reed-solomon-erasure crate uses n and k directly.
        // It also uses a default generator polynomial for GF(2^8) which should be 0x11D.
        let rs = ReedSolomon::new(120, 110).unwrap();
        let shortening_offset = 135; // 255 - 120

        RSDecoder {
            rs,
            shortening_offset,
        }
    }

    // This function will perform the RS decoding and error correction.
    // It takes the superframe data as a mutable slice.
    pub fn decode_superframe(&self, sf_buff: &mut [u8], subch_index: usize) -> (usize, bool) {
        let mut total_corr_count = 0;
        let mut uncorr_errors = false;

        // The C++ code processes 120-byte RS packets.
        // The superframe data is interleaved such that each 120-byte RS packet
        // is formed by taking bytes from sf_buff at intervals of subch_index.
        // For example, the first RS packet is sf_buff[0], sf_buff[subch_index], sf_buff[2*subch_index], ...
        // This is equivalent to transposing the matrix.

        // Create a buffer for a single RS packet (120 bytes)
        let mut rs_packet = vec![0u8; 120];

        // Iterate through each RS packet (there are 'subch_index' such packets)
        for i in 0..subch_index {
            // De-interleave (transpose) to form the RS packet
            for pos in 0..120 {
                rs_packet[pos] = sf_buff[pos * subch_index + i];
            }

            // Decode and correct errors
            let mut codeword = rs_packet.clone(); // Clone to pass to RS decoder
            let result = self.rs.decode(&mut codeword);

            match result {
                Ok(metrics) => {
                    total_corr_count += metrics.errors_corrected;
                    // If errors were corrected, copy the corrected data back to rs_packet
                    if metrics.errors_corrected > 0 {
                        rs_packet.copy_from_slice(&codeword);
                    }
                }
                Err(_) => {
                    // Uncorrectable errors
                    uncorr_errors = true;
                    // In the C++ code, uncorrectable errors mean the packet is not corrected.
                    // So, we don't copy back the codeword.
                }
            }

            // Re-interleave (transpose back) to write corrected data to sf_buff
            // Only if there were no uncorrectable errors for this packet
            if result.is_ok() {
                for pos in 0..120 {
                    sf_buff[pos * subch_index + i] = rs_packet[pos];
                }
            }
        }

        (total_corr_count, uncorr_errors)
    }
}
