use crate::dab::tables::EBU_LATIN_TO_UNICODE;

pub fn decode_chars(chars: &[u8], charset: u8) -> String {
    match charset {
        0xF => String::from_utf8_lossy(chars).to_string(),
        0x4 => chars.iter().map(|&b| b as char).collect(),
        0x0 => chars
            .iter()
            .map(|&b| char::from_u32(EBU_LATIN_TO_UNICODE[b as usize] as u32).unwrap_or('?'))
            .collect(),
        _ => format!("[unsupported charset 0x{:X}]", charset),
    }
}
