use artem::{config::ConfigBuilder, convert};
use derivative::Derivative;
use humansize::{format_size, DECIMAL};
use std::num::NonZeroU32;

use ansi_to_tui::IntoText;
use ratatui::text::Text;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use ratatui::widgets::block::{BorderType, Padding};

pub struct SLSWidget {
    sls_image: Option<SLSImage>,
}

impl SLSWidget {
    pub fn new(sls_image: Option<SLSImage>) -> Self {
        Self { sls_image }
    }
}

impl Widget for SLSWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let has_sls_image = self.sls_image.is_some();
        let area_warning: bool = area.width < 84 || area.height < 28;

        let text = if area_warning {
            Text::from("TERMINAL TOO SMALL")
        } else if let Some(sls_image) = self.sls_image.clone() {
            sls_image
                .ascii
                .into_text()
                .unwrap_or_else(|_| Text::from("ERROR"))
        } else {
            Text::from("NO SLS")
        };

        let text_footer = if let Some(sls_image) = self.sls_image {
            format!(
                " {} | {}x{} | {} ",
                sls_image.mimetype,
                sls_image.width,
                sls_image.height,
                format_size(sls_image.len as u64, DECIMAL)
            )
        } else {
            "".to_string()
        };

        let render_text = if area_warning || !has_sls_image {
            Text::from(format!(
                "{}{}",
                "\n".repeat((area.height.saturating_sub(4) / 2) as usize),
                text
            ))
        } else {
            text
        };

        Clear.render(area, buf);

        Paragraph::new(render_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    // .title(format!(" {:?} ", area))
                    .title_bottom(Line::from(text_footer).centered())
                    .style(
                        Style::default()
                            .bg(if area_warning {
                                Color::Red
                            } else {
                                Color::Black
                            })
                            .fg(Color::White),
                    )
                    .padding(Padding::horizontal(1))
                    .border_type(BorderType::Double)
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct SLSImage {
    pub mimetype: String,
    pub len: usize,
    pub md5: String,
    pub width: u32,
    pub height: u32,
    pub ascii: String,
}

impl SLSImage {
    pub fn new(mimetype: String, len: usize, md5: String, data: Vec<u8>) -> Self {
        let (width, height, ascii) = match image::load_from_memory(&data) {
            Ok(img) => {
                let width = img.width();
                let height = img.height();

                let size = NonZeroU32::try_from(80).unwrap_or(NonZeroU32::new(1).unwrap()); // i don't get this ;)

                let config = ConfigBuilder::new()
                    .target_size(size)
                    .characters("█▓▒░:+-. ".to_string())
                    .hysteresis(true)
                    .color(true)
                    .invert(false)
                    .build();

                // artem::convert returns a String with ANSI escape codes
                let ascii_art = convert(img, &config);
                (width, height, ascii_art)
            }
            Err(_) => (0, 0, "ERROR".to_string()),
        };

        Self {
            mimetype,
            len,
            md5,
            width,
            height,
            ascii,
        }
    }
}
