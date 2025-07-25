use std::{cell::RefCell, fs::{File, Metadata}, io::Seek, time::{Duration, Instant}};

use color_eyre::{eyre::bail, Result};
use ratatui::{layout::Size, widgets::ScrollbarState, DefaultTerminal}
;
use serde_json::Value;

use crate::{actions::{Action, AppNavigationAction, CursorDirection, EditingAction, MainViewActions, SearchingAction, SystemAction}, utils::json::{get_nested_object_to_insert_into, get_current_value_at_position}, widgets::text_input::TextInput};

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
#[derive(Debug)]
pub struct App<'a> {
    /// The key element in the popup for inserting new pairs.
    pub key_input: TextInput,
    /// The currently being edited json value.
    pub value_input: TextInput,
    /// The search input widget.
    pub search_widget: TextInput,
    /// If there is an active search, this data structure saves all matches found by their (rendered) line numbers.
    pub search_matches: Vec<usize>,
    /// The representation of the json file data. It could be an array or an object at the top level.
    pub json: Value,
    /// Holds all the pairs serialized out of the JSON. Has empty pairs to represent a line separator 
    /// for the beginning of an array value.
    pub json_pairs: Vec<ValuePair>,
    /// The current screen the user is looking at, and will later determine what is rendered.
    pub current_screen: CurrentScreen,
    /// The optional state containing which of the key or value pair the user is editing. It is an option, 
    /// because when the user is not directly editing a key-value pair, this will be set to `None`.
    pub currently_editing: Option<CurrentlyEditing>,
    /// Tracks whether we're inserting new data or editing existing data
    pub editing_mode: EditingMode,
    /// Keeps track of where the current focused line is. Represents a line in the UI. So if the UI
    /// needs an empty line, you will find it at it. It doesn't represent the actual count of paris in the JSON.
    pub line_at_cursor: usize,
    /// A temporary message to report in the UI to the user.
    pub message_to_report: RefCell<ReportedMessage>,
    /// The total number of lines drawn (counts nested objects).
    pub lines_count: usize,
    pub viewport_lines_count: usize,
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
    pub scrolled_so_far: usize,
    // @Fix: It's an Option because of tests and maintaining the default() method. Change all the tests to read
    // from a file to make this non-optional.
    pub file_metadata: Option<Metadata>,
    pub size: Size,
    file: Option<&'a mut File>,
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

        let json = match serde_json::from_str(json_content) {
            Ok(value) => value,
            Err(err) => bail!("Failed to parse JSON: {}", err)
        };
        app.json = json;
        app.file_metadata = file_metadata;
        app.file = file;
        app.size = size;

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
            Action::MainView(action) => self.handle_main_view_messages(action),
            Action::Editing(action) => self.handle_editing_actions(action),
            Action::Searching(action) => self.handle_searching_actions(action),
            Action::App(action) => self.handle_app_actions(action),
        }
    }
    
    /// Inserts the from the user popup to the file/data.
    pub fn insert_new_data_from_user_input(&mut self) {
        match self.is_inside_array() {
            true => {
                if self.value_input.content().to_string().is_empty() {
                    return;
                }
            }
            false => {
                if self.key_input.content().to_string().is_empty() || self.value_input.content().to_string().is_empty() {
                    return;
                }
            }
        };
        
        
        let value = serde_json::to_value(self.value_input.content()).unwrap();
        
        // Serialize/Parse the value to its correct type by trying to parse it.
        // The input text component it comes from treats & makes it a string
        // because that's how it views and modify it. But it may be a number
        // so we insert it as such.
        // The result is still a value, but it's now constructed with information
        // about what it really is instead of saying a Value::String for everything.
        let value: Value = match value {
            Value::String(s) => {
                if s.parse::<f64>().is_ok() {
                    Value::Number(s.parse().unwrap())
                } else if s.parse::<bool>().is_ok() {
                    Value::Bool(s.parse().unwrap())
                } else {
                    Value::String(s)
                }
            },
            _ => value, // The value is an object or an array. Not even a string.
        };
        
        let (object_to_insert_into, index) = get_nested_object_to_insert_into(self.line_at_cursor_without_empty_lines(), &mut self.json);
        if let Some(object_to_insert_into) = object_to_insert_into {
            
            match object_to_insert_into {
                Value::Object(map) => {
                    // Check if we're at the last index. If yes, just insert, otherwise, insert safely
                    // at `index + 1`.
                    if index > map.len() - 2 {
                        map.insert(
                            self.key_input.content().to_string(),
                            value,
                        );
                    } else {
                        map.shift_insert(
                            index + 1,
                            self.key_input.content().to_string(),
                            value,
                        );
                    }
                },
                Value::Array(values) => {
                    values.insert(index + 1, value);
                },
                _ => {}
            }
        }
        
        self.report(
            format!("Inserted new key-value pair: {} -> {}", self.key_input.content(), self.value_input.content()),
            ReportedMessageKinds::Success,
            Duration::from_secs(3)
        );

        if let Some(file) = self.file.as_mut() {
            // Clear the file by truncating it to 0 bytes
            if let Err(err) = file.set_len(0) {
                self.report(
                    format!("Failed to clear file: {}", err), 
                    ReportedMessageKinds::Error, 
                    Duration::from_secs(3)
                );
                return;
            }
            
            // Reset file position to the beginning
            if let Err(err) = file.seek(std::io::SeekFrom::Start(0)) {
                self.report(
                    format!("Failed to reset file position: {}", err), 
                    ReportedMessageKinds::Error, 
                    Duration::from_secs(3)
                );
                return;
            }
            
            // Write the new JSON content directly to the file
            if let Err(err) = serde_json::to_writer_pretty(file, &self.json) {
                self.report(
                    format!("Failed to save changes: {}", err), 
                    ReportedMessageKinds::Error, 
                    Duration::from_secs(3)
                );
            }
        }

        self.update(Action::AppNavigation(AppNavigationAction::ToViewingScreen));
    }

    /// Starts editing an existing value at the current cursor position.
    pub fn start_editing_existing_value(&mut self) {
        // Check if we're on a line that has an actual value (not an object/array header)
        if self.line_at_cursor >= self.json_pairs.len() {
            self.report(
                "No value to edit at current position".to_string(),
                ReportedMessageKinds::Error,
                Duration::from_secs(2)
            );
            return;
        }

        let current_pair = &self.json_pairs[self.line_at_cursor];
        
        // Can't edit object/array headers, only actual values
        if current_pair.value.is_none() {
            self.report(
                "Cannot edit object or array headers".to_string(),
                ReportedMessageKinds::Error,
                Duration::from_secs(2)
            );
            return;
        }

        // Get the current value and key information
        let (_, key, current_value, _) = get_current_value_at_position(
            self.line_at_cursor_without_empty_lines(), 
            &self.json
        );

        if let Some(value) = current_value {
            // Set editing mode
            self.editing_mode = EditingMode::Editing;
            self.current_screen = CurrentScreen::Editing;

            // Populate input fields with current values
            if let Some(key) = key {
                // We're editing a key-value pair in an object
                self.key_input.set_content(&key);
                self.currently_editing = Some(CurrentlyEditing::Key);
                self.key_input.is_focused = true;
                self.value_input.is_focused = false;
            } else {
                // We're editing a value in an array
                self.key_input.clear();
                self.currently_editing = Some(CurrentlyEditing::Value);
                self.key_input.is_focused = false;
                self.value_input.is_focused = true;
            }

            // Set the value input with the current value as a string
            let value_str = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "null".to_string(),
                _ => serde_json::to_string(value).unwrap_or_default()
            };
            self.value_input.set_content(&value_str);

            self.report(
                "Editing existing value".to_string(),
                ReportedMessageKinds::Info,
                Duration::from_secs(2)
            );
        } else {
            self.report(
                "Could not find value to edit".to_string(),
                ReportedMessageKinds::Error,
                Duration::from_secs(2)
            );
        }
    }

    /// Updates an existing value based on user input.
    pub fn update_existing_data_from_user_input(&mut self) {
        // Validate input
        let is_array_item = self.is_inside_array();
        
        if is_array_item {
            if self.value_input.content().to_string().is_empty() {
                return;
            }
        } else {
            if self.key_input.content().to_string().is_empty() || self.value_input.content().to_string().is_empty() {
                return;
            }
        }

        // Parse the new value
        let new_value = serde_json::to_value(self.value_input.content()).unwrap();
        let new_value: Value = match new_value {
            Value::String(s) => {
                if s.parse::<f64>().is_ok() {
                    Value::Number(s.parse().unwrap())
                } else if s.parse::<bool>().is_ok() {
                    Value::Bool(s.parse().unwrap())
                } else if s == "null" {
                    Value::Null
                } else {
                    Value::String(s)
                }
            },
            _ => new_value,
        };

        // Get the parent object and update the value
        let (object_to_update, index) = get_nested_object_to_insert_into(
            self.line_at_cursor_without_empty_lines(), 
            &mut self.json
        );

        if let Some(object_to_update) = object_to_update {
            match object_to_update {
                Value::Object(map) => {
                    // Get the key at the current index
                    if let Some(old_key) = map.keys().nth(index).cloned() {
                        let new_key = self.key_input.content().to_string();
                        
                        // If key changed, we need to remove old and insert new
                        if old_key != new_key {
                            map.shift_remove(&old_key);
                            map.shift_insert(index, new_key.clone(), new_value.clone());
                            
                            self.report(
                                format!("Updated key-value pair: {} -> {}", new_key, self.value_input.content()),
                                ReportedMessageKinds::Success,
                                Duration::from_secs(3)
                            );
                        } else {
                            // Just update the value
                            let old_key_clone = old_key.clone();
                            map.insert(old_key, new_value.clone());
                            
                            self.report(
                                format!("Updated value: {} -> {}", old_key_clone, self.value_input.content()),
                                ReportedMessageKinds::Success,
                                Duration::from_secs(3)
                            );
                        }
                    }
                },
                Value::Array(values) => {
                    if index < values.len() {
                        values[index] = new_value.clone();
                        
                        self.report(
                            format!("Updated array value: {}", self.value_input.content()),
                            ReportedMessageKinds::Success,
                            Duration::from_secs(3)
                        );
                    }
                },
                _ => {
                    self.report(
                        "Cannot update this value".to_string(),
                        ReportedMessageKinds::Error,
                        Duration::from_secs(3)
                    );
                    return;
                }
            }
        } else {
            self.report(
                "Could not find parent to update value".to_string(),
                ReportedMessageKinds::Error,
                Duration::from_secs(3)
            );
            return;
        }

        // Save to file if available
        if let Some(file) = self.file.as_mut() {
            // Clear the file by truncating it to 0 bytes
            if let Err(err) = file.set_len(0) {
                self.report(
                    format!("Failed to clear file: {}", err), 
                    ReportedMessageKinds::Error, 
                    Duration::from_secs(3)
                );
                return;
            }
            
            // Reset file position to the beginning
            if let Err(err) = file.seek(std::io::SeekFrom::Start(0)) {
                self.report(
                    format!("Failed to reset file position: {}", err), 
                    ReportedMessageKinds::Error, 
                    Duration::from_secs(3)
                );
                return;
            }
            
            // Write the new JSON content directly to the file
            if let Err(err) = serde_json::to_writer_pretty(file, &self.json) {
                self.report(
                    format!("Failed to save changes: {}", err), 
                    ReportedMessageKinds::Error, 
                    Duration::from_secs(3)
                );
            }
        }

        // Reset editing mode and return to viewing
        self.editing_mode = EditingMode::Inserting;
        self.update(Action::AppNavigation(AppNavigationAction::ToViewingScreen));
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
                        if !self.is_inside_array() { 
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
                if !self.is_inside_array() { 
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
    
    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Inserts all the data recursively to the app. It also calculates and sets the
    /// indentations for the lines based on the nest-ness of the data.
    pub fn insert_data_to_tree(
        &self, 
        pairs: &mut Vec<ValuePair>,
        data: &Value,
        mut indentation_counter: usize,
    ) -> usize {
        indentation_counter += 1;
        let mut lines_count = 0;

        // Check if the root element of the file is an objet or an array. Then recursively
        // walk the tree.
        if data.is_object() {
            for (key, value) in data.as_object().unwrap() {
                self.walk_data_tree_for_json(key, value, pairs, &mut lines_count, indentation_counter);
            }
        } else {
            for value in data.as_array().unwrap() {
                self.walk_data_tree_for_json("", value, pairs, &mut lines_count, indentation_counter);
            }
        }
        
        return lines_count;
    }
    
    /// Tells us if the cursor is currently inside an array parent.
    pub fn is_inside_array(&mut self) -> bool {
        let (object_to_insert_into, _) = get_nested_object_to_insert_into(self.line_at_cursor_without_empty_lines(), &mut self.json);
        
        return match object_to_insert_into {
            Some(val) => val.is_array(),
            None => false,
        };
    }
    
    /// Returns another version of line_at_cursor that doesn't count empty representation lines.
    /// Useful for example when we want to step into the json with actual steps count.
    fn line_at_cursor_without_empty_lines(&self) -> usize {
        let mut i = 0;
        let mut empty_lines = 0;
        for pair in self.json_pairs.iter() {
            if i == self.line_at_cursor {
                break;
            }
            
            // Some pairs are None because they represent an empty line in the UI.
            if pair.key == "" && pair.value.is_none() {
                empty_lines += 1;
            }
            
            i += 1;
        }
        
        return self.line_at_cursor.saturating_sub(empty_lines);
    }
    
    /// Actually does the walking and inserting of all values in the json.
    fn walk_data_tree_for_json(
        &self, 
        key: &str, 
        value: &Value, 
        pairs: &mut Vec<ValuePair>,
        lines_count: &mut usize,
        indentation_counter: usize,
    ) {
        if value.is_object() {
            *lines_count += 1;
            
            if value.as_object().unwrap().len() == 0 {
                pairs.push(
                    ValuePair {
                        indentation: indentation_counter, 
                        key: key.to_owned(), 
                        value: None,
                        is_array_value: false,
                    }
                );
            } else {
                pairs.push(
                    ValuePair {
                        indentation: indentation_counter, 
                        key: key.to_owned(), 
                        value: None,
                        is_array_value: false,
                    }
                );
                
                *lines_count += self.insert_data_to_tree(
                    pairs,
                    &value,
                    indentation_counter
                );
            }
        } else if value.is_array() {
            *lines_count += 1;
            pairs.push(
                ValuePair { 
                    indentation: indentation_counter, 
                    key: key.to_owned(), 
                    value: None,
                    is_array_value: false,
                }
            );
            
            // Insert all values in the array at once with one more indentation level. No recursion
            // needed.
            for it in value.as_array().unwrap() {
                self.walk_data_tree_for_json("", it, pairs, lines_count, indentation_counter + 1);
            }
        } else {
            *lines_count += 1;
            pairs.push(
                ValuePair {
                    indentation: indentation_counter, 
                    key: key.to_owned(), 
                    value: Some(serde_json::from_value(value.clone()).unwrap()),
                    is_array_value: if key == "" { true } else { false }, // We're in an array.
                }
            );
        }
    }

    fn handle_app_navigation_actions(&mut self, action: AppNavigationAction) {
        match action {
            AppNavigationAction::ToViewingScreen => {
                self.key_input.is_focused = false;
                self.value_input.is_focused = false;
                self.currently_editing = None;
                self.editing_mode = EditingMode::Inserting;
                self.key_input.clear();
                self.value_input.clear();
                self.current_screen = CurrentScreen::ViewingFile;
            },
            AppNavigationAction::ToEditingScreen => {
                self.toggle_editing();
            },
            AppNavigationAction::ToSearchingWidget => {
                self.current_screen = CurrentScreen::Searching;
            },
        }
    }
    
    // Call this when you update `self.vertical_scroll`. It updates the scrollbar state accordingly without borrowing Self.
    fn set_vertical_scroll_state(scroll_state: &mut ScrollbarState, scroll: usize, viewport_lines_count: usize) {
        *scroll_state = scroll_state.position(scroll + if scroll != 0 { viewport_lines_count / 2 } else { 0 });
    }
    
    fn handle_main_view_messages(&mut self, action: MainViewActions) {
        let scroll_offset = 5;

        match action {
            MainViewActions::MoveDown => {
                if self.line_at_cursor + 1 < self.lines_count {
                    self.line_at_cursor += 1;

                    if self.is_current_line_at_viewport_end_with_offset(Some(scroll_offset)) {
                        self.vertical_scroll += 1;
                        App::set_vertical_scroll_state(&mut self.vertical_scroll_state, self.vertical_scroll, self.viewport_lines_count);
                        self.scrolled_so_far = self.vertical_scroll;
                    }
                }
            }
            MainViewActions::MoveUp => {
                self.line_at_cursor = self.line_at_cursor.saturating_sub(1);
                
                if self.is_current_line_at_viewport_start_with_offset(Some(scroll_offset)) {
                    self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                    App::set_vertical_scroll_state(&mut self.vertical_scroll_state, self.vertical_scroll, self.viewport_lines_count);
                    self.scrolled_so_far = self.vertical_scroll;
                }
            }
            MainViewActions::MoveToTop => {
                if self.lines_count == 0 {
                    return;
                }

                self.line_at_cursor = 0;
                self.vertical_scroll = 0;
                App::set_vertical_scroll_state(&mut self.vertical_scroll_state, self.vertical_scroll, self.viewport_lines_count);
                self.scrolled_so_far = self.vertical_scroll;
            }
            MainViewActions::MoveToBottom => {
                if self.lines_count == 0 {
                    return;
                }

                self.line_at_cursor = self.lines_count.saturating_sub(1);
                self.vertical_scroll = self.lines_count.saturating_sub(self.viewport_lines_count);
                App::set_vertical_scroll_state(&mut self.vertical_scroll_state, self.vertical_scroll, self.viewport_lines_count);
                self.scrolled_so_far = self.vertical_scroll;
            }
            MainViewActions::MoveHalfPageDown => {
                if self.lines_count == 0 {
                    return;
                }

                if self.line_at_cursor < (self.lines_count.saturating_sub(self.viewport_lines_count / 2)) {
                    self.line_at_cursor += self.viewport_lines_count / 2;
                    self.vertical_scroll += self.viewport_lines_count / 2;
                    self.scrolled_so_far += self.viewport_lines_count / 2;
                } else {
                    self.line_at_cursor += self.lines_count - self.line_at_cursor - 1;
                    self.vertical_scroll = self.lines_count.saturating_sub(self.viewport_lines_count - (self.viewport_lines_count / 2));
                    self.scrolled_so_far = self.vertical_scroll;
                }

                App::set_vertical_scroll_state(&mut self.vertical_scroll_state, self.vertical_scroll, self.viewport_lines_count);
            },
            MainViewActions::MoveHalfPageUp => {
                if self.lines_count == 0 {
                    return;
                }

                if self.line_at_cursor > self.viewport_lines_count / 2 {
                    self.line_at_cursor = self.line_at_cursor.saturating_sub(self.viewport_lines_count / 2);
                    self.vertical_scroll = self.vertical_scroll.saturating_sub(self.viewport_lines_count / 2);
                    App::set_vertical_scroll_state(&mut self.vertical_scroll_state, self.vertical_scroll, self.viewport_lines_count);
                    self.scrolled_so_far = self.vertical_scroll;
                } else {
                    self.update(Action::MainView(MainViewActions::MoveToTop));
                }
            },
        }
    }
    
    fn handle_editing_actions(&mut self, edit_msg: EditingAction) {
        match edit_msg {
            EditingAction::SwitchToKey => {
                self.currently_editing = Some(CurrentlyEditing::Key);
            },
            EditingAction::SwitchToValue => {
                self.currently_editing = Some(CurrentlyEditing::Value);
            },
            EditingAction::EditExisting => {
                self.start_editing_existing_value();
            },
            // @Cleanup: The below four events should be divided into KeyInput(InputAction)
            EditingAction::AppendChar(c) => {
                if let Some(currently_editing) = &self.currently_editing {
                    match currently_editing {
                        CurrentlyEditing::Key => {
                            if self.key_input.is_focused {
                                self.update(Action::Editing(EditingAction::AppendToKey(c)));
                            }
                        }
                        CurrentlyEditing::Value => {
                            if self.value_input.is_focused {
                                self.update(Action::Editing(EditingAction::AppendToValue(c)));
                            }
                        }
                    }
                }
            }
            EditingAction::MoveCursor(direction) => {
                match direction {
                    CursorDirection::Left => {
                        if let Some(focused_text_input) = self.get_focused_text_input() {
                            focused_text_input.move_cursor_left();
                        }
                    },
                    CursorDirection::Right => {
                        if let Some(focused_text_input) = self.get_focused_text_input() {
                            focused_text_input.move_cursor_right();
                        }
                    },
                }
            }
            EditingAction::AppendToKey(c) => {
                self.key_input.append_char(c);
            },
            EditingAction::AppendToValue(c) => {
                self.value_input.append_char(c);
            },
            EditingAction::PopFromKey => {
                if self.key_input.is_focused {
                    self.key_input.delete_char();
                }
            },
            EditingAction::PopFromValue => {
                if self.value_input.is_focused {
                    self.value_input.delete_char();
                }
            },
            EditingAction::Submit => {
                match self.editing_mode {
                    EditingMode::Inserting => self.insert_new_data_from_user_input(),
                    EditingMode::Editing => self.update_existing_data_from_user_input(),
                }
            },
        }
    }
    
    fn handle_searching_actions(&mut self, action: SearchingAction) {
        match action {
            SearchingAction::AppendChar(c) => {
                self.update(Action::Searching(SearchingAction::ClearMatches)); // Clear previous matches before the new ones with the new character.
                self.search_widget.append_char(c);
            }
            SearchingAction::MoveCursor(direction) => {
                match direction {
                    CursorDirection::Left => {
                        self.search_widget.move_cursor_left();
                    },
                    CursorDirection::Right => {
                        self.search_widget.move_cursor_right();
                    },
                }
            }
            SearchingAction::PopChar => {
                self.search_widget.delete_char();
            },
            SearchingAction::ClearSearch => {
                self.search_widget.clear();
                self.search_matches = vec![];
            }
            SearchingAction::GoToNextMatch => {
                if self.search_widget.content().is_empty() {
                    return;
                }
                
                let mut found = false;
                for (i, line_match) in self.search_matches.iter().enumerate() {
                    if self.line_at_cursor < *line_match {
                        found = true;

                        let line_diff = line_match.saturating_sub(self.vertical_scroll);
                        if line_diff > (self.viewport_lines_count.saturating_sub(self.viewport_lines_count / 4)) {
                            self.vertical_scroll += line_diff.saturating_sub(self.viewport_lines_count / 2);
                            App::set_vertical_scroll_state(&mut self.vertical_scroll_state, self.vertical_scroll, self.viewport_lines_count);
                            self.scrolled_so_far = self.vertical_scroll;
                        }

                        self.line_at_cursor = *line_match;

                        self.report(
                            format!("Match {} out of {}", i + 1, self.search_matches.len()),
                            ReportedMessageKinds::Info,
                            Duration::from_secs(1)
                        );
                        
                        break;
                    }
                }
                
                if !found {
                    self.report(
                        format!("No more matches"),
                        ReportedMessageKinds::Error,
                        Duration::from_secs(1)
                    );
                }
            }
            SearchingAction::GoToPrevMatch => {
                if self.search_widget.content().is_empty() {
                    return;
                }
                
                let mut found = false;
                for (i, line_match) in self.search_matches.iter().rev().enumerate() {
                    if self.line_at_cursor > *line_match {
                        found = true;

                        // let line_diff = self.line_at_cursor.saturating_sub(*line_match);
                        self.vertical_scroll = line_match.saturating_sub(self.viewport_lines_count / 2);
                        App::set_vertical_scroll_state(&mut self.vertical_scroll_state, self.vertical_scroll, self.viewport_lines_count);
                        self.scrolled_so_far = self.vertical_scroll;

                        self.line_at_cursor = *line_match;

                        self.report(
                            format!("Match {} out of {}", self.search_matches.len() - i, self.search_matches.len()),
                            ReportedMessageKinds::Info,
                            Duration::from_secs(1)
                        );
                        
                        break;
                    }
                }
                
                if !found {
                    self.report(
                        format!("No previous matches"),
                        ReportedMessageKinds::Error,
                        Duration::from_secs(1)
                    );
                }
            }
            SearchingAction::ClearMatches => {
                self.search_matches.clear();
            }
            SearchingAction::ReportResults => {
                self.report(
                    format!("Found {} matches", self.search_matches.len()),
                    ReportedMessageKinds::Info,
                    Duration::from_secs(1)
                );
            }
        }
    }

    fn handle_app_actions(&mut self, sys_msg: SystemAction) {
        match sys_msg {
            SystemAction::Quit => {
                self.quit();
            },
        }
    }
    
    fn get_focused_text_input(&mut self) -> Option<&mut TextInput> {
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
    
    /// Tells us whether the currently focused line is at or near the end of the viewport.
    /// 
    /// This function is useful for auto-scrolling behavior when the cursor approaches
    /// the bottom of the viewport.
    fn is_current_line_at_viewport_end_with_offset(&self, offset: Option<usize>) -> bool {
        let offset = if offset.unwrap_or(0) > self.lines_count {
            0
        } else {
            offset.unwrap_or(0)
        };

        return self.line_at_cursor >= ((self.viewport_lines_count + self.scrolled_so_far).saturating_sub(offset));
    }

    /// Tells us whether the currently focused line is at the start of the viewport.
    /// 
    /// This function is useful for auto-scrolling behavior when the cursor approaches
    /// the top of the viewport.
    fn is_current_line_at_viewport_start_with_offset(&self, offset: Option<usize>) -> bool {
        let offset = if offset.unwrap_or(0) > self.lines_count {
            0
        } else {
            offset.unwrap_or(0)
        };

        return self.line_at_cursor <= (self.scrolled_so_far + offset);
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
            search_widget: TextInput::new(Some("Look For")),
            search_matches: vec![],
            json: Value::default(),
            lines_count: 0,
            viewport_lines_count: 0,
            message_to_report: RefCell::new(ReportedMessage {
                message: "".to_string(),
                show_time: Instant::now(),
                kind: ReportedMessageKinds::Info,
                show_duration: Duration::from_secs(0),
            }),
            current_screen: CurrentScreen::ViewingFile,
            currently_editing: None,
            editing_mode: EditingMode::Inserting,
            line_at_cursor: 0,
            json_pairs: vec![],
            file_metadata: None,
            file: None,
            vertical_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
            scrolled_so_far: 0,
            size: Size::default(),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_data_to_tree_for_object() {
        let app = App::default();

        let data = r#"
        {
            "name": "Omar",
            "role": {
                "role_name": "Admin",
                "active": "true"
            },
            "age": "24",
            "business": {
                "business_name": "Omar",
                "business_contact": {
                    "phone": "123456789",
                    "email": "mail@email.com"
                }
            },
            "permissions": [
                "READ",
                "WRITE"
            ]
        }
        "#;

        let data = serde_json::from_str(data).unwrap();

        {
            let mut pairs: Vec<ValuePair> = vec![];
            app.insert_data_to_tree(&mut pairs, &data, 0);
    
            assert_eq!(pairs.len(), 13);
            assert_eq!(
                pairs[0],
                ValuePair {
                    indentation: 1,
                    key: "name".to_string(),
                    value: Some(serde_json::to_value("Omar").unwrap()),
                    is_array_value: false,
                }
            );
        }

        {
            let mut pairs: Vec<ValuePair> = vec![];
            app.insert_data_to_tree(&mut pairs, &data, 0);
    
            assert_eq!(pairs.len(), 13);
            assert_eq!(
                pairs[2],
                ValuePair {
                    indentation: 2,
                    key: "role_name".to_string(),
                    value: Some(serde_json::to_value("Admin").unwrap()),
                    is_array_value: false,
                }
            );
        }
        
        {
            let mut pairs: Vec<ValuePair> = vec![];
            app.insert_data_to_tree(&mut pairs, &data, 0);
            
            assert_eq!(pairs.len(), 13);
            assert_eq!(
                pairs[8],
                ValuePair {
                    indentation: 3,
                    key: "phone".to_string(),
                    value: Some(serde_json::to_value("123456789").unwrap()),
                    is_array_value: false,
                }
            );
        }

        {
            let mut pairs: Vec<ValuePair> = vec![];
            app.insert_data_to_tree(&mut pairs, &data, 0);
            
            assert_eq!(pairs.len(), 13);
            assert_eq!(
                pairs[11],
                ValuePair {
                    indentation: 2,
                    key: String::new(),
                    value: Some(serde_json::to_value("READ").unwrap()),
                    is_array_value: true,
                }
            );
        }
    }
    
    
    #[test]
    fn test_insert_data_to_tree_for_array() {
        let app = App::default();

        let data = r#"
        [
            42,
            null,
            {
                "name": "Omar",
                "role": {
                    "role_name": "Admin",
                    "active": "true"
                },
                "age": "24",
                "business": {
                    "business_name": "Omar",
                    "business_contact": {
                        "phone": "123456789",
                        "email": "mail@email.com"
                    }
                }
            }
        ]
        "#;
        
        let data = serde_json::from_str(data).unwrap();

        {
            let mut pairs: Vec<ValuePair> = vec![];
            app.insert_data_to_tree(&mut pairs, &data, 0);
    
            assert_eq!(pairs.len(), 13); // The line between `null,` and `"name": "Omar",` is an empty pair on purpose for the UI.
            assert_eq!(
                pairs[0],
                ValuePair {
                    indentation: 1,
                    key: "".to_string(),
                    value: Some(serde_json::to_value(42).unwrap()),
                    is_array_value: true,
                }
            );
        }
    }

    #[test]
    fn test_insert_data_to_tree_for_arrays_inside_objects() {
        let app = App::default();

        let data = r#"
        {
            "name": "Jane Doe",
            "age": 9,
            "address": {
                "street": "123 Main St",
                "city": "Anytown",
                "state": "CA",
                "zip": "12345"
            },
            "salary": "5",
            "billing_info": {
                "card_number": "1234567890123456",
                "expiry_date": "12/25",
                "invoices": [
                    {
                        "amount": 100.0,
                        "due_date": "2023-06-30"
                    },
                    {
                        "amount": 100.0,
                        "due_date": "2023-06-30"
                    }
                ],
                "cvv": "123"
            },
            "currency": "USD",
            "currency_symbol": "$"
        }
        "#;

        let data = serde_json::from_str(data).unwrap();

        {
            let mut pairs: Vec<ValuePair> = vec![];
            app.insert_data_to_tree(&mut pairs, &data, 0);

            assert_eq!(pairs.len(), 21);
            assert_eq!(
                pairs[11],
                ValuePair {
                    indentation: 2,
                    key: "invoices".to_string(),
                    value: None,
                    is_array_value: false,
                }
            );
        }

        {
            let mut pairs: Vec<ValuePair> = vec![];
            app.insert_data_to_tree(&mut pairs, &data, 0);

            assert_eq!(pairs.len(), 21); // The line between `null,` and `"name": "Omar",` is an empty pair on purpose for the UI.
            assert_eq!(
                pairs[13],
                ValuePair {
                    indentation: 4,
                    key: "amount".to_string(),
                    value: Some(serde_json::to_value(100.0).unwrap()),
                    is_array_value: false,
                }
            );
        }
    }
    
    #[test]
    fn test_insert_new_pair_from_input() {
        let data = r#"
        {
            "name": "Jane Doe",
            "age": 9,
            "address": {
                "street": "123 Main St",
                "city": "Anytown",
                "state": "CA",
                "zip": "12345"
            },
            "salary": "5",
            "billing_info": {
                "card_number": "1234567890123456",
                "expiry_date": "12/25",
                "invoices": [
                    {
                        "amount": 100.0,
                        "due_date": "2023-06-30"
                    },
                    {
                        "amount": 200.0,
                        "due_date": "2024-06-30"
                    }
                ],
                "cvv": "123"
            },
            "currency_symbol": "$"
        }
        "#;

        let mut app = App::default();
        let data = serde_json::from_str(data).unwrap();
        app.json = data;

        // Test case: Insert after the first line,
        {
            app.key_input = TextInput::new(Some("Title"));
            app.key_input.set_content("Currency");
            app.value_input.set_content("USD");
            app.line_at_cursor = 0;
            app.insert_new_data_from_user_input();

            let json_as_ordered_map = app.json.as_object().unwrap();

            assert_eq!(
                json_as_ordered_map.iter().nth(1).unwrap(),
                (&"Currency".to_string(), &serde_json::to_value("USD").unwrap()),
            );
        }
        
        // Test case: Insert after the third line. Into the object below it.
        // @NotImplemented: Inserting inside of an object instead of after it because the index
        // is at an object value.
        // {
        //     app.key_input = TextInput::new(Some("Title"));
        //     app.key_input.set_content("Currency");
        //     app.value_input.set_content("USD");
        //     app.line_at_cursor = 2;
        //     app.insert_new_pair_from_input();

        //     let json_as_ordered_map = app.json.as_object().unwrap().get("address").unwrap().as_object().unwrap();

        //     assert_eq!(
        //         json_as_ordered_map.iter().nth(0).unwrap(),
        //         (&"Currency".to_string(), &serde_json::to_value("USD").unwrap()),
        //     );
        // }
    }
    
    
    #[test]
    fn test_lines_count() {
        let data = r#"
        {
            "string_examples": "Unicode text with \"quotes\", tabs\t, newlines\n and emoji 🦀",
            "number_examples": {
                "integer": 42,
                "negative": -17,
                "float": 3.14159,
                "scientific": 1.23e-4
            },
            "boolean_examples": { "true_value": true, "false_value": false },
            "null_example": null,
            "array_examples": [
                "string",
                123,
                false,
                null,
                { "nested_object": "inside_array" },
                [1, 2, [3, 4]]
            ],
            "nested_object": {
                "level1": {
                    "level2": {
                        "level3": "deeply nested value"
                    }
                }
            },
            "empty_values": { "empty_object": {}, "empty_array": [] }
        }
        "#;

        let mut app = App::new(data, None, None, Size::default()).unwrap();
        let mut pairs = vec![];
        let lines_count = app.insert_data_to_tree(&mut pairs, &app.json, 0);
        app.lines_count = lines_count;
        app.json_pairs = pairs.clone();
        
        {
            assert_eq!(
                app.lines_count,
                30,
            );
        }

        {
            // Move 100 times. Shouldn't move more than the actual count of lines.
            for _ in 0..100 {
                app.update(Action::MainView(MainViewActions::MoveDown));
            }

            assert_eq!(
                app.line_at_cursor,
                29,
            );
        }
    }

    #[test]
    fn test_editing_existing_values() {
        let data = r#"
        {
            "name": "Jane Doe",
            "age": 30,
            "hobbies": ["reading", "coding"],
            "active": true
        }
        "#;

        let mut app = App::new(data, None, None, Size::default()).unwrap();
        let mut pairs = vec![];
        let lines_count = app.insert_data_to_tree(&mut pairs, &app.json, 0);
        app.lines_count = lines_count;
        app.json_pairs = pairs;

        // Test editing a string value
        {
            app.line_at_cursor = 0; // "name": "Jane Doe"
            app.start_editing_existing_value();

            assert_eq!(app.editing_mode, EditingMode::Editing);
            assert_eq!(app.key_input.content(), "name");
            assert_eq!(app.value_input.content(), "Jane Doe");

            // Change the value
            app.key_input.set_content("full_name");
            app.value_input.set_content("John Smith");
            app.update_existing_data_from_user_input();

            // Verify the change
            let json_obj = app.json.as_object().unwrap();
            assert_eq!(json_obj.get("full_name").unwrap().as_str().unwrap(), "John Smith");
            assert!(json_obj.get("name").is_none());
        }

        // Test editing a number value
        {
            app.line_at_cursor = 1; // "age": 30
            app.start_editing_existing_value();

            assert_eq!(app.key_input.content(), "age");
            assert_eq!(app.value_input.content(), "30");

            // Change the value
            app.value_input.set_content("25");
            app.update_existing_data_from_user_input();

            // Verify the change
            let json_obj = app.json.as_object().unwrap();
            assert_eq!(json_obj.get("age").unwrap().as_i64().unwrap(), 25);
        }

        // Test editing an array value
        {
            app.line_at_cursor = 3; // "reading" in hobbies array
            app.start_editing_existing_value();

            assert_eq!(app.value_input.content(), "reading");

            // Change the value
            app.value_input.set_content("writing");
            app.update_existing_data_from_user_input();

            // Verify the change
            let json_obj = app.json.as_object().unwrap();
            let hobbies = json_obj.get("hobbies").unwrap().as_array().unwrap();
            assert_eq!(hobbies[0].as_str().unwrap(), "writing");
        }
    }
}
