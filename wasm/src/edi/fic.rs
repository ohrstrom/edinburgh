use crate::utils;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Serialize)]
pub struct Fig0 {
    cn: bool,
    oe: bool,
    pd: bool,
    ext: u8,
}

#[derive(Debug, Serialize)]
pub struct Fig1 {
    charset: u8,
    oe: bool,
    ext: u8,
}

// FIG 0s
#[derive(Debug, Serialize)]
pub struct Fig0_0 {
    base: Fig0,
    pub eid: u16,
    pub al_flag: bool,
}
impl Fig0_0 {
    // FIG 0/0 - Ensemble information (MCI)
    // EID and alarm flag only
    pub fn from_bytes(base: Fig0, data: &[u8]) -> Result<Self, FIGError> {
        if data.len() < 4 {
            return Err(FIGError::InvalidSize { l: data.len() });
        }

        // Extract 16-bit Ensemble ID (Big-Endian)
        let eid = u16::from_be_bytes([data[0], data[1]]);

        // Extract alarm flag (bit 5 of data[2])
        let al_flag = (data[2] & 0x20) != 0;

        // log::debug!("FIG0/0: EID: 0x{:04X}, AL: {}", eid, al_flag);

        Ok(Self { base, eid, al_flag })
    }
}

#[derive(Debug, Serialize)]
pub struct Fig0_1 {
    base: Fig0,
    pub subchannels: Vec<Subchannel>,
}

#[derive(Debug, Serialize)]
pub struct Subchannel {
    pub id: u8,
    pub start: usize,
    pub size: Option<usize>,
    pub pl: Option<String>,
    pub bitrate: Option<usize>,
}

impl Fig0_1 {
    pub fn from_bytes(base: Fig0, data: &[u8]) -> Result<Self, FIGError> {
        let mut offset = 0;
        let mut subchannels = Vec::new();

        while offset < data.len() {
            if offset + 2 > data.len() {
                return Err(FIGError::InvalidSize { l: data.len() });
            }

            let id = data[offset] >> 2;
            let start = ((data[offset] & 0x03) as usize) << 8 | data[offset + 1] as usize;
            offset += 2;

            let mut size = None;
            let mut pl = None;
            let mut bitrate = None;

            let short_long_form = data.get(offset).map(|&b| b & 0x80 != 0).unwrap_or(false);

            if short_long_form {
                // Long form
                if offset + 1 >= data.len() {
                    return Err(FIGError::InvalidSize { l: data.len() });
                }

                let option = (data[offset] & 0x70) >> 4;
                let pl_index = (data[offset] & 0x0C) >> 2;
                let subch_size = ((data[offset] & 0x03) as usize) << 8 | data[offset + 1] as usize;
                offset += 2;

                match option {
                    0b000 => {
                        size = Some(subch_size);
                        pl = Some(format!("EEP {}-A", pl_index + 1));
                        bitrate = Some(subch_size / EEP_A_SIZE_FACTORS[pl_index as usize] * 8);
                    }
                    0b001 => {
                        size = Some(subch_size);
                        pl = Some(format!("EEP {}-B", pl_index + 1));
                        bitrate = Some(subch_size / EEP_B_SIZE_FACTORS[pl_index as usize] * 32);
                    }
                    _ => {}
                }
            } else {
                // Short form
                let table_switch = data.get(offset).map(|&b| b & 0x40 != 0).unwrap_or(false);
                if !table_switch {
                    let table_index = (data[offset] & 0x3F) as usize;
                    if table_index < UEP_SIZES.len() {
                        size = Some(UEP_SIZES[table_index]);
                        pl = Some(format!("UEP {}", UEP_PLS[table_index]));
                        bitrate = Some(UEP_BITRATES[table_index]);
                    }
                }
                offset += 1;
            }

            // Ignore sc_id > 30
            if id <= 30 {
                subchannels.push(Subchannel {
                    id,
                    start,
                    size,
                    pl,
                    bitrate,
                });
            }
        }

        Ok(Self { base, subchannels })
    }
}

#[derive(Debug, Serialize)]
pub struct Fig0_2 {
    base: Fig0,
    pub services: Vec<ServiceComponent>,
}

#[derive(Debug, Serialize)]
pub struct ServiceComponent {
    pub sid: u16,
    pub tmid: u8,
    pub scid: u8,
    pub primary: bool,
    pub ca: bool,
}

impl Fig0_2 {
    // FIG 0/2 - Service organization (MCI)
    pub fn from_bytes(base: Fig0, data: &[u8]) -> Result<Self, FIGError> {
        let mut offset = 0;
        let mut services = Vec::new();

        while offset + 2 <= data.len() {
            // Extract Service ID (SID) - first two bytes
            let sid = u16::from_be_bytes([data[offset], data[offset + 1]]);
            offset += 2;

            // Check remaining bytes
            if offset >= data.len() {
                return Err(FIGError::InvalidSize { l: data.len() });
            }

            let num_components = data[offset] & 0x0F; // Number of service components
            offset += 1;

            for _ in 0..num_components {
                if offset + 1 >= data.len() {
                    return Err(FIGError::InvalidSize { l: data.len() });
                }

                let tmid = (data[offset] & 0xC0) >> 6; // Transport Mechanism ID
                let ascty = data[offset] & 0x3F; // Audio Service Type (ignored)
                let scid = data[offset + 1] >> 2; // Subchannel ID
                let primary = (data[offset + 1] & 0x02) != 0; // Primary component flag
                let ca = (data[offset + 1] & 0x01) != 0; // Conditional Access flag
                offset += 2;

                // astci  0: DAB
                // ascti 63: DAB+
                // log::debug!("ASCTI: {}", ascty);

                // Ignore CA components
                if !ca {
                    services.push(ServiceComponent {
                        sid,
                        tmid,
                        scid,
                        primary,
                        ca,
                    });

                    // log::debug!("FIG0/2: SID: 0x{:04X}, TMID: {}, scid: {}, Primary: {}, CA: {}",
                    //            sid, tmid, scid, primary, ca);
                }
            }
        }

        Ok(Self { base, services })
    }
}

#[derive(Debug, Serialize)]
pub struct Fig0_3 {
    base: Fig0,
    pub sid: u16,
    pub scids: u8,
    pub scid: u8,
}
impl Fig0_3 {
    // FIG 0/3 - Service component in packet mode (MCI)
    pub fn from_bytes(base: Fig0, data: &[u8]) -> Result<Self, FIGError> {
        if data.len() < 3 {
            return Err(FIGError::InvalidSize { l: data.len() });
        }

        // Extract Service ID (SID) - first two bytes
        let sid = u16::from_be_bytes([data[0], data[1]]);
        // Extract Service Component ID (SCIdS) - upper 4 bits of byte 2
        let scids = (data[2] & 0xF0) >> 4;
        // Extract Subchannel ID - lower 6 bits of byte 2
        let scid = data[2] & 0x3F;

        // log::debug!("FIG0/3: SID: 0x{:04X}, SCIdS: {}, scid: {}", sid, scids, scid);

        Ok(Self {
            base,
            sid,
            scids,
            scid,
        })
    }
}

// FIG 1s
#[derive(Debug, Serialize)]
pub struct Fig1_0 {
    base: Fig1,
    pub eid: u16,
    pub label: String,
    pub short_label: String,
}
impl Fig1_0 {
    pub fn from_bytes(base: Fig1, data: &[u8]) -> Result<Self, FIGError> {
        if data.len() < 18 {
            return Err(FIGError::InvalidSize { l: data.len() });
        }

        let eid = u16::from_be_bytes([data[0], data[1]]);
        let label = Self::convert_label_to_utf8(&data[2..18]);
        let short_label =
            Self::derive_short_label(&label, u16::from_be_bytes([data[16], data[17]]));

        Ok(Self {
            base,
            eid,
            label,
            short_label,
        })
    }

    fn convert_label_to_utf8(data: &[u8]) -> String {
        String::from_utf8_lossy(data).trim_end().to_string()
    }

    fn derive_short_label(label: &str, mask: u16) -> String {
        label.to_string()
    }
}

#[derive(Debug, Serialize)]
pub struct Fig1_1 {
    base: Fig1,
    pub sid: u16,
    pub label: String,
    pub short_label: String,
}

impl Fig1_1 {
    pub fn from_bytes(base: Fig1, data: &[u8]) -> Result<Self, FIGError> {
        if data.len() < 18 {
            return Err(FIGError::InvalidSize { l: data.len() });
        }

        let sid = u16::from_be_bytes([data[0], data[1]]);
        let label_bytes = &data[2..18];
        let label = Self::label_str(label_bytes);
        let short_label =
            Self::short_label_str(label_bytes, u16::from_be_bytes([data[18], data[19]]));

        // let (label, short_label) = Self::decode_label(&data[2..19]);

        Ok(Self {
            base,
            sid,
            label,
            short_label,
        })
    }

    fn decode_label(data: &[u8]) -> (String, String) {
        // data contains 16 bytes label and 1 byte short label mask
        let label_bytes = &data[..16];
        let mask = u16::from_be_bytes([data[16], data[17]]);

        let label = String::from_utf8_lossy(label_bytes).trim_end().to_string();

        let mut short_label = String::new();

        for (i, &byte) in label_bytes.iter().enumerate() {
            if mask & (1 << (15 - i)) != 0 {
                short_label.push(byte as char);
            }
        }

        short_label = short_label.trim().to_string();

        (label, short_label)
    }

    fn label_str(label_bytes: &[u8]) -> String {
        String::from_utf8_lossy(label_bytes).trim_end().to_string()
    }

    fn short_label_str(label_bytes: &[u8], mask: u16) -> String {
        let mut short_label = String::new();

        for (i, &byte) in label_bytes.iter().enumerate() {
            if mask & (1 << (15 - i)) != 0 {
                short_label.push(byte as char);
            }
        }

        short_label.trim().to_string()
    }
}

#[derive(Debug, Serialize)]
pub struct Fig1_4 {
    base: Fig1,
}
impl Fig1_4 {
    pub fn from_bytes(base: Fig1, data: &[u8]) -> Result<Self, FIGError> {
        // implement decoding here

        Ok(Self { base })
    }
}

#[derive(Debug, Serialize)]
pub enum FIG {
    F0_0(Fig0_0),
    F0_1(Fig0_1),
    F0_2(Fig0_2),
    F0_3(Fig0_3),
    //
    F1_0(Fig1_0),
    F1_1(Fig1_1),
    F1_4(Fig1_4),
}

#[derive(Debug, Error)]
pub enum FIGError {
    #[error("Unsupported FIG: {kind}")]
    Unsupported { kind: u8 },

    #[error("Missing FIG data")]
    InvalidSize { l: usize },

    #[error("Missing FIG data")]
    NoData,
}

#[derive(Debug, Error)]
pub enum FICError {
    #[error("Invalid FIC size: {l}")]
    SizeInvalid { l: usize },

    #[error("FIG error: {0}")]
    FigError(#[from] FIGError), // converts FIGError to FICError
}

#[derive(Debug)]
pub struct FICDecoder {
    eid: Option<String>,
}

impl FICDecoder {
    pub fn from_bytes(data: &[u8]) -> Result<Vec<FIG>, FICError> {
        if (data.len() % 32) != 0 {
            return Err(FICError::SizeInvalid { l: data.len() });
        }

        let mut figs: Vec<FIG> = Vec::new();

        for chunk in data.chunks(32) {
            figs.extend(Self::decode_fib(chunk)?);
        }

        Ok(figs)
    }

    fn decode_fib(data: &[u8]) -> Result<Vec<FIG>, FICError> {
        let crc_stored = u16::from_be_bytes([data[30], data[31]]);
        let crc_calculated = utils::calc_crc16_ccitt(&data[..30]);

        if crc_stored != crc_calculated {
            log::warn!("FICDecoder: Discarding FIB due to CRC mismatch");
        }

        let mut figs: Vec<FIG> = Vec::new();

        let mut offset = 0;

        while offset < 30 && data[offset] != 0xFF {
            let fig_type = data[offset] >> 5;
            let fig_length = (data[offset] & 0x1F) as usize;

            offset += 1;

            // primary type: 0 / 1
            match fig_type {
                0 => {
                    match Self::decode_fig0(&data[offset..offset + fig_length]) {
                        Ok(fig) => figs.push(fig),
                        Err(e) => {}
                    };
                }
                1 => {
                    match Self::decode_fig1(&data[offset..offset + fig_length]) {
                        Ok(fig) => figs.push(fig),
                        Err(e) => {}
                    };
                }
                _ => {
                    log::warn!("Unknown FIG type: {}", fig_type);
                }
            }

            offset += fig_length;
        }

        // log::debug!("FICDecoder: {} figs", figs.len());

        Ok(figs)
    }

    fn decode_fig0(data: &[u8]) -> Result<FIG, FIGError> {
        if data.is_empty() {
            return Err(FIGError::NoData);
        }

        let header = data[0];

        let cn = header & 0x80 != 0; // Bit 7
        let oe = header & 0x40 != 0; // Bit 6
        let pd = header & 0x20 != 0; // Bit 5
        let ext = header & 0x1F; // Bits 0-4

        // log::debug!("FIG0: cn: {}, oe: {}, pd: {}, ext: {}", cn, oe, pd, ext);

        let base = Fig0 { cn, oe, pd, ext };

        match ext {
            0 => Ok(FIG::F0_0(Fig0_0::from_bytes(base, &data[1..])?)),
            1 => Ok(FIG::F0_1(Fig0_1::from_bytes(base, &data[1..])?)),
            2 => Ok(FIG::F0_2(Fig0_2::from_bytes(base, &data[1..])?)),
            3 => Ok(FIG::F0_3(Fig0_3::from_bytes(base, &data[1..])?)),
            _ => Err(FIGError::Unsupported { kind: ext }),
        }
    }

    fn decode_fig1(data: &[u8]) -> Result<FIG, FIGError> {
        if data.is_empty() {
            return Err(FIGError::NoData);
        }

        let header = data[0];

        let charset = header >> 4; // Upper 4 bits
        let oe = (header & 0x08) != 0; // Bit 3 (boolean)
        let ext = header & 0x07; // Lower 3 bits

        // log::debug!("FIG1: charset: {}, oe: {}, ext: {}", charset, oe, ext);

        let base = Fig1 { charset, oe, ext };

        match ext {
            0 => Ok(FIG::F1_0(Fig1_0::from_bytes(base, &data[1..])?)),
            1 => Ok(FIG::F1_1(Fig1_1::from_bytes(base, &data[1..])?)),
            4 => Ok(FIG::F1_4(Fig1_4::from_bytes(base, &data[1..])?)),
            _ => Err(FIGError::Unsupported { kind: ext }),
        }
    }
}

const UEP_SIZES: [usize; 64] = [
    16, 21, 24, 29, 35, 24, 29, 35, 42, 52, 29, 35, 42, 52, 32, 42, 48, 58, 70, 40, 52, 58, 70, 84,
    48, 58, 70, 84, 104, 58, 70, 84, 104, 64, 84, 96, 116, 140, 80, 104, 116, 140, 168, 96, 116,
    140, 168, 208, 116, 140, 168, 208, 232, 128, 168, 192, 232, 280, 160, 208, 280, 192, 280, 416,
];

const UEP_PLS: [u8; 64] = [
    5, 4, 3, 2, 1, 5, 4, 3, 2, 1, 5, 4, 3, 2, 5, 4, 3, 2, 1, 5, 4, 3, 2, 1, 5, 4, 3, 2, 1, 5, 4, 3,
    2, 5, 4, 3, 2, 1, 5, 4, 3, 2, 1, 5, 4, 3, 2, 1, 5, 4, 3, 2, 1, 5, 4, 3, 2, 1, 5, 4, 2, 5, 3, 1,
];

const UEP_BITRATES: [usize; 64] = [
    32, 32, 32, 32, 32, 48, 48, 48, 48, 48, 56, 56, 56, 56, 64, 64, 64, 64, 64, 80, 80, 80, 80, 80,
    96, 96, 96, 96, 96, 112, 112, 112, 112, 128, 128, 128, 128, 128, 160, 160, 160, 160, 160, 192,
    192, 192, 192, 192, 224, 224, 224, 224, 224, 256, 256, 256, 256, 256, 320, 320, 320, 384, 384,
    384,
];

const EEP_A_SIZE_FACTORS: [usize; 4] = [12, 8, 6, 4];
const EEP_B_SIZE_FACTORS: [usize; 4] = [27, 21, 18, 15];
