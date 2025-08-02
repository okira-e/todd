use std::{cell::RefCell, fs::{File, Metadata}, time::{Duration, Instant}};

use color_eyre::{eyre::bail, Result};
use ratatui::{layout::Size, DefaultTerminal}
;
use serde_json::Value;

use crate::{actions::Action, json_tree::JsonTree, widgets::text_input::TextInput};

#[derive(Debug)]
pub enum CurrentScreen {
    ViewingFile,
    Editing,
    Searching,
}

#[derive(Debug, PartialEq)]
pub enum CurrentlyEditing {
    Key,
    Value,
}

#[derive(Debug, PartialEq)]
pub enum EditingMode {
    Inserting,
    Editing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValuePair {
    pub indentation: usize,
    pub key: String,
    pub value: Option<Value>,
    pub is_array_value: bool,
}


#[derive(Debug)]
pub struct ReportedMessage {
    pub message: String,
    pub show_time: Instant,
    pub show_duration: Duration,
    pub kind: ReportedMessageKinds,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ReportedMessageKinds {
    Error,
    Info,
    Debug,
    Warning,
    Success,
}


/// Represents the parent app state.
pub struct App<'a> {
    /// The key element in the popup for inserting new pairs.
    pub key_input: TextInput,
    /// The currently being edited json value.
    pub value_input: TextInput,
    /// The current screen the user is looking at, and will later determine what is rendered.
    pub current_screen: CurrentScreen,
    /// The optional state containing which of the key or value pair the user is editing. It is an option, 
    /// because when the user is not directly editing a key-value pair, this will be set to `None`.
    pub currently_editing: Option<CurrentlyEditing>,
    /// Tracks whether we're inserting new data or editing existing data
    pub editing_mode: EditingMode,
    /// A temporary message to report in the UI to the user.
    pub message_to_report: RefCell<ReportedMessage>,
    pub json_tree: JsonTree,
    /// The search input widget.
    pub search_widget: TextInput,
    /// If there is an active search, this data structure saves all matches found by their (rendered) line numbers.
    pub search_matches: Vec<usize>,
    // @Fix: It's an Option because of tests and maintaining the default() method. Change all the tests to read
    // from a file to make this non-optional.
    pub file_metadata: Option<Metadata>,
    pub size: Size,
    pub file: Option<&'a mut File>,
    running: bool,
}

impl<'a> App<'a> {
    /// Construct the app and sets the JSON data.
    pub fn new(
        json_content: &str, 
        file_metadata: Option<Metadata>, 
        file: Option<&'a mut File>,
        size: Size,
    ) -> Result<Self> {
        let mut app = Self::default();

        app.file_metadata = file_metadata;
        app.file = file;
        app.size = size;
        
        app.json_tree = match JsonTree::new(json_content) {
            Ok(val) => val,
            Err(err) => bail!("Failed to create JSON tree: {}", err),
        };
        
        return Ok(app);
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;

        while self.running {
            terminal.draw(|frame| {
                self.draw(frame);
            })?;

            self.handle_crossterm_events()?;
        }

        return Ok(());
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::AppNavigation(action) => self.handle_app_navigation_actions(action),
            Action::MainView(action) => self.json_tree.handle_action(action),
            Action::Editing(action) => self.handle_editing_actions(action),
            Action::Searching(action) => self.handle_searching_actions(action),
            Action::App(action) => self.handle_app_actions(action),
        }
    }
    
    pub fn toggle_editing(&mut self) {
        // Switch between key and value keys unless we're not on either then toggle to key.
        match &self.currently_editing {
            Some(edit_mode) => {
                match edit_mode {
                    CurrentlyEditing::Key => {
                        self.currently_editing = Some(CurrentlyEditing::Value);
                        self.key_input.is_focused = false;
                        self.value_input.is_focused = true;
                    }
                    CurrentlyEditing::Value => {
                        // There is no key input to toggle into if we're inserting to an array.
                        if !self.json_tree.is_inside_array() {
                            self.currently_editing = Some(CurrentlyEditing::Key);
                            self.key_input.is_focused = true;
                            self.value_input.is_focused = false;
                        }
                    }
                };
            }
            None => {
                self.current_screen = CurrentScreen::Editing;
                self.editing_mode = EditingMode::Inserting;
                if !self.json_tree.is_inside_array() { 
                    self.currently_editing = Some(CurrentlyEditing::Key);
                    self.key_input.is_focused = true;
                    self.value_input.is_focused = false;
                } else { 
                    self.currently_editing = Some(CurrentlyEditing::Value);
                    self.key_input.is_focused = false;
                    self.value_input.is_focused = true;
                }
            }
        };
    }
    
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn get_focused_text_input(&mut self) -> Option<&mut TextInput> {
        if let Some(currently_editing) = &self.currently_editing {
            match currently_editing {
                CurrentlyEditing::Key => {
                    if self.key_input.is_focused {
                        return Some(&mut self.key_input);
                    }
                }
                CurrentlyEditing::Value => {
                    if self.value_input.is_focused {
                        return Some(&mut self.value_input);
                    }
                }
            }
        }
        
        return None;
    }
    
    pub fn report(&self, message: String, kind: ReportedMessageKinds, duration: Duration) {
        *self.message_to_report.borrow_mut() = ReportedMessage {
            message,
            show_time: Instant::now(),
            show_duration: duration,
            kind,
        };
    }
}

impl Default for App<'_> {
    fn default() -> Self {
        Self {
            running: false,
            key_input: TextInput::new(Some("Key")),
            value_input: TextInput::new(Some("Value")),
            json_tree: JsonTree::default(),
            search_widget: TextInput::new(Some("Look For")),
            search_matches: vec![],
            message_to_report: RefCell::new(ReportedMessage {
                message: "".to_string(),
                show_time: Instant::now(),
                kind: ReportedMessageKinds::Info,
                show_duration: Duration::from_secs(0),
            }),
            current_screen: CurrentScreen::ViewingFile,
            currently_editing: None,
            editing_mode: EditingMode::Inserting,
            file_metadata: None,
            file: None,
            size: Size::default(),
        }
    }
}
