use shared::edi::Ensemble;
use shared::edi::msc::{AACPResult, AudioFormat};
use shared::edi::pad::dl::DLObject;
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

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub struct ServiceRow {
    pub sid: String,
    pub label: String,
    pub short_label: String,
    pub sc_info: String,
    pub scid: u8,
    pub format: String,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub scid: u8,
    pub audio_format: AudioFormat,
}

#[derive(Debug)]
pub enum TUIEvent {
    EnsembleUpdated(Ensemble),
    DLObjectReceived(DLObject),
}

pub enum TUICommand {
    ScIDSelected(u8),
    Shutdown,
}

pub async fn run_tui(
    scid: Option<u8>,
    tx: UnboundedSender<TUIEvent>,
    mut rx: UnboundedReceiver<TUIEvent>,
    cmd_tx: UnboundedSender<TUICommand>,
) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut current_ensemble: Option<Ensemble> = None;

    // let mut selected_scid: Option<u8> = None;
    let mut selected_scid: Option<u8> = scid;

    let mut services: Vec<ServiceRow> = vec![];
    let mut table_state = TableState::default();


    let mut dl_objects: Vec<(u8, Option<DLObject>)> = vec![];


    table_state.select(Some(0));

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
            let ensemble_block = Paragraph::new("asd")
                .block(Block::default().title(" Ensemble ").borders(Borders::ALL))
                .wrap(Wrap { trim: true });

            frame.render_widget(ensemble_block, layout[0]);

            ///////////////////////////////////////////////////////////
            // service table
            ///////////////////////////////////////////////////////////
            let header = Row::new(vec!["SC", "SID", "Label", "Short", "Format"]).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

            let rows = services.iter().map(|svc| {
                let style = if Some(svc.scid) == selected_scid {
                    Style::default()
                        .bg(Color::White)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    format!("{:>4}", svc.scid),
                    svc.sid.clone(),
                    svc.label.clone(),
                    svc.short_label.clone(),
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
                    Constraint::Length(80),
                ],
            )
            .header(header)
            .block(Block::default().title(" Services ").borders(Borders::TOP | Borders::LEFT | Borders::RIGHT))
            .row_highlight_style(
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );

            frame.render_stateful_widget(table, layout[1], &mut table_state);

            ///////////////////////////////////////////////////////////
            // player
            ///////////////////////////////////////////////////////////
            let current_service = if let Some(scid) = selected_scid {
                services.iter().find(|svc| svc.scid == scid)
            } else {
                None
            };

            let player_title = match(current_service) {
                Some(svc) => format!(
                    " Player SC {:>2} - {} ",
                    svc.scid,
                    svc.format
                ),
                None => " Player ".to_string(),
            };

            let player_text = match(current_service) {
                Some(svc) => format!(
                    "{}",
                    svc.label
                ),
                None => "No service selected".to_string(),
            };

            let player_dl = if let Some(selected) = selected_scid {
                match dl_objects.iter().find(|(scid, _)| *scid == selected) {
                    Some((_, Some(dl))) => format!(
                        "{}",
                        dl.decode_label()
                    ),
                    _ => "-".to_string(),
                }
            } else {
                "-".to_string()
            };

            let player_left = Paragraph::new(player_text)
                .block(Block::default().title(player_title).borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM))
                .wrap(Wrap { trim: true });

            let player_right = Paragraph::new(player_dl)
                .block(Block::default().title(" DL ").borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM))
                .wrap(Wrap { trim: true });

            let player_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[2]);

            frame.render_widget(player_left, player_layout[0]);
            frame.render_widget(player_right, player_layout[1]);


            // frame.render_widget(ensemble_block, layout[2]);
            // frame.render_widget(ensemble_block, layout[2]);

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
                        if let Some(selected) = table_state.selected() {
                            let new = if selected == 0 {
                                services.len().saturating_sub(1)
                            } else {
                                selected - 1
                            };
                            table_state.select(Some(new));
                        }
                    }
                    KeyCode::Down => {
                        if let Some(selected) = table_state.selected() {
                            let new = if selected >= services.len().saturating_sub(1) {
                                0
                            } else {
                                selected + 1
                            };
                            table_state.select(Some(new));
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(selected) = table_state.selected() {
                            let scid = services[selected].scid;
                            selected_scid = Some(scid);

                            if let Err(e) = cmd_tx.send(TUICommand::ScIDSelected(scid)) {
                                eprintln!("Failed to send TUICommand::ScIDSelected: {:?}", e);
                            }
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

                    current_ensemble = Some(ensemble.clone());

                    services = ensemble
                        .services
                        .iter()
                        .map(|svc| {
                            let scid = svc.components.first().map(|c| c.scid).unwrap_or(0);
                            let audio_format = svc
                                .components
                                .first()
                                .and_then(|c| c.audio_format.clone());

                            let codec = audio_format
                                .as_ref()
                                .map(|af| af.codec.as_str())
                                .unwrap_or("-");

                            ServiceRow {
                                sid: format!("0x{:04X}", svc.sid),
                                label: svc
                                    .label
                                    .clone()
                                    .unwrap_or_else(|| "(no label)".to_string()),
                                short_label: svc
                                    .short_label
                                    .clone()
                                    .unwrap_or_else(|| "".to_string()),
                                sc_info: "asd".to_string(),
                                scid: scid,
                                format: audio_format.map(|x| format!("{}", x))
                                    .unwrap_or_else(|| "-".to_string())
                            }
                        })
                        .collect();

                    services.sort_by_key(|svc| svc.scid);

                    if services.is_empty() {
                        table_state.select(None);
                    } else {
                        let current = table_state.selected().unwrap_or(0);
                        let new = current.min(services.len() - 1);
                        table_state.select(Some(new));
                    }
                }
                TUIEvent::DLObjectReceived(d) => {
                    match dl_objects.iter_mut().find(|(scid, _)| *scid == d.scid) {
                        Some((_, obj)) => {
                            *obj = Some(d);
                        }
                        None => {
                            dl_objects.push((d.scid, Some(d)));
                        }
                    }
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
