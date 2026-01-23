use crate::dab::bus::{emit_event, DabEvent};
use crate::dab::utils::decode_chars;
use derive_more::Debug;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone)]
pub struct DlObject {
    pub scid: u8,
    toggle: u8,
    #[debug(skip)]
    chars: Vec<u8>,
    charset: u8,
    #[debug("{} tags", dl_plus_tags.len())]
    dl_plus_tags: Vec<DlPlusTag>,
    pub seg_count: u8,
}

impl DlObject {
    pub fn new(scid: u8, toggle: u8, charset: u8) -> Self {
        Self {
            scid,
            toggle,
            charset,
            chars: Vec::new(),
            dl_plus_tags: Vec::new(),
            seg_count: 0,
        }
    }
    pub fn decode_label(&self) -> String {
        decode_chars(&self.chars, self.charset)
    }
    pub fn is_dl_plus(&self) -> bool {
        !self.dl_plus_tags.is_empty()
    }
    pub fn get_dl_plus(&self) -> Vec<DlPlusTagDecoded> {
        let label = self.decode_label();
        let label_chars: Vec<char> = label.chars().collect();

        /*
        // not safe: slice index starts at 21 but ends at 20
        self.dl_plus_tags
            .iter()
            .map(|tag| {
                let start = tag.start as usize;
                let end = (start + tag.len as usize).min(label_chars.len());
                let value: String = label_chars[start..end].iter().collect();
                DlPlusTagDecoded {
                    kind: DlPlusContentType::from(tag.kind),
                    value,
                }
            })
            .collect()
        */

        let len = label_chars.len();
        self.dl_plus_tags
            .iter()
            .filter_map(|tag| {
                let kind = DlPlusContentType::from(tag.kind);

                if matches!(kind, DlPlusContentType::Dummy) {
                    log::debug!("Ignoring DL+ tag with kind: {}", kind);
                    return None;
                }

                let start = tag.start as usize;

                if start >= len {
                    log::warn!("DL+ tag start {} >= len {}", start, len);
                    return None;
                }

                let mut end = start.saturating_add(tag.len as usize);
                if end > len {
                    end = len;
                }

                if end <= start {
                    return None;
                }

                let value: String = label_chars[start..end].iter().collect();

                Some(DlPlusTagDecoded { kind, value })
            })
            .collect()
    }
}

impl Serialize for DlObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("DlObject", 5)?;
        s.serialize_field("scid", &self.scid)?;
        s.serialize_field("charset", &self.charset)?;

        // derived fields
        s.serialize_field("label", &self.decode_label())?;
        s.serialize_field("dl_plus", &self.get_dl_plus())?;

        s.end()
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct DlPlusTag {
    pub kind: u8,
    pub start: u8,
    pub len: u8,
}

impl DlPlusTag {
    pub fn new(kind: u8, start: u8, len: u8) -> Self {
        Self { kind, start, len }
    }
}

#[derive(Serialize, Debug)]
pub struct DlPlusTagDecoded {
    pub kind: DlPlusContentType,
    pub value: String,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[repr(u8)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DlPlusContentType {
    Dummy = 0,
    ItemTitle = 1,
    ItemAlbum = 2,
    ItemTracknumber = 3,
    ItemArtist = 4,
    ItemComposition = 5,
    ItemMovement = 6,
    ItemConductor = 7,
    ItemComposer = 8,
    ItemBand = 9,
    ItemComment = 10,
    ItemGenre = 11,
    InfoNews = 12,
    InfoNewsLocal = 13,
    InfoStockmarket = 14,
    InfoSport = 15,
    InfoLottery = 16,
    InfoHoroscope = 17,
    InfoDailyDiversion = 18,
    InfoHealth = 19,
    InfoEvent = 20,
    InfoScene = 21,
    InfoCinema = 22,
    InfoTv = 23,
    InfoDateTime = 24,
    InfoWeather = 25,
    InfoTraffic = 26,
    InfoAlarm = 27,
    InfoAdvertisement = 28,
    InfoUrl = 29,
    InfoOther = 30,
    StationnameShort = 31,
    StationnameLong = 32,
    ProgrammeNow = 33,
    ProgrammeNext = 34,
    ProgrammePart = 35,
    ProgrammeHost = 36,
    ProgrammeEditorialStaff = 37,
    ProgrammeFrequency = 38,
    ProgrammeHomepage = 39,
    ProgrammeSubchannel = 40,
    PhoneHotline = 41,
    PhoneStudio = 42,
    PhoneOther = 43,
    SmsStudio = 44,
    SmsOther = 45,
    EmailHotline = 46,
    EmailStudio = 47,
    EmailOther = 48,
    MmsOther = 49,
    Chat = 50,
    ChatCenter = 51,
    VoteQuestion = 52,
    VoteCentre = 53,
    Private1 = 56,
    Private2 = 57,
    Private3 = 58,
    DescriptorPlace = 59,
    DescriptorAppointment = 60,
    DescriptorIdentifier = 61,
    DescriptorPurchase = 62,
    DescriptorGetData = 63,
    Unknown(u8),
}

impl From<u8> for DlPlusContentType {
    fn from(value: u8) -> Self {
        match value {
            0 => DlPlusContentType::Dummy,
            1 => DlPlusContentType::ItemTitle,
            2 => DlPlusContentType::ItemAlbum,
            3 => DlPlusContentType::ItemTracknumber,
            4 => DlPlusContentType::ItemArtist,
            5 => DlPlusContentType::ItemComposition,
            6 => DlPlusContentType::ItemMovement,
            7 => DlPlusContentType::ItemConductor,
            8 => DlPlusContentType::ItemComposer,
            9 => DlPlusContentType::ItemBand,
            10 => DlPlusContentType::ItemComment,
            11 => DlPlusContentType::ItemGenre,
            12 => DlPlusContentType::InfoNews,
            13 => DlPlusContentType::InfoNewsLocal,
            14 => DlPlusContentType::InfoStockmarket,
            15 => DlPlusContentType::InfoSport,
            16 => DlPlusContentType::InfoLottery,
            17 => DlPlusContentType::InfoHoroscope,
            18 => DlPlusContentType::InfoDailyDiversion,
            19 => DlPlusContentType::InfoHealth,
            20 => DlPlusContentType::InfoEvent,
            21 => DlPlusContentType::InfoScene,
            22 => DlPlusContentType::InfoCinema,
            23 => DlPlusContentType::InfoTv,
            24 => DlPlusContentType::InfoDateTime,
            25 => DlPlusContentType::InfoWeather,
            26 => DlPlusContentType::InfoTraffic,
            27 => DlPlusContentType::InfoAlarm,
            28 => DlPlusContentType::InfoAdvertisement,
            29 => DlPlusContentType::InfoUrl,
            30 => DlPlusContentType::InfoOther,
            31 => DlPlusContentType::StationnameShort,
            32 => DlPlusContentType::StationnameLong,
            33 => DlPlusContentType::ProgrammeNow,
            34 => DlPlusContentType::ProgrammeNext,
            35 => DlPlusContentType::ProgrammePart,
            36 => DlPlusContentType::ProgrammeHost,
            37 => DlPlusContentType::ProgrammeEditorialStaff,
            38 => DlPlusContentType::ProgrammeFrequency,
            39 => DlPlusContentType::ProgrammeHomepage,
            40 => DlPlusContentType::ProgrammeSubchannel,
            41 => DlPlusContentType::PhoneHotline,
            42 => DlPlusContentType::PhoneStudio,
            43 => DlPlusContentType::PhoneOther,
            44 => DlPlusContentType::SmsStudio,
            45 => DlPlusContentType::SmsOther,
            46 => DlPlusContentType::EmailHotline,
            47 => DlPlusContentType::EmailStudio,
            48 => DlPlusContentType::EmailOther,
            49 => DlPlusContentType::MmsOther,
            50 => DlPlusContentType::Chat,
            51 => DlPlusContentType::ChatCenter,
            52 => DlPlusContentType::VoteQuestion,
            53 => DlPlusContentType::VoteCentre,
            56 => DlPlusContentType::Private1,
            57 => DlPlusContentType::Private2,
            58 => DlPlusContentType::Private3,
            59 => DlPlusContentType::DescriptorPlace,
            60 => DlPlusContentType::DescriptorAppointment,
            61 => DlPlusContentType::DescriptorIdentifier,
            62 => DlPlusContentType::DescriptorPurchase,
            63 => DlPlusContentType::DescriptorGetData,
            _ => DlPlusContentType::Unknown(value),
        }
    }
}

impl fmt::Display for DlPlusContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DlPlusContentType::Dummy => write!(f, "DUMMY"),
            DlPlusContentType::ItemTitle => write!(f, "ITEM_TITLE"),
            DlPlusContentType::ItemAlbum => write!(f, "ITEM_ALBUM"),
            DlPlusContentType::ItemTracknumber => write!(f, "ITEM_TRACKNUMBER"),
            DlPlusContentType::ItemArtist => write!(f, "ITEM_ARTIST"),
            DlPlusContentType::ItemComposition => write!(f, "ITEM_COMPOSITION"),
            DlPlusContentType::ItemMovement => write!(f, "ITEM_MOVEMENT"),
            DlPlusContentType::ItemConductor => write!(f, "ITEM_CONDUCTOR"),
            DlPlusContentType::ItemComposer => write!(f, "ITEM_COMPOSER"),
            DlPlusContentType::ItemBand => write!(f, "ITEM_BAND"),
            DlPlusContentType::ItemComment => write!(f, "ITEM_COMMENT"),
            DlPlusContentType::ItemGenre => write!(f, "ITEM_GENRE"),
            DlPlusContentType::InfoNews => write!(f, "INFO_NEWS"),
            DlPlusContentType::InfoNewsLocal => write!(f, "INFO_NEWS_LOCAL"),
            DlPlusContentType::InfoStockmarket => write!(f, "INFO_STOCKMARKET"),
            DlPlusContentType::InfoSport => write!(f, "INFO_SPORT"),
            DlPlusContentType::InfoLottery => write!(f, "INFO_LOTTERY"),
            DlPlusContentType::InfoHoroscope => write!(f, "INFO_HOROSCOPE"),
            DlPlusContentType::InfoDailyDiversion => write!(f, "INFO_DAILY_DIVERSION"),
            DlPlusContentType::InfoHealth => write!(f, "INFO_HEALTH"),
            DlPlusContentType::InfoEvent => write!(f, "INFO_EVENT"),
            DlPlusContentType::InfoScene => write!(f, "INFO_SCENE"),
            DlPlusContentType::InfoCinema => write!(f, "INFO_CINEMA"),
            DlPlusContentType::InfoTv => write!(f, "INFO_TV"),
            DlPlusContentType::InfoDateTime => write!(f, "INFO_DATE_TIME"),
            DlPlusContentType::InfoWeather => write!(f, "INFO_WEATHER"),
            DlPlusContentType::InfoTraffic => write!(f, "INFO_TRAFFIC"),
            DlPlusContentType::InfoAlarm => write!(f, "INFO_ALARM"),
            DlPlusContentType::InfoAdvertisement => write!(f, "INFO_ADVERTISEMENT"),
            DlPlusContentType::InfoUrl => write!(f, "INFO_URL"),
            DlPlusContentType::InfoOther => write!(f, "INFO_OTHER"),
            DlPlusContentType::StationnameShort => write!(f, "STATIONNAME_SHORT"),
            DlPlusContentType::StationnameLong => write!(f, "STATIONNAME_LONG"),
            DlPlusContentType::ProgrammeNow => write!(f, "PROGRAMME_NOW"),
            DlPlusContentType::ProgrammeNext => write!(f, "PROGRAMME_NEXT"),
            DlPlusContentType::ProgrammePart => write!(f, "PROGRAMME_PART"),
            DlPlusContentType::ProgrammeHost => write!(f, "PROGRAMME_HOST"),
            DlPlusContentType::ProgrammeEditorialStaff => write!(f, "PROGRAMME_EDITORIAL_STAFF"),
            DlPlusContentType::ProgrammeFrequency => write!(f, "PROGRAMME_FREQUENCY"),
            DlPlusContentType::ProgrammeHomepage => write!(f, "PROGRAMME_HOMEPAGE"),
            DlPlusContentType::ProgrammeSubchannel => write!(f, "PROGRAMME_SUBCHANNEL"),
            DlPlusContentType::PhoneHotline => write!(f, "PHONE_HOTLINE"),
            DlPlusContentType::PhoneStudio => write!(f, "PHONE_STUDIO"),
            DlPlusContentType::PhoneOther => write!(f, "PHONE_OTHER"),
            DlPlusContentType::SmsStudio => write!(f, "SMS_STUDIO"),
            DlPlusContentType::SmsOther => write!(f, "SMS_OTHER"),
            DlPlusContentType::EmailHotline => write!(f, "EMAIL_HOTLINE"),
            DlPlusContentType::EmailStudio => write!(f, "EMAIL_STUDIO"),
            DlPlusContentType::EmailOther => write!(f, "EMAIL_OTHER"),
            DlPlusContentType::MmsOther => write!(f, "MMS_OTHER"),
            DlPlusContentType::Chat => write!(f, "CHAT"),
            DlPlusContentType::ChatCenter => write!(f, "CHAT_CENTER"),
            DlPlusContentType::VoteQuestion => write!(f, "VOTE_QUESTION"),
            DlPlusContentType::VoteCentre => write!(f, "VOTE_CENTRE"),
            DlPlusContentType::Private1 => write!(f, "PRIVATE_1"),
            DlPlusContentType::Private2 => write!(f, "PRIVATE_2"),
            DlPlusContentType::Private3 => write!(f, "PRIVATE_3"),
            DlPlusContentType::DescriptorPlace => write!(f, "DESCRIPTOR_PLACE"),
            DlPlusContentType::DescriptorAppointment => write!(f, "DESCRIPTOR_APPOINTMENT"),
            DlPlusContentType::DescriptorIdentifier => write!(f, "DESCRIPTOR_IDENTIFIER"),
            DlPlusContentType::DescriptorPurchase => write!(f, "DESCRIPTOR_PURCHASE"),
            DlPlusContentType::DescriptorGetData => write!(f, "DESCRIPTOR_GET_DATA"),
            DlPlusContentType::Unknown(v) => write!(f, "UNKNOWN_{}", v),
        }
    }
}

#[derive(Debug)]
pub struct DlDecoder {
    scid: u8,
    current: Option<DlObject>,
    last_toggle: Option<u8>,
}

impl DlDecoder {
    pub fn new(scid: u8) -> Self {
        Self {
            scid,
            current: None,
            last_toggle: None,
        }
    }

    pub fn feed(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        if data.len() < 2 {
            return None;
        }

        let flags = data[0];
        let num_chars = (flags & 0x0F) + 1;
        let is_first = flags & 0x40 != 0;
        let is_last = flags & 0x20 != 0;
        let toggle = (flags & 0x80) >> 7;

        match (data[0] & 0x10 != 0, data[0] & 0x0F) {
            (true, 0b0001) => {
                log::debug!("[{:2}] DL: CMD clear display", self.scid);
                // TODO: implement reset
            }
            (true, 0b0010) => {
                // TODO: abort if t != toggle
                let _t = data[0] & 0x80;

                if data.len() < 3 {
                    log::warn!(
                        "[{:2}] DL+ too short: expected min 3 bytes, got {}",
                        self.scid,
                        data.len()
                    );
                    return None;
                }

                self.parse_dl_plus(&data[2..]);

                // do not continue on DL+ command
                return None;
            }
            (true, _) => {
                log::debug!(
                    "[{:2}] DL: unexpected command: 0x{:02X}",
                    self.scid,
                    data[0]
                );
            }
            _ => {
                // not a DL+ or display-clear command
            }
        }

        let nibble = (data[1] >> 4) & 0x0F;
        let (_seg_no, charset) = if is_first {
            (0, Some(nibble)) // charset = full 4 bits
        } else {
            (nibble & 0x07, None) // charset not in data
        };

        // log::debug!("DL: seg = {}", seg_no);

        if is_first {
            self.flush();

            self.current = Some(DlObject::new(self.scid, toggle, charset.unwrap_or(0)));
        }

        let start = 2;
        let end = start + num_chars as usize;
        if data.len() >= end {
            // self.current.chars.extend_from_slice(&data[start..end]);
            if let Some(current) = self.current.as_mut() {
                current.chars.extend_from_slice(&data[start..end]);
            }
        } else {
            log::warn!(
                "[{:2}] DL: segment too short: expected {} bytes, got {}",
                self.scid,
                end,
                data.len()
            );
            return None;
        }

        // log::debug!("DL current chars: {:?}", self.current.chars.len());

        if is_last {
            // log::debug!("DL: {}", self.current.decode_label());
            // self.reset();
        }

        None
    }

    pub fn parse_dl_plus(&mut self, data: &[u8]) {
        if data.is_empty() {
            log::warn!("[{:2}] DL+ empty command", self.scid);
            return;
        }

        // there is a bug when pad is short (6 or 8)

        let cid = (data[0] >> 4) & 0x0F;

        if cid != 0 {
            log::debug!("[{:2}] DL+ unexpected command ID = {}", self.scid, cid);
            return;
        }

        // log::debug!("DL Plus: {:?}", cid);

        let _cb = data[0] & 0x0F;
        let _it_toggle = (data[0] >> 3) & 0x01;
        let _it_running = (data[0] >> 2) & 0x01;
        let num_tags = (data[0] & 0x03) + 1;

        // log::debug!("DL+ CID = {}, CB = {}, tags = {} # {} bytes", cid, cb, num_tags, data.len());

        // if data.len() < 0 + num_tags as usize * 3 {
        if data.len() < 1 + num_tags as usize * 3 {
            log::debug!(
                "[{:2}] DL+ unexpected length, expected at least {}",
                self.scid,
                1 + num_tags * 3
            );
            return;
        }

        for i in 0..num_tags {
            let base = 1 + (i * 3) as usize;
            let content_type = data[base] & 0x7F;
            let start = data[base + 1] & 0x7F;
            let len = (data[base + 2] & 0x7F) + 1;

            let tag = DlPlusTag::new(content_type, start, len);

            // log::debug!(
            //     "DL+ tag: {:?}", tag
            // );

            if let Some(current) = self.current.as_mut() {
                current.dl_plus_tags.push(tag);
            }
        }

        // log::debug!("DL+ it_toggle={}, it_running={}", it_toggle, it_running);
    }

    pub fn flush(&mut self) {
        if let Some(current) = self.current.take() {
            if !current.chars.is_empty() && self.last_toggle != Some(current.toggle) {
                log::debug!(
                    "[{:2}] DL: {} - {:?}",
                    self.scid,
                    current.decode_label(),
                    current
                );

                // log::debug!("[{:2}] DL{} {}", self.scid, if !current.dl_plus_tags.is_empty() {"+"} else {" "}, current.decode_label());

                // log::debug!("{:?}", current.get_dl_plus());

                // let json = serde_json::to_string_pretty(&current).unwrap();
                // println!("{}", json);

                emit_event(DabEvent::DlObjectReceived(current.clone()));
                self.last_toggle = Some(current.toggle);
            }
        }
    }
}
