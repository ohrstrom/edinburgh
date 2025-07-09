use humansize::{format_size, BINARY, DECIMAL};
use shared::edi::pad::dl::DLObject;
use shared::edi::{EDISStats, Ensemble, Subchannel};
use std::{io, time::Duration};

use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState, Wrap},
    Terminal,
};

use ratatui::widgets::block::{BorderType, Padding, Position, Title};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub enum TUIEvent {
    EnsembleUpdated(Ensemble),
    DLObjectReceived(DLObject),
    EDISStatsUpdated(EDISStats),
}

pub enum TUICommand {
    ScIDSelected(u8),
    Shutdown,
}

#[derive(Debug)]
pub struct TuiState {
    pub addr: String,
    pub current_ensemble: Option<Ensemble>,
    pub selected_scid: Option<u8>,
    pub services: Vec<ServiceRow>,
    pub table_state: TableState,
    pub dl_objects: Vec<(u8, Option<DLObject>)>,
    pub edi_stats: EDISStats,
}

impl TuiState {
    pub fn new(addr: String, initial_scid: Option<u8>) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            addr,
            current_ensemble: None,
            selected_scid: initial_scid,
            services: Vec::new(),
            table_state,
            dl_objects: Vec::new(),
            edi_stats: EDISStats::new(), // NOTE: should we rather use option & none here?
        }
    }

    pub fn update_services(&mut self, ensemble: Ensemble) {
        self.current_ensemble = Some(ensemble.clone());

        self.services = ensemble
            .services
            .iter()
            .map(|svc| {
                let scid = svc.components.first().map(|c| c.scid).unwrap_or(0);

                let subchannel = ensemble
                    .subchannels
                    .iter()
                    .find(|sub| sub.id == scid)
                    .cloned();

                let audio_format = svc.components.first().and_then(|c| c.audio_format.clone());

                ServiceRow {
                    sid: format!("0x{:04X}", svc.sid),
                    label: svc
                        .label
                        .clone()
                        .unwrap_or_else(|| "(no label)".to_string()),
                    short_label: svc.short_label.clone().unwrap_or_else(|| "".to_string()),
                    scid,
                    subchannel,
                    format: audio_format
                        .map(|x| format!("{}", x))
                        .unwrap_or_else(|| "-".into()),
                }
            })
            .collect();

        self.services.sort_by_key(|svc| svc.scid);

        if self.services.is_empty() {
            self.table_state.select(None);
        } else {
            let current = self.table_state.selected().unwrap_or(0);
            let new = current.min(self.services.len() - 1);
            self.table_state.select(Some(new));
        }
    }

    pub fn update_dl_object(&mut self, dl: DLObject) {
        match self
            .dl_objects
            .iter_mut()
            .find(|(scid, _)| *scid == dl.scid)
        {
            Some((_, obj)) => *obj = Some(dl),
            None => self.dl_objects.push((dl.scid, Some(dl))),
        }
    }

    pub fn update_edi_stats(&mut self, stats: EDISStats) {
        self.edi_stats = stats;
    }
}

#[derive(Debug, Clone)]
pub struct ServiceRow {
    pub sid: String,
    pub label: String,
    pub short_label: String,
    pub scid: u8,
    pub subchannel: Option<Subchannel>,
    pub format: String,
}

pub async fn run_tui(
    addr: String,
    scid: Option<u8>,
    tx: UnboundedSender<TUIEvent>,
    mut rx: UnboundedReceiver<TUIEvent>,
    cmd_tx: UnboundedSender<TUICommand>,
) -> io::Result<()> {
    // term init
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // state
    let mut state = TuiState::new(addr, scid);

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(area);

            ///////////////////////////////////////////////////////////
            // ensemble info
            ///////////////////////////////////////////////////////////
            let ensemble_left = Paragraph::new(format!(
                "{}\n0x{:04X}",
                state
                    .current_ensemble
                    .as_ref()
                    .and_then(|e| e.label.as_deref())
                    .unwrap_or("-"),
                state
                    .current_ensemble
                    .as_ref()
                    .and_then(|e| e.eid)
                    .unwrap_or(0),
            ))
            .block(
                Block::default()
                    .title(" Ensemble ")
                    .padding(Padding::horizontal(1))
                    .border_type(BorderType::Plain)
                    .borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM),
            )
            .wrap(Wrap { trim: true });

            let ensemble_right = Paragraph::new(format!(
                "tcp://{}\nRX: {:>5.0} kbits • {} frames • {}",
                state.addr,
                state.edi_stats.rx_rate as f64 / 128.0,
                state.edi_stats.rx_frames,
                format_size(state.edi_stats.rx_bytes, DECIMAL),
            ))
            .block(
                Block::default()
                    .title(" EDI ")
                    .padding(Padding::horizontal(1))
                    .border_type(BorderType::Plain)
                    .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM),
            )
            .wrap(Wrap { trim: true });

            let ensemble_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Fill(1), Constraint::Length(64)])
                .split(layout[0]);

            frame.render_widget(ensemble_left, ensemble_layout[0]);
            frame.render_widget(ensemble_right, ensemble_layout[1]);

            ///////////////////////////////////////////////////////////
            // service table
            ///////////////////////////////////////////////////////////
            let header = Row::new(vec![
                "SC",
                "SID",
                "Label",
                "Short",
                "EEP      CUs SA",
                "Format",
            ])
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

            let rows = state.services.iter().map(|svc| {
                let style = if Some(svc.scid) == state.selected_scid {
                    Style::default()
                        .bg(Color::White)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let sc_info = if let Some(sc) = &svc.subchannel {
                    format!(
                        "{} {:>3} {:>3} ",
                        sc.pl.clone().unwrap_or("-".to_string()),
                        sc.size.unwrap_or(0),
                        sc.start.unwrap_or(0),
                    )
                } else {
                    svc.scid.to_string()
                };

                Row::new(vec![
                    format!("{:>4}", svc.scid),
                    svc.sid.clone(),
                    svc.label.clone(),
                    svc.short_label.clone(),
                    // service_dl,
                    sc_info,
                    svc.format.clone(),
                ])
                .style(style)
            });

            let table = Table::new(
                rows,
                [
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Length(18),
                    Constraint::Length(36),
                    Constraint::Length(36),
                    Constraint::Length(80),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .title(" Services ")
                    .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT | Borders::BOTTOM),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );

            frame.render_stateful_widget(table, layout[1], &mut state.table_state);

            ///////////////////////////////////////////////////////////
            // player
            ///////////////////////////////////////////////////////////
            let current_service = if let Some(scid) = state.selected_scid {
                state.services.iter().find(|svc| svc.scid == scid)
            } else {
                None
            };

            let player_title = match (current_service) {
                Some(svc) => format!(" Player SC {:>2} - {} ", svc.scid, svc.format),
                None => " Player ".to_string(),
            };

            let player_text = match (current_service) {
                Some(svc) => format!("{}", svc.label),
                None => "No service selected".to_string(),
            };

            let player_dl = if let Some(selected) = state.selected_scid {
                match state.dl_objects.iter().find(|(scid, _)| *scid == selected) {
                    Some((_, Some(dl))) => dl.decode_label(),
                    _ => "-".into(),
                }
            } else {
                "-".into()
            };

            let player_left = Paragraph::new(player_text)
                .block(
                    Block::default()
                        .title(player_title)
                        .padding(Padding::horizontal(1))
                        .borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM),
                )
                .wrap(Wrap { trim: true });

            let player_right = Paragraph::new(player_dl)
                .block(
                    Block::default()
                        .title(" DL ")
                        .padding(Padding::horizontal(1))
                        .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM),
                )
                .wrap(Wrap { trim: true });

            let player_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[2]);

            frame.render_widget(player_left, player_layout[0]);
            frame.render_widget(player_right, player_layout[1]);
        })?;

        // key input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        let _ = cmd_tx.send(TUICommand::Shutdown);
                        break;
                    }
                    KeyCode::Esc => {
                        let _ = cmd_tx.send(TUICommand::Shutdown);
                        break;
                    }
                    KeyCode::Up => {
                        if let Some(selected) = state.table_state.selected() {
                            let new = if selected == 0 {
                                state.services.len().saturating_sub(1)
                            } else {
                                selected - 1
                            };
                            state.table_state.select(Some(new));
                        }
                    }
                    KeyCode::Down => {
                        if let Some(selected) = state.table_state.selected() {
                            let new = if selected >= state.services.len().saturating_sub(1) {
                                0
                            } else {
                                selected + 1
                            };
                            state.table_state.select(Some(new));
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(selected) = state.table_state.selected() {
                            let scid = state.services[selected].scid;
                            state.selected_scid = Some(scid);
                            let _ = cmd_tx.send(TUICommand::ScIDSelected(scid));
                        }
                    }
                    KeyCode::Char('m') => {
                        println!("Mute toggled");
                    }
                    _ => {}
                }
            }
        }

        while let Ok(msg) = rx.try_recv() {
            match msg {
                TUIEvent::EnsembleUpdated(ensemble) => {
                    state.update_services(ensemble);
                }
                TUIEvent::DLObjectReceived(d) => {
                    state.update_dl_object(d);
                }
                TUIEvent::EDISStatsUpdated(s) => {
                    state.update_edi_stats(s);
                }
                _ => {}
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
