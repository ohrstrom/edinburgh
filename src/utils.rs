// CRC-16 CCITT
pub fn calc_crc16_ccitt(data: &[u8]) -> u16 {
    let initial_invert = true;
    let final_invert = true;
    let gen_polynom: u16 = 0x1021;

    let mut crc: u16 = if initial_invert { 0xFFFF } else { 0x0000 };

    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ gen_polynom;
            } else {
                crc <<= 1;
            }
        }
    }

    if final_invert {
        crc ^= 0xFFFF;
    }

    crc
}

// CRC-16 FIRE CODE
pub fn calc_crc_fire_code(data: &[u8]) -> u16 {
    let gen_polynom: u16 = 0x782F; // FIRE CODE polynomial
    let mut crc: u16 = 0; // No initial inversion

    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ gen_polynom;
            } else {
                crc <<= 1;
            }
        }
    }

    crc // No final inversion
}

pub fn is_aac(input: &[u8]) -> bool {
    // Check if we have at least 7 bytes (minimum ADTS header size)
    if input.len() < 7 {
        eprintln!("AAC: Not enough bytes for AAC header");
        return false;
    }

    // Check for the ADTS sync word (12 bits)
    if input[0] != 0xFF || (input[1] & 0xF0) != 0xF0 {
        eprintln!("AAC: Invalid ADTS sync word: {:04X} - {:04X}", input[0], input[1]);
        return false;
    }

    let layer = (input[1] & 0x06) >> 1;
    if layer != 0 {
        // Layer must be '00' for AAC
        return false;
    }

    // Check profile (2 bits)
    let profile = (input[2] & 0xC0) >> 6;
    if profile == 3 {
        // '11' is reserved
        return false;
    }

    // Check sampling frequency index (4 bits)
    let sampling_freq_index = (input[2] & 0x3C) >> 2;
    if sampling_freq_index > 11 {
        // Valid range is 0-11
        return false;
    }

    // All checks passed
    true
}