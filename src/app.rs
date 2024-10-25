use crossterm::event::{self, Event, KeyCode};
use layout::Flex;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Paragraph, Row, Table, TableState},
};
use thiserror::Error;

use crate::{
    config::{Config, ConfigError},
    storage::{Storage, StorageLoadError, StorageRunError},
    Transaction, User,
};

pub mod popups;
use popups::{AddTransaction, Popup};

const MANTRA_INTRO: &str = r"  __       __   ______   __    __        __  ________  _______    ______   
 /  \     /  | /      \ /  \  /  |      /  |/        |/       \  /      \  
 $$  \   /$$ |/$$$$$$  |$$  \ $$ |     /$$/ $$$$$$$$/ $$$$$$$  |/$$$$$$  | 
 $$$  \ /$$$ |$$ |__$$ |$$$  \$$ |    /$$/     $$ |   $$ |__$$ |$$ |__$$ | 
 $$$$  /$$$$ |$$    $$ |$$$$  $$ |   /$$/      $$ |   $$    $$< $$    $$ | 
 $$ $$ $$/$$ |$$$$$$$$ |$$ $$ $$ |  /$$/       $$ |   $$$$$$$  |$$$$$$$$ | 
 $$ |$$$/ $$ |$$ |  $$ |$$ |$$$$ | /$$/        $$ |   $$ |  $$ |$$ |  $$ | 
 $$ | $/  $$ |$$ |  $$ |$$ | $$$ |/$$/         $$ |   $$ |  $$ |$$ |  $$ | 
 $$/      $$/ $$/   $$/ $$/   $$/ $$/          $$/    $$/   $$/ $$/   $$/  ";
const INTRO_HEIGHT: u16 = 8;
const INTRO_WIDTH: u16 = 77;

pub struct App {
    config: Config,
    storage: Storage,
    current_user: Option<User>,
    transactions: Vec<Transaction>,
    table_state: TableState,
    status_text: String,
    popup: Option<Popup>,
    active: bool,
}

#[derive(Error, Debug)]
pub enum AppInitError {
    #[error(transparent)]
    ConfigError(#[from] ConfigError),
    #[error(transparent)]
    StorageLoadError(#[from] StorageLoadError),
    #[error(transparent)]
    StorageRunError(#[from] StorageRunError),
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    StorageRunError(#[from] StorageRunError),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
}

impl App {
    pub async fn init(username: String) -> Result<Self, AppInitError> {
        let config = Config::load_or_create();
        let storage = Storage::load_or_create().await?;
        let user = storage.get_or_create_user(username.to_lowercase()).await?;
        Ok(App {
            config: config.await?,
            transactions: storage.get_transactions(user.id, ..).await?,
            storage,
            current_user: Some(user),
            table_state: TableState::default(),
            status_text: String::new(),
            popup: None,
            active: false,
        })
    }

    pub fn ui(&mut self, frame: &mut Frame<'_>) {
        let area = frame.area();
        if !self.active {
            let text_progress = &MANTRA_INTRO[0..(frame.count() * 5).clamp(0, MANTRA_INTRO.len())];
            let [intro_area, instruct_area] =
                Layout::vertical([Constraint::Length(INTRO_HEIGHT + 4), Constraint::Length(3)])
                    .flex(Flex::Center)
                    .areas(
                        Layout::horizontal([Constraint::Length(INTRO_WIDTH)])
                            .flex(Flex::Center)
                            .areas::<1>(area)[0],
                    );
            let intro_text = Paragraph::new(text_progress)
                .block(Block::bordered().border_type(BorderType::Thick));
            let instruct_text = Paragraph::new("Press any key to start")
                .block(Block::bordered())
                .alignment(Alignment::Center);

            frame.render_widget(intro_text, intro_area);
            frame.render_widget(instruct_text, instruct_area);

            return;
        }
        let widths = [
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Fill(1),
        ];

        let rows: Vec<_> = self
            .transactions
            .iter()
            .map(|trans| {
                Row::new([
                    format!("{}", trans.value),
                    trans.msg.clone(),
                    trans
                        .datetime
                        .assume_utc()
                        .to_offset(self.config.timezone)
                        .format(time::macros::format_description!(
                            "[year]-[month]-[day] [hour]:[minute]"
                        ))
                        .unwrap(),
                ])
            })
            .collect();
        let block = Block::bordered()
            .border_style(Style::new().white())
            .title("MAN/TRA");
        let table_widget = Table::new(rows, widths)
            .block(block)
            .header(Row::new(["Amount", "Note", "Date/Time"]).underlined())
            .highlight_style(Style::new().black().on_white());
        let [table_area, status_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(area);
        frame.render_stateful_widget(&table_widget, table_area, &mut self.table_state);
        frame.render_widget(
            Paragraph::new(self.status_text.clone()).block(Block::bordered().title("Status")),
            status_area,
        );

        if let Some(popup) = &self.popup {
            popup.render_to_frame(table_area, frame);
        }
    }

    pub async fn run(&mut self) -> Result<bool, AppError> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Some(popup) = self.popup.take() {
                self.popup = popup.process_event(self, event::read()?).await?;
            } else if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    if !self.active {
                        self.active = true;
                        return Ok(true);
                    }
                    match key.code {
                        KeyCode::Char('q') => {
                            return Ok(false);
                        }
                        KeyCode::Char('a') => {
                            self.popup = Some(Popup::AddTransaction(AddTransaction::default()));
                        }
                        KeyCode::Char('c') => {
                            self.storage
                                .remove_transactions(&format!(
                                    "user_id = {}",
                                    self.current_user.as_ref().map(|v| v.id).unwrap()
                                ))
                                .await?;

                            self.status_text = String::from("Cleared log");
                            self.update_table().await?;
                        }
                        KeyCode::Char('d') => {
                            if let Some(index) = self.table_state.selected() {
                                let transaction = &self.transactions[index];
                                self.storage
                                    .remove_transactions(&format!("id = {}", transaction.trans_id))
                                    .await?;
                                self.status_text = format!(
                                    "Deleted \"{} | {}\"",
                                    transaction.value, transaction.msg
                                );
                                self.update_table().await?
                            }
                        }
                        KeyCode::Down => self.table_state.select_next(),
                        KeyCode::Up => self.table_state.select_previous(),
                        _ => (),
                    }
                }
            }
        }
        Ok(true)
    }

    pub async fn update_table(&mut self) -> Result<(), AppError> {
        self.transactions = self
            .storage
            .get_transactions(self.current_user.as_ref().map(|v| v.id).unwrap(), ..)
            .await?;
        Ok(())
    }
}
