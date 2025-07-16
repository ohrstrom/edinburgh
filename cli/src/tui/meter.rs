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
    levels: AudioLevels,
}

impl LevelMeterWidget {
    pub fn new(levels: AudioLevels) -> Self {
        Self { levels }
    }

    fn render_bar(&self, buf: &mut Buffer, area: Rect, label: &str, peak: f32, rms: f32) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(36),
            ])
            .split(area);

        buf.set_string(cols[0].x, cols[0].y, label, Style::default());

        let bar_area = cols[1];
        let bar_width = bar_area.width as usize;

        let dbfs = level_to_dbfs_48(rms);
        let fill = ((dbfs + 48.0) / 48.0).clamp(0.0, 1.0);
        let filled_width = ((bar_width as f32) * fill).round() as usize;

        /*
        /
        for i in 0..bar_width {
            let symbol = if i < filled_width { "█" } else { " " };
            buf.set_string(
                bar_area.x + i as u16,
                bar_area.y,
                symbol,
                Style::default().fg(Color::White),
            );
        }
        */

        let peak_dbfs = level_to_dbfs_48(peak);
        let peak_fill = ((peak_dbfs + 48.0) / 48.0).clamp(0.0, 1.0);
        let peak_pos = ((bar_width as f32) * peak_fill).round() as usize;

        for i in 0..bar_width {
            let is_peak = i == peak_pos.saturating_sub(1).min(bar_width - 1);

            let symbol = if is_peak {
                // "█"
                "░"
            } else if i < filled_width {
                "█"
                // "░"
            } else {
                " "
            };

            let style = if is_peak {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::White)
            };

            buf.set_string(bar_area.x + i as u16, bar_area.y, symbol, style);
        }

        let peak_dbfs = level_to_dbfs(peak);
        let rms_dbfs = level_to_dbfs(rms);
        buf.set_string(
            cols[2].x,
            cols[2].y,
            format!(" | RMS: {:5.1} PEAK: {:5.1}", rms_dbfs, peak_dbfs),
            Style::default().fg(Color::White),
        );
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

        let levels = [
            (self.levels.peak_smooth.0, self.levels.rms_smooth.0),
            (self.levels.peak_smooth.1, self.levels.rms_smooth.1),
        ];
        let labels = ["L", "R"];

        for (row, (&(peak, rms), label)) in levels.iter().zip(labels.iter()).enumerate() {
            self.render_bar(buf, rows[row], label, peak, rms);
        }
    }
}
