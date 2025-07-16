use humansize::{format_size, DECIMAL};
use shared::edi::pad::dl::DLObject;
use shared::edi::pad::mot::MOTImage;
use shared::edi::{EDISStats, Ensemble, Subchannel};
use std::{io, time::Duration};

use derivative::Derivative;

use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Terminal,
};

use ratatui::widgets::block::{BorderType, Padding};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::sls::{SLSImage, SLSWidget};

fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

#[derive(Debug)]
pub enum TUIEvent {
    EnsembleUpdated(Ensemble),
    DLObjectReceived(DLObject),
    MOTImageReceived(MOTImage),
    EDISStatsUpdated(EDISStats),
}

pub enum TUICommand {
    ScIDSelected(u8),
    Shutdown,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct TuiState {
    pub addr: String,
    pub current_ensemble: Option<Ensemble>,
    pub selected_scid: Option<u8>,
    pub services: Vec<ServiceRow>,
    pub table_state: TableState,
    pub dl_objects: Vec<(u8, Option<DLObject>)>,
    pub sls_images: Vec<(u8, Option<SLSImage>)>,
    pub edi_stats: EDISStats,
    //
    pub show_meter: bool,
    pub show_sls: bool,
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
            sls_images: Vec::new(),
            edi_stats: EDISStats::new(), // NOTE: should we rather use option & none here?
            //
            show_meter: false,
            show_sls: false,
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

    pub fn update_mot_image(&mut self, m: MOTImage) {
        let s = SLSImage::new(
            m.mimetype.clone().to_uppercase(),
            m.len,
            m.md5_hex().to_uppercase(),
            m.data.clone(),
        );

        match self.sls_images.iter_mut().find(|(scid, _)| *scid == m.scid) {
            Some((_, obj)) => *obj = Some(s),
            None => self.sls_images.push((m.scid, Some(s))),
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
    #[allow(unused_variables)] tx: UnboundedSender<TUIEvent>,
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
                    Constraint::Length(1),
                    Constraint::Min(0),
                    Constraint::Length(4),
                    // level meter: 4 if state.show_meter on,. else 0
                    Constraint::Length(if state.show_meter { 4 } else { 0 }),
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
            // keyboard input display
            ///////////////////////////////////////////////////////////
            let input_text = "q: quit • m: toggle mute • Enter: select";
            let input_paragraph = Paragraph::new(input_text)
                .block(
                    Block::default()
                        .padding(Padding::horizontal(2))
                        .borders(Borders::NONE),
                )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            frame.render_widget(input_paragraph, layout[1]);

            ///////////////////////////////////////////////////////////
            // service table
            ///////////////////////////////////////////////////////////
            let header = Row::new(vec![
                " SC",
                "SID",
                "Label",
                "Short",
                "EEP      CUs SA",
                "Format",
                "DL",
                "SLS",
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

                let dl_info = if let Some(dl) = state
                    .dl_objects
                    .iter()
                    .find(|(scid, _)| *scid == svc.scid)
                {
                    if let Some(dl) = dl.1.as_ref() {
                        if !dl.get_dl_plus().is_empty() {
                            "DL+"
                        } else {
                            "DL"
                        }
                    } else {
                        "-"
                    }
                } else {
                    "-"
                };


                let sls_info = if let Some((_, Some(sls_image))) =
                    state.sls_images.iter().find(|(scid, _)| *scid == svc.scid)
                {
                    let size_style = if sls_image.len < 15_000 {
                        Style::default()
                    } else {
                        Style::default().fg(Color::Red)
                    };

                    let dimensions_style = if sls_image.width == 320 && sls_image.height == 240 {
                        Style::default()
                    } else {
                        Style::default().fg(Color::Red)
                    };

                    Line::from(vec![
                        Span::raw(format!("{:<10}  ", sls_image.mimetype,)),
                        Span::styled(
                            format!("{:>8}  ", format_size(sls_image.len as u64, DECIMAL)),
                            size_style,
                        ),
                        Span::styled(
                            format!("{}x{}", sls_image.width, sls_image.height),
                            dimensions_style,
                        ),
                    ])
                } else {
                    Line::from("")
                };

                Row::new(vec![
                    Cell::from(format!("{:>4}", svc.scid)),
                    Cell::from(svc.sid.clone()),
                    Cell::from(svc.label.clone()),
                    Cell::from(svc.short_label.clone()),
                    Cell::from(sc_info),
                    Cell::from(svc.format.clone()),
                    Cell::from(dl_info),
                    Cell::from(sls_info),
                ])
                .style(style)
            });

            let table = Table::new(
                rows,
                [
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Length(18),
                    // Constraint::Length(36),
                    Constraint::Fill(1),
                    Constraint::Length(18),
                    Constraint::Length(36),
                    Constraint::Length(7),
                    Constraint::Length(36),
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

            frame.render_stateful_widget(table, layout[2], &mut state.table_state);

            ///////////////////////////////////////////////////////////
            // player
            ///////////////////////////////////////////////////////////

            let current_service = if let Some(scid) = state.selected_scid {
                state.services.iter().find(|svc| svc.scid == scid)
            } else {
                None
            };

            let player_title = match current_service {
                Some(svc) => format!(" Player SC {:>2} - {} ", svc.scid, svc.format),
                None => " Player ".to_string(),
            };

            let player_text = match current_service {
                Some(svc) => format!("{}", svc.label),
                None => "No service selected".to_string(),
            };

            // let player_dl =

            /*
            let player_dl = if let Some(selected) = state.selected_scid {
                state.dl_objects.iter().find(|(scid, _)| *scid == selected)
            } else {
                None
            };

            let player_dl_text = if let Some((_, Some(dl))) = player_dl {
                dl.decode_label()
            } else {
                "-".into()
            };

            let player_dl_title: String = if let Some((_, Some(dl))) = player_dl {
                if dl.get_dl_plus().len() > 0 {
                    " DL+ ".into()
                } else {
                    " DL ".into()
                }
            } else {
                " DL ".into()
            };
            */

            let player_dl = state
                .selected_scid
                .and_then(|selected| state.dl_objects.iter().find(|(scid, _)| *scid == selected))
                .and_then(|(_, dl)| dl.as_ref());

            let player_sls_image = state
                .selected_scid
                .and_then(|selected| state.sls_images.iter().find(|(scid, _)| *scid == selected))
                .and_then(|(_, dl)| dl.as_ref());


            let player_dl_text: Text = match player_dl {
                Some(dl) => {
                    let mut lines = vec![
                        Line::from(dl.decode_label()), // base line: label, normal style
                    ];

                    let dl_plus_tags = dl.get_dl_plus();
                    if !dl_plus_tags.is_empty() {
                        let tags_joined = dl_plus_tags
                            .iter()
                            .map(|tag| format!("{}: {}", tag.kind, tag.value))
                            .collect::<Vec<_>>()
                            .join(" | ");

                        // Add DL+ line with special style
                        lines.push(
                            Line::from(vec![
                                Span::styled(
                                    tags_joined,
                                    Style::default().fg(Color::DarkGray),
                                )
                            ])
                        );
                    }

                    Text::from(lines)
                }
                None => Text::from("-"),
            };


            let player_dl_title: String = player_dl
                .map(|dl| {
                    if !dl.get_dl_plus().is_empty() {
                        " DL+ "
                    } else {
                        " DL "
                    }
                })
                .unwrap_or(" DL ")
                .into();



            let player_left = Paragraph::new(player_text)
                .block(
                    Block::default()
                        .title(player_title)
                        .padding(Padding::horizontal(1))
                        .borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM),
                )
                .wrap(Wrap { trim: true });

            let player_right = Paragraph::new(player_dl_text)
                .block(
                    Block::default()
                        .title(player_dl_title)
                        .padding(Padding::horizontal(1))
                        .borders(Borders::TOP | Borders::BOTTOM),
                )
                .wrap(Wrap { trim: true });

            let player_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(40),
                    Constraint::Min(30),
                    Constraint::Length(48),
                ])
                .split(layout[3]);

            frame.render_widget(player_left, player_layout[0]);
            frame.render_widget(player_right, player_layout[1]);

            let player_sls_text: Text = match player_sls_image {
                Some(sls) => {
                    let lines = vec![
                        Line::from(format!(
                            "{} • {} • {}x{}",
                            sls.mimetype, format_size(sls.len as u64, DECIMAL), sls.width, sls.height
                        )),
                        Line::from(vec![
                            Span::styled(
                                format!("MD5: {}", sls.md5),
                                Style::default().fg(Color::DarkGray),
                            )
                        ])
                    ];
                    Text::from(lines)
                }
                None => Text::from("-"),
            };

            let player_sls = Paragraph::new(player_sls_text)
                .block(
                    Block::default()
                        .title(" SLS ")
                        .padding(Padding::horizontal(1))
                        .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM),
                )
                .wrap(Wrap { trim: true });

            frame.render_widget(player_sls, player_layout[2]);

            if state.show_sls {
                let sls_area = center(
                    frame.area(),
                    Constraint::Length(84.min(area.width)),
                    Constraint::Length(28.min(area.height)),
                );

                // let sls_image = state.table_state.selected().and_then(|selected| {
                //     state
                //         .sls_images
                //         .iter()
                //         .find(|(scid, _)| *scid -1 == selected as u8)
                //         .and_then(|(_, m)| m.clone())
                // });

                let sls_image = state.selected_scid.and_then(|selected| {
                    state
                        .sls_images
                        .iter()
                        .find(|(scid, _)| *scid == selected)
                        .and_then(|(_, m)| m.clone())
                });

                let sls_widget = SLSWidget::new(sls_image);

                frame.render_widget(sls_widget, sls_area);
            }
        })?;

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
                    KeyCode::Char('s') => {
                        state.show_sls = !state.show_sls;
                    }
                    KeyCode::Char('l') => {
                        state.show_meter = !state.show_meter;
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
                TUIEvent::MOTImageReceived(m) => {
                    state.update_mot_image(m);
                }
                TUIEvent::EDISStatsUpdated(s) => {
                    state.update_edi_stats(s);
                }
                #[allow(unreachable_patterns)]
                _ => {}
            }
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
