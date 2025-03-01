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
