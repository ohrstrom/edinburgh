use std::fmt;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Language {
    ALB = 0x01, BRE = 0x02, CAT = 0x03, HRV = 0x04, CYM = 0x05,
    CES = 0x06, DAN = 0x07, DEU = 0x08, ENG = 0x09, SPA = 0x0A,
    EPO = 0x0B, EST = 0x0C, EUS = 0x0D, FAE = 0x0E, FRA = 0x0F,
    FRY = 0x10, GLE = 0x11, GLG = 0x13, ISL = 0x14, ITA = 0x15,
    LAT = 0x17, LAV = 0x18, LUX = 0x19, LIT = 0x1A, HUN = 0x1B,
    MLT = 0x1C, NLD = 0x1D, NOR = 0x1E, OCI = 0x1F, POL = 0x20,
    POR = 0x21, RON = 0x22, ROH = 0x23, SRP = 0x24, SLK = 0x25,
    SLV = 0x26, FIN = 0x27, SWE = 0x28, TUR = 0x29, ZUL = 0x45,
    VIE = 0x46, UZB = 0x47, URD = 0x48, UKR = 0x49, THA = 0x4A,
    TEL = 0x4B, TAT = 0x4C, TAM = 0x4D, TGK = 0x4E, SWA = 0x4F,
    SOM = 0x51, SIN = 0x52, SHO = 0x53, RUS = 0x56, QUE = 0x57,
    PST = 0x58, PAN = 0x59, PER = 0x5A, ORI = 0x5C, NEP = 0x5D,
    MAR = 0x5F, MOL = 0x60, MAL = 0x61, MKD = 0x63, KOR = 0x65,
    KHM = 0x66, KAZ = 0x67, JPN = 0x69, IND = 0x6A, HIN = 0x6B,
    HEB = 0x6C, GRE = 0x70, CHI = 0x75, BUL = 0x77, BEN = 0x78,
    ARM = 0x7D, ARA = 0x7E, AMH = 0x7F, Unknown = 0xFF
}

impl From<u8> for Language {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Language::ALB, 0x02 => Language::BRE, 0x03 => Language::CAT, 0x04 => Language::HRV,
            0x05 => Language::CYM, 0x06 => Language::CES, 0x07 => Language::DAN, 0x08 => Language::DEU,
            0x09 => Language::ENG, 0x0A => Language::SPA, 0x0B => Language::EPO, 0x0C => Language::EST,
            0x0D => Language::EUS, 0x0E => Language::FAE, 0x0F => Language::FRA, 0x10 => Language::FRY,
            0x11 => Language::GLE, 0x13 => Language::GLG, 0x14 => Language::ISL, 0x15 => Language::ITA,
            0x17 => Language::LAT, 0x18 => Language::LAV, 0x19 => Language::LUX, 0x1A => Language::LIT,
            0x1B => Language::HUN, 0x1C => Language::MLT, 0x1D => Language::NLD, 0x1E => Language::NOR,
            0x1F => Language::OCI, 0x20 => Language::POL, 0x21 => Language::POR, 0x22 => Language::RON,
            0x23 => Language::ROH, 0x24 => Language::SRP, 0x25 => Language::SLK, 0x26 => Language::SLV,
            0x27 => Language::FIN, 0x28 => Language::SWE, 0x29 => Language::TUR, 0x45 => Language::ZUL,
            0x46 => Language::VIE, 0x47 => Language::UZB, 0x48 => Language::URD, 0x49 => Language::UKR,
            0x4A => Language::THA, 0x4B => Language::TEL, 0x4C => Language::TAT, 0x4D => Language::TAM,
            0x4E => Language::TGK, 0x4F => Language::SWA, 0x51 => Language::SOM, 0x52 => Language::SIN,
            0x53 => Language::SHO, 0x56 => Language::RUS, 0x57 => Language::QUE, 0x58 => Language::PST,
            0x59 => Language::PAN, 0x5A => Language::PER, 0x5C => Language::ORI, 0x5D => Language::NEP,
            0x5F => Language::MAR, 0x60 => Language::MOL, 0x61 => Language::MAL, 0x63 => Language::MKD,
            0x65 => Language::KOR, 0x66 => Language::KHM, 0x67 => Language::KAZ, 0x69 => Language::JPN,
            0x6A => Language::IND, 0x6B => Language::HIN, 0x6C => Language::HEB, 0x70 => Language::GRE,
            0x75 => Language::CHI, 0x77 => Language::BUL, 0x78 => Language::BEN, 0x7D => Language::ARM,
            0x7E => Language::ARA, 0x7F => Language::AMH, _ => Language::Unknown
        }
    }
}

impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::ALB => write!(f, "Albanian"),
            Language::AMH => write!(f, "Amharic"),
            Language::ARA => write!(f, "Arabic"),
            Language::ARM => write!(f, "Armenian"),
            Language::BEN => write!(f, "Bengali"),
            Language::BRE => write!(f, "Breton"),
            Language::BUL => write!(f, "Bulgarian"),
            Language::CAT => write!(f, "Catalan"),
            Language::CES => write!(f, "Czech"),
            Language::CHI => write!(f, "Chinese"),
            Language::CYM => write!(f, "Welsh"),
            Language::DAN => write!(f, "Danish"),
            Language::DEU => write!(f, "German"),
            Language::ENG => write!(f, "English"),
            Language::EPO => write!(f, "Esperanto"),
            Language::EST => write!(f, "Estonian"),
            Language::EUS => write!(f, "Basque"),
            Language::FAE => write!(f, "Faroese"),
            Language::FIN => write!(f, "Finnish"),
            Language::FRA => write!(f, "French"),
            Language::FRY => write!(f, "Frisian"),
            Language::GLE => write!(f, "Irish"),
            Language::GLG => write!(f, "Galician"),
            Language::GRE => write!(f, "Greek"),
            Language::HEB => write!(f, "Hebrew"),
            Language::HIN => write!(f, "Hindi"),
            Language::HRV => write!(f, "Croatian"),
            Language::HUN => write!(f, "Hungarian"),
            Language::IND => write!(f, "Indonesian"),
            Language::ISL => write!(f, "Icelandic"),
            Language::ITA => write!(f, "Italian"),
            Language::JPN => write!(f, "Japanese"),
            Language::KAZ => write!(f, "Kazakh"),
            Language::KHM => write!(f, "Khmer"),
            Language::KOR => write!(f, "Korean"),
            Language::LAT => write!(f, "Latin"),
            Language::LAV => write!(f, "Latvian"),
            Language::LIT => write!(f, "Lithuanian"),
            Language::LUX => write!(f, "Luxembourgish"),
            Language::MAL => write!(f, "Malay"),
            Language::MAR => write!(f, "Marathi"),
            Language::MKD => write!(f, "Macedonian"),
            Language::MLT => write!(f, "Maltese"),
            Language::MOL => write!(f, "Moldavian"),
            Language::NEP => write!(f, "Nepali"),
            Language::NLD => write!(f, "Dutch"),
            Language::NOR => write!(f, "Norwegian"),
            Language::OCI => write!(f, "Occitan"),
            Language::ORI => write!(f, "Oriya"),
            Language::PAN => write!(f, "Punjabi"),
            Language::PER => write!(f, "Persian"),
            Language::POL => write!(f, "Polish"),
            Language::POR => write!(f, "Portuguese"),
            Language::PST => write!(f, "Pushtu"),
            Language::QUE => write!(f, "Quechua"),
            Language::RON => write!(f, "Romanian"),
            Language::ROH => write!(f, "Romansh"),
            Language::RUS => write!(f, "Russian"),
            Language::SHO => write!(f, "Shona"),
            Language::SIN => write!(f, "Sinhalese"),
            Language::SLK => write!(f, "Slovak"),
            Language::SLV => write!(f, "Slovene"),
            Language::SOM => write!(f, "Somali"),
            Language::SPA => write!(f, "Spanish"),
            Language::SRP => write!(f, "Serbian"),
            Language::SWA => write!(f, "Swahili"),
            Language::SWE => write!(f, "Swedish"),
            Language::TAM => write!(f, "Tamil"),
            Language::TAT => write!(f, "Tatar"),
            Language::TEL => write!(f, "Telugu"),
            Language::TGK => write!(f, "Tajik"),
            Language::THA => write!(f, "Thai"),
            Language::TUR => write!(f, "Turkish"),
            Language::UKR => write!(f, "Ukrainian"),
            Language::URD => write!(f, "Urdu"),
            Language::UZB => write!(f, "Uzbek"),
            Language::VIE => write!(f, "Vietnamese"),
            Language::ZUL => write!(f, "Zulu"),
            Language::Unknown => write!(f, "UNKNOWN")
        }
    }
}




#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum UserApplication {
    Reserved = 0x000,
    SLS = 0x002,           // SlideShow
    TPEG = 0x004,          // Transport Protocol Experts Group
    SPI = 0x007,           // Service and Programme Information
    DMB = 0x009,           // Digital Multimedia Broadcasting
    Filecasting = 0x00D,
    FIS = 0x00E,           // Fast Information Channel
    Journaline = 0x044A,   // Fraunhofer service

    Unknown(u8),           // fallback for unmapped (0..=255 only)
}

impl From<u16> for UserApplication {
    fn from(value: u16) -> Self {
        match value {
            0x000 => UserApplication::Reserved,
            0x002 => UserApplication::SLS,
            0x004 => UserApplication::TPEG,
            0x007 => UserApplication::SPI,
            0x009 => UserApplication::DMB,
            0x00D => UserApplication::Filecasting,
            0x00E => UserApplication::FIS,
            0x044A => UserApplication::Journaline,
            val => UserApplication::Unknown((val & 0xFF) as u8),
        }
    }
}

impl fmt::Display for UserApplication {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserApplication::Reserved     => write!(f, "Reserved"),
            UserApplication::SLS          => write!(f, "SlideShow"),
            UserApplication::TPEG         => write!(f, "TPEG"),
            UserApplication::SPI          => write!(f, "SPI"),
            UserApplication::DMB          => write!(f, "DMB"),
            UserApplication::Filecasting  => write!(f, "Filecasting"),
            UserApplication::FIS          => write!(f, "FIS"),
            UserApplication::Journaline   => write!(f, "Journaline"),
            UserApplication::Unknown(v)   => write!(f, "Unknown(0x{:02X})", v),
        }
    }
}

impl Serialize for UserApplication {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}