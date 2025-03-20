pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

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
    let gen_polynom: u16 = 0x782F;
    let mut crc: u16 = 0;

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

    crc
}
