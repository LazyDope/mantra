//! This module provides the front end application through the [`App`] type
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use layout::Flex;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Paragraph, Row, Table, TableState},
};
use thiserror::Error;

use crate::{
    config::{Config, ConfigError},
    storage::{Storage, StorageLoadError, StorageRunError, Transaction, User},
    CursoredString,
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
const INTRO_HEIGHT: u16 = 9;
const INTRO_WIDTH: u16 = 77;

/// Structure to represent all the running app's state
pub struct App {
    pub data: AppData,
    pub mode: AppMode,
}

/// Shared state for [`App`] between modes
pub struct AppData {
    config: Config,
    storage: Storage,
    current_user: Option<User>,
    transactions: Vec<Transaction>,
    table_state: TableState,
    status_text: String,
    popup: Option<Popup>,
}

/// NewType of [`CursoredString`] for the purposes of displaying new user pane
pub type Username = CursoredString;

/// Error that occurred at App initialization
#[derive(Error, Debug)]
pub enum AppInitError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    StorageLoad(#[from] StorageLoadError),
    #[error(transparent)]
    StorageRun(#[from] StorageRunError),
}

/// Error that occurred while [`App`] is running
#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    StorageRun(#[from] StorageRunError),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
}

/// Modes of [`App`]
pub enum AppMode {
    /// Intro animation sequence, internal field for animation progress
    Intro { animation_progress: usize },
    /// User login prompt, internal field for Username currently being typed
    UserLogin(Username),
    /// Table with log entires for the current user
    LogTable,
    /// App is in the process of closing
    Quitting,
}

impl App {
    /// Initialize a new App, starting with the intro animation then into a login screen
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
            mode: AppMode::Intro {
                animation_progress: 0,
            },
        })
    }

    /// Initialize App with a given username, skipping the login screen
    pub async fn init_with_username(username: String) -> Result<Self, AppInitError> {
        let config = Config::load_or_create();
        let storage = Storage::load_or_create().await?;
        let username = username.to_lowercase();
        storage.create_user(&username).await?;
        let user = storage.get_user(&username).await?;
        Ok(App {
            data: AppData {
                config: config.await?,
                transactions: storage.get_transactions(user.get_id(), ..).await?,
                storage,
                current_user: Some(user),
                table_state: TableState::default(),
                status_text: String::new(),
                popup: None,
            },
            mode: AppMode::Intro {
                animation_progress: 0,
            },
        })
    }

    /// UI for the app, separating based on mode and displaying any popups on top of the current window
    pub fn ui(&mut self, frame: &mut Frame<'_>) {
        match &mut self.mode {
            AppMode::Intro { animation_progress } => {
                self.data.play_intro(frame, animation_progress)
            }
            AppMode::LogTable => self.data.display_log(frame),
            AppMode::UserLogin(username) => username.user_login(frame, self.data.popup.is_some()),
            AppMode::Quitting => (),
        }

        if let Some(popup) = &self.data.popup {
            popup.render_to_frame(frame.area(), frame);
        }
    }

    /// Run the input handling of the app, polling and passing along key events to mode specific functions
    /// App is meant to continue running as long as this returns true, gracefully quitting when
    /// it returns false, or short circuiting any errors
    ///
    /// ```rust,no_run
    /// use mantra::app::App;
    ///
    /// #[async_std::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let mut terminal = ratatui::init();
    ///
    ///     let mut app = App::init().await?;
    ///     while app.run().await? {
    ///         terminal.draw(|frame| app.ui(frame))?;
    ///     }
    ///
    ///     ratatui::restore();
    ///     Ok(())
    /// }
    ///```
    pub async fn run(&mut self) -> Result<bool, AppError> {
        // poll events every 50 milliseconds
        if event::poll(std::time::Duration::from_millis(50))? {
            // popups grab all key events
            if let Some(popup) = self.data.popup.take() {
                self.data.popup = popup.process_event(self, event::read()?).await?;
            } else if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    // modes switch between one another by returning Some(AppMode)
                    // otherwise the current mode is maintained
                    let new_state: Option<AppMode> = match &mut self.mode {
                        AppMode::Intro { .. } => {
                            if self.data.current_user.is_some() {
                                Some(AppMode::LogTable)
                            } else {
                                Some(AppMode::UserLogin(Default::default()))
                            }
                        }
                        AppMode::UserLogin(username) => {
                            self.data.run_user_login(username, key).await?
                        }
                        AppMode::LogTable => self.data.run_table(key).await?,
                        AppMode::Quitting => None,
                    };
                    match new_state {
                        Some(AppMode::Quitting) => return Ok(false),
                        Some(mode) => self.mode = mode,
                        None => (),
                    }
                }
            }
        }
        Ok(true)
    }
}

impl AppData {
    /// Updates the table from the DB, done after making any changes
    pub async fn update_table(&mut self) -> Result<(), AppError> {
        self.transactions = self
            .storage
            .get_transactions(self.current_user.as_ref().map(|v| v.get_id()).unwrap(), ..)
            .await?;
        Ok(())
    }

    /// Play the intro animation on the given [`Frame`]
    pub fn play_intro(&self, frame: &mut Frame<'_>, animation_progress: &mut usize) {
        // animate based on how many frames have passed to give a speeding up effect
        *animation_progress = animation_progress
            .saturating_add(frame.count() / 4)
            .clamp(0, MANTRA_INTRO.len());
        let text_progress = &MANTRA_INTRO[0..*animation_progress];

        // make the heights of each text box match the heights needed for the text and borders, plus some margin for the MAN/TRA text
        let [intro_area, instruct_area] =
            Layout::vertical([Constraint::Length(INTRO_HEIGHT + 3), Constraint::Length(3)])
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

    /// Displays the log in the given [`Frame`]
    pub fn display_log(&mut self, frame: &mut Frame) {
        let widths = [
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Fill(1),
        ];

        // create the iterator of rows from App's vector of transactions
        let rows = self.transactions.iter().map(|trans| {
            Row::new([
                trans.value.to_string(),
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
        });

        // styling and layout
        let block = Block::bordered()
            .border_style(Style::new().white())
            .title("MAN/TRA");
        let [table_area, status_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(frame.area());

        // create table with currency, note, and date+time columns
        let table_widget = Table::new(rows, widths)
            .block(block)
            .header(
                Row::new([self.config.currency.long.as_str(), "Note", "Date/Time"]).underlined(),
            )
            .highlight_style(Style::new().black().on_white());

        frame.render_stateful_widget(&table_widget, table_area, &mut self.table_state);
        frame.render_widget(
            Paragraph::new(self.status_text.clone()).block(Block::bordered().title("Status")),
            status_area,
        );
    }

    /// Handle input for the user login prompt
    /// If the username provided doesn't match to a user already in the db then this opens a new user popup
    pub async fn run_user_login(
        &mut self,
        username: &mut Username,
        key: KeyEvent,
    ) -> Result<Option<AppMode>, AppError> {
        match key.code {
            KeyCode::Left => {
                username.prev();
            }
            KeyCode::Right => {
                username.next();
            }
            KeyCode::Enter if !username.is_empty() => {
                // try to get the user from DB, if this fails show the new user popup
                let username = username.to_lowercase();
                match self.storage.get_user(&username).await {
                    Ok(user) => {
                        self.status_text = format!("Logged in as '{}'", user.get_name());
                        self.current_user = Some(user);
                        self.update_table().await?;
                        return Ok(Some(AppMode::LogTable));
                    }
                    Err(StorageRunError::RecordMissing) => {
                        self.popup = Some(Popup::CreateUser(CreateUser::new(username)))
                    }
                    Err(e) => return Err(e.into()),
                }
            }
            KeyCode::Backspace => username.remove_behind(),
            KeyCode::Delete => username.remove_ahead(),
            KeyCode::Insert => username.inserting = !username.inserting,
            KeyCode::Esc => return Ok(Some(AppMode::Quitting)),
            KeyCode::Char(c) if !c.is_whitespace() => username.insert(c),
            _ => (),
        }
        Ok(None)
    }

    /// Handles input for the table mode
    pub async fn run_table(&mut self, key: KeyEvent) -> Result<Option<AppMode>, AppError> {
        match key.code {
            KeyCode::Down => self.table_state.select_next(),
            KeyCode::Up => self.table_state.select_previous(),
            KeyCode::Esc => return Ok(Some(AppMode::Quitting)),
            KeyCode::Char('q') => {
                return Ok(Some(AppMode::Quitting));
            }
            KeyCode::Char('o') => {
                self.current_user = None;
                self.transactions = vec![];
                return Ok(Some(AppMode::UserLogin(Default::default())));
            }
            KeyCode::Char('a') => {
                self.popup = Some(Popup::AddTransaction(AddTransaction::default()));
            }
            KeyCode::Char('c') => {
                self.storage
                    .remove_transactions(&format!(
                        "user_id = {}",
                        self.current_user.as_ref().map(|v| v.get_id()).unwrap()
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
    /// Displays the user login prompt to the given [`Frame`]
    pub fn user_login(&self, frame: &mut Frame, hide_cursor: bool) {
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

        let username_text = Paragraph::new(self.as_str()).block(username_field);
        if !hide_cursor {
            frame.set_cursor_position(Position::new(
                username_area.x + self.index as u16 + 1,
                username_area.y + 1,
            ));
        }

        frame.render_widget(username_text, username_area);
    }
}
