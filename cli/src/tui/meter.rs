use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span, Text},
    widgets::block::{BorderType, Padding},
    widgets::{Block, Borders, Clear, Gauge, LineGauge, StatefulWidget, Widget},
};

use crate::audio::AudioLevels;

fn level_to_dbfs(level: f32) -> f32 {
    if level <= 0.0 {
        -100.0
    } else {
        20.0 * level.log10()
    }
}

fn level_to_dbfs_48(level: f32) -> f32 {
    if level <= 0.000001 {
        -48.0
    } else {
        20.0 * level.log10().max(-48.0)
    }
}

pub struct LevelMeterWidget {
    levels: AudioLevels
}

impl LevelMeterWidget {
    pub fn new(levels: AudioLevels) -> Self {
        Self { levels }
    }
}

impl Widget for LevelMeterWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {


        Clear.render(area, buf);

        let block = Block::default()
            .title(" LEVELS [dbFS] ")
            .borders(Borders::ALL);

        block.clone().render(area, buf);
        let inner = block.inner(area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(inner);

        for (row, (level, label)) in [(self.levels.rms_smooth.0, " L"), (self.levels.rms_smooth.1, " R")].iter().enumerate() {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(10),
                ])
                .split(rows[row]);

            // LABEL LEFT
            buf.set_string(cols[0].x, cols[0].y, format!("{label}"), Style::default());

            // METER BAR
            let bar_area = cols[1];
            let bar_width = bar_area.width as usize;

            let dbfs = level_to_dbfs_48(*level);
            let fill = ((dbfs + 48.0) / 48.0).clamp(0.0, 1.0);

            let filled_width = ((bar_width as f32) * fill).round() as usize;

            for i in 0..bar_width {
                let symbol = if i < filled_width { "█" } else { " " };
                buf.set_string(
                    bar_area.x + i as u16,
                    bar_area.y,
                    symbol,
                    Style::default().fg(Color::White),
                );
            }

            // VALUE RIGHT
            let dbfs = level_to_dbfs(*level);
            let db = format!("{:5.1} dB", dbfs);
            buf.set_string(cols[2].x, cols[2].y, db, Style::default().fg(Color::White));
        }
    }
}
