use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Widget},
};

use crate::audio::AudioLevels;

fn level_to_dbfs_48(level: f32) -> f32 {
    if level <= 0.000001 {
        -48.0
    } else {
        20.0 * level.max(0.000001).log10().max(-48.0)
    }
}

pub struct LevelMeterWidget {
    levels: AudioLevels,
}

impl LevelMeterWidget {
    pub fn new(levels: AudioLevels) -> Self {
        Self { levels }
    }

    fn render_meters(&self, buf: &mut Buffer, area: Rect, peaks: (f32, f32), rms: (f32, f32)) {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(5),
                Constraint::Length(1),
                Constraint::Length(2),
            ])
            .split(area);

        let base_y = area.y + 1;
        let h = (area.height as usize).saturating_sub(4);

        let db_step = 4;
        let num_steps = 48 / db_step;
        let tick_interval = h / num_steps.max(1);

        let mut draw_bar = |x: u16, peak: f32, rms: f32| {
            let db_peak = level_to_dbfs_48(peak);
            let db_rms = level_to_dbfs_48(rms);

            let peak_pos = ((1.0 - (db_peak + 48.0) / 48.0) * h as f32).round() as usize;
            let rms_top = ((1.0 - (db_rms + 48.0) / 48.0) * h as f32).round() as usize;

            for i in 0..h {
                let y = base_y + i as u16;

                let style = if i == peak_pos {
                    Style::default().fg(Color::Red)
                } else if i >= rms_top {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Black)
                };

                /*
                let symbol = if i == peak_pos {
                    "██"
                } else if i >= rms_top {
                    "██"
                } else {
                    "██"
                };
                */

                let symbol = "██";

                buf.set_string(x, y, symbol, style);
            }
        };

        draw_bar(columns[1].x, peaks.0, rms.0);
        draw_bar(columns[5].x, peaks.1, rms.1);

        for i in 0..=num_steps {
            let db = i as i32 * db_step as i32;
            let y = base_y + i as u16 * tick_interval as u16;
            if y >= area.y + area.height - 2 {
                continue;
            }
            let label = format!("─{:>2}─", db);
            buf.set_span(
                columns[3].x,
                y,
                &Span::styled(label, Style::default().fg(Color::DarkGray)),
                7,
            );
        }

        let peak_line = format!(
            "{:>5.1} PK  {:>5.1}",
            level_to_dbfs_48(peaks.0),
            level_to_dbfs_48(peaks.1)
        );
        let rms_line = format!(
            "{:>5.1} RMS {:>5.1}",
            level_to_dbfs_48(rms.0),
            level_to_dbfs_48(rms.1)
        );

        let y_base = base_y + h as u16 + 1;
        if y_base < area.y + area.height {
            let style = if level_to_dbfs_48(peaks.0) > 0.0 || level_to_dbfs_48(peaks.1) > 0.0 {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            buf.set_string(area.x, y_base, &peak_line, style);
        }
        if y_base + 1 < area.y + area.height {
            buf.set_string(
                area.x,
                y_base + 1,
                &rms_line,
                Style::default().fg(Color::DarkGray),
            );
        }
    }
}

impl Widget for LevelMeterWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let block = Block::default().title(" Levels ").borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        self.render_meters(buf, inner, self.levels.peak_smooth, self.levels.rms_smooth);
    }
}
