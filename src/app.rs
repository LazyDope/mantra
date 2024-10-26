use crossterm::event::{self, Event, KeyCode, KeyEvent};
use layout::Flex;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Paragraph, Row, Table, TableState},
};
use text::ToText;
use thiserror::Error;

use crate::{
    config::{Config, ConfigError},
    storage::{Storage, StorageLoadError, StorageRunError},
    CursoredString, Transaction, User,
};

pub mod popups;
use popups::{AddTransaction, CreateUser, Popup};

const MANTRA_INTRO: &str = r"  __       __   ______   __    __        __  ________  _______    ______
 /  \     /  | /      \ /  \  /  |      /  |/        |/       \  /      \ 
 $$  \   /$$ |/$$$$$$  |$$  \ $$ |     /$$/ $$$$$$$$/ $$$$$$$  |/$$$$$$  |
 $$$  \ /$$$ |$$ |__$$ |$$$  \$$ |    /$$/     $$ |   $$ |__$$ |$$ |__$$ |
 $$$$  /$$$$ |$$    $$ |$$$$  $$ |   /$$/      $$ |   $$    $$< $$    $$ |
 $$ $$ $$/$$ |$$$$$$$$ |$$ $$ $$ |  /$$/       $$ |   $$$$$$$  |$$$$$$$$ |
 $$ |$$$/ $$ |$$ |  $$ |$$ |$$$$ | /$$/        $$ |   $$ |  $$ |$$ |  $$ |
 $$ | $/  $$ |$$ |  $$ |$$ | $$$ |/$$/         $$ |   $$ |  $$ |$$ |  $$ |
 $$/      $$/ $$/   $$/ $$/   $$/ $$/          $$/    $$/   $$/ $$/   $$/";
const INTRO_HEIGHT: u16 = 8;
const INTRO_WIDTH: u16 = 77;

pub struct App {
    pub data: AppData,
    pub state: AppState,
}

pub struct AppData {
    config: Config,
    storage: Storage,
    current_user: Option<User>,
    transactions: Vec<Transaction>,
    table_state: TableState,
    status_text: String,
    popup: Option<Popup>,
}

#[derive(Default)]
pub struct Username(CursoredString);

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

pub enum AppState {
    Intro { animation_progress: usize },
    UserLogin(Username),
    LogTable,
    Quitting,
}

impl App {
    pub async fn init() -> Result<Self, AppInitError> {
        let config = Config::load_or_create();
        let storage = Storage::load_or_create().await?;
        Ok(App {
            data: AppData {
                config: config.await?,
                transactions: vec![],
                storage,
                current_user: None,
                table_state: TableState::default(),
                status_text: String::new(),
                popup: None,
            },
            state: AppState::Intro {
                animation_progress: 0,
            },
        })
    }

    pub async fn init_with_username(username: String) -> Result<Self, AppInitError> {
        let config = Config::load_or_create();
        let storage = Storage::load_or_create().await?;
        let user = storage.get_or_create_user(username.to_lowercase()).await?;
        Ok(App {
            data: AppData {
                config: config.await?,
                transactions: storage.get_transactions(user.id, ..).await?,
                storage,
                current_user: Some(user),
                table_state: TableState::default(),
                status_text: String::new(),
                popup: None,
            },
            state: AppState::Intro {
                animation_progress: 0,
            },
        })
    }

    pub fn ui(&mut self, frame: &mut Frame<'_>) {
        match &mut self.state {
            AppState::Intro { animation_progress } => {
                self.data.play_intro(frame, animation_progress)
            }
            AppState::LogTable => self.data.display_log(frame),
            AppState::UserLogin(username) => username.user_login(frame, self.data.popup.is_some()),
            AppState::Quitting => (),
        }

        if let Some(popup) = &self.data.popup {
            popup.render_to_frame(frame.area(), frame);
        }
    }

    pub async fn run(&mut self) -> Result<bool, AppError> {
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Some(popup) = self.data.popup.take() {
                self.data.popup = popup.process_event(self, event::read()?).await?;
            } else if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    let new_state: Option<AppState> = match &mut self.state {
                        AppState::Intro { .. } => {
                            if self.data.current_user.is_some() {
                                Some(AppState::LogTable)
                            } else {
                                Some(AppState::UserLogin(Default::default()))
                            }
                        }
                        AppState::UserLogin(username) => {
                            self.data.run_user_login(username, key).await?
                        }
                        AppState::LogTable => self.data.run_table(key).await?,
                        AppState::Quitting => None,
                    };
                    if let Some(state) = new_state {
                        self.state = state
                    };
                    return Ok(!matches!(self.state, AppState::Quitting));
                }
            }
        }
        Ok(true)
    }
}

impl AppData {
    pub async fn update_table(&mut self) -> Result<(), AppError> {
        self.transactions = self
            .storage
            .get_transactions(self.current_user.as_ref().map(|v| v.id).unwrap(), ..)
            .await?;
        Ok(())
    }

    fn play_intro(&self, frame: &mut Frame<'_>, animation_progress: &mut usize) {
        *animation_progress = animation_progress
            .saturating_add(frame.count() / 4)
            .clamp(0, MANTRA_INTRO.len());
        let text_progress = &MANTRA_INTRO[0..*animation_progress];
        let [intro_area, instruct_area] =
            Layout::vertical([Constraint::Length(INTRO_HEIGHT + 4), Constraint::Length(3)])
                .flex(Flex::Center)
                .areas(
                    Layout::horizontal([Constraint::Length(INTRO_WIDTH)])
                        .flex(Flex::Center)
                        .areas::<1>(frame.area())[0],
                );
        let intro_text =
            Paragraph::new(text_progress).block(Block::bordered().border_type(BorderType::Thick));
        let instruct_text = Paragraph::new("Press any key to start")
            .block(Block::bordered())
            .alignment(Alignment::Center);

        frame.render_widget(intro_text, intro_area);
        frame.render_widget(instruct_text, instruct_area);
    }

    fn display_log(&mut self, frame: &mut Frame) {
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
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(frame.area());
        frame.render_stateful_widget(&table_widget, table_area, &mut self.table_state);
        frame.render_widget(
            Paragraph::new(self.status_text.clone()).block(Block::bordered().title("Status")),
            status_area,
        );
    }

    async fn run_user_login(
        &mut self,
        username: &mut Username,
        key: KeyEvent,
    ) -> Result<Option<AppState>, AppError> {
        match key.code {
            KeyCode::Left => {
                username.0.prev();
            }
            KeyCode::Right => {
                username.0.next();
            }
            KeyCode::Enter if !username.0.text.is_empty() => {
                match self.storage.get_user(username.0.text.to_lowercase()).await {
                    Ok(user) => {
                        self.status_text = format!("Logged in as '{}'", user.name);
                        self.current_user = Some(user);
                        self.update_table().await?;
                        return Ok(Some(AppState::LogTable));
                    }
                    Err(StorageRunError::RecordMissing) => {
                        self.popup = Some(Popup::CreateUser(CreateUser::new(Box::from(
                            username.0.text.as_str(),
                        ))))
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            KeyCode::Backspace => username.0.remove_behind(),
            KeyCode::Delete => username.0.remove_ahead(),
            KeyCode::Insert => username.0.inserting = !username.0.inserting,
            KeyCode::Esc => return Ok(Some(AppState::Quitting)),
            KeyCode::Char(c) if !c.is_whitespace() => username.0.insert(c),
            _ => (),
        }
        Ok(None)
    }

    async fn run_table(&mut self, key: KeyEvent) -> Result<Option<AppState>, AppError> {
        match key.code {
            KeyCode::Down => self.table_state.select_next(),
            KeyCode::Up => self.table_state.select_previous(),
            KeyCode::Char('q') => {
                return Ok(Some(AppState::Quitting));
            }
            KeyCode::Char('o') => {
                self.current_user = None;
                self.transactions = vec![];
                return Ok(Some(AppState::UserLogin(Default::default())));
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
                    self.status_text =
                        format!("Deleted \"{} | {}\"", transaction.value, transaction.msg);
                    self.update_table().await?
                }
            }
            _ => (),
        }
        Ok(None)
    }
}

impl Username {
    fn user_login(&self, frame: &mut Frame, hide_cursor: bool) {
        const USERNAME_HEIGHT: u16 = 1;
        const BORDER_SIZE: u16 = 1;

        let [area] = Layout::vertical([Constraint::Length(USERNAME_HEIGHT + 4 * BORDER_SIZE)])
            .flex(Flex::Center)
            .areas(frame.area());
        let [area] = Layout::horizontal([Constraint::Percentage(40)])
            .flex(Flex::Center)
            .areas(area);
        let block = Block::bordered().title("Login");
        frame.render_widget(block, area);
        let area = area.inner(Margin::new(BORDER_SIZE, BORDER_SIZE));
        let [username_area] =
            Layout::vertical([Constraint::Length(USERNAME_HEIGHT + BORDER_SIZE * 2)]).areas(area);

        let username_field = Block::bordered()
            .title("Username")
            .style(Style::default().bg(Color::LightYellow).fg(Color::Black));

        let username_text = Paragraph::new(self.0.text.to_text()).block(username_field);
        if !hide_cursor {
            frame.set_cursor_position(Position::new(
                username_area.x + self.0.index as u16 + 1,
                username_area.y + 1,
            ));
        }

        frame.render_widget(username_text, username_area);
    }
}
