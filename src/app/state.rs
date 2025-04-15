use std::{cell::RefCell, time::{Duration, Instant}};

use color_eyre::Result;
use ratatui::
    DefaultTerminal
;
use serde_json::Value;

use crate::{ui::widgets::text_input::TextInput, utils::json::get_nested_object_to_insert_into};


#[derive(Debug)]
pub enum CurrentScreen {
    ViewingFile,
    Editing,
    // Exiting,
}

#[derive(Debug, PartialEq)]
pub enum CurrentlyEditing {
    Key,
    Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValuePair {
    pub indentation: usize,
    pub key: String,
    pub value: Option<Value>,
    pub is_array_value: bool,
}

pub enum Action {
    AppNavigation(AppNavigationAction),
    MainView(MainViewActions),
    Editing(EditingAction),
    App(SystemAction),
}

pub enum AppNavigationAction {
    ToViewingScreen,
    ToEditingScreen,
}

pub enum MainViewActions{
    MoveDown,
    MoveUp,
}

pub enum EditingAction {
    SwitchToKey,
    SwitchToValue,
    AppendChar(char),
    AppendToKey(char),
    AppendToValue(char),
    MoveCursor(CursorDirection),
    PopFromKey,
    PopFromValue,
    Submit,
}

pub enum CursorDirection {
    Left,
    // Up,
    Right,
    // Down,
}

pub enum SystemAction {
    Quit,
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

// /// Represents the data in the file.
// #[derive(Debug)]
// struct JsonData {
//     pub is_array: bool,
//     object: IndexMap<String, Value>,
//     array: Vec<Value>,
// }

// impl JsonData {
//     pub fn new(json_content: &str) -> color_eyre::Result<Self> {
//         let mut ret = Self {
//             is_array: false,
//             array: vec![],
//             object: IndexMap::new(),
//         };
        
//         let parsed: Value = serde_json::from_str(json_content)?;
        
//         match parsed {
//             Value::Object(map) => {
//                 // Convert to IndexMap if needed
//                 let index_map: IndexMap<String, Value> = map.into_iter().collect();
                
//                 ret.object = index_map;
//             },
//             Value::Array(arr) => {
//                 ret.is_array = true;
//                 ret.array = arr;
//             },
//             _ => {
//                 // Handle primitive values (string, number, bool, null)
//                 // You could wrap them in an array or create a single-item object
//                 let mut map = IndexMap::new();
//                 map.insert("root".to_string(), parsed);

//                 ret.object = map;
//             }
//         };
        
//         return Ok(ret);
//     }
// }

#[derive(Debug)]
pub struct App {
    /// The key element in the popup for inserting new pairs.
    pub key_input: TextInput,
    /// The currently being edited json value.
    pub value_input: TextInput,
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
    /// Keeps track of where the current focused line is. Represents a line in the UI. So if the UI
    /// needs an empty line, you will find it at it. It doesn't represent the actual count of paris in the JSON.
    pub line_at_cursor: usize,
    /// A temporary message to report in the UI to the user.
    pub message_to_report: RefCell<ReportedMessage>,
    /// The total number of lines drawn (counts nested objects).
    pub lines_count: usize,
    running: bool,
}

impl App {
    /// Construct the app and sets the JSON data.
    pub fn new(json_content: &str) -> Result<Self> {
        let mut app = Self::default();

        // app.json = JsonData::new(json_content)?;
        let json = serde_json::from_str(json_content)?;
        app.json = json;

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
            Action::AppNavigation(action) => self.handle_navigation_actions(action),
            Action::MainView(action) => self.handle_main_view_messages(action),
            Action::Editing(action) => self.handle_editing_actions(action),
            Action::App(action) => self.handle_app_actions(action),
        }
    }
    
    /// Inserts the from the user popup to the file/data.
    pub fn insert_new_pair_from_input(&mut self) {
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
            // Check if we're at the last index. If yes, just insert, otherwise, insert safely
            // at `index + 1`.
            if index > object_to_insert_into.as_object().unwrap().len() - 2 {
                object_to_insert_into.as_object_mut().unwrap().insert(
                    self.key_input.content().to_string(),
                    value,
                );
            } else {
                object_to_insert_into.as_object_mut().unwrap().shift_insert(
                    index + 1,
                    self.key_input.content().to_string(),
                    value,
                );
            }
        }
        
        self.report(
            format!("Inserted new key-value pair: {} -> {}", self.key_input.content(), self.value_input.content()),
            ReportedMessageKinds::Success,
            Duration::from_secs(3)
        );
    }

    pub fn toggle_editing(&mut self) {
        // Switch between key and value keys unless we're not on either then toggle to key.
        match &self.currently_editing {
            Some(edit_mode) => {
                match edit_mode {
                    CurrentlyEditing::Key => self.currently_editing = Some(CurrentlyEditing::Value),
                    CurrentlyEditing::Value => self.currently_editing = Some(CurrentlyEditing::Key),
                };
            }
            None => {
                self.current_screen = CurrentScreen::Editing;
                self.currently_editing = Some(CurrentlyEditing::Key);
            }
        }
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
                if !it.is_object() {
                    *lines_count += 1;
                }

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

    fn handle_navigation_actions(&mut self, action: AppNavigationAction) {
        match action {
            AppNavigationAction::ToViewingScreen => {
                self.key_input.is_focused = false;
                self.value_input.is_focused = false;
                self.currently_editing = None;
                self.key_input.clear();
                self.value_input.clear();
                self.current_screen = CurrentScreen::ViewingFile;
            },
            AppNavigationAction::ToEditingScreen => {
                self.toggle_editing();
            },
        }
    }
    
    fn handle_main_view_messages(&mut self, action: MainViewActions) {
        match action {
            MainViewActions::MoveDown => {
                if self.line_at_cursor + 1 < self.lines_count {
                    self.line_at_cursor += 1;
                }
            },
            MainViewActions::MoveUp => {
                self.line_at_cursor = self.line_at_cursor.saturating_sub(1);
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
                    // CursorDirection::Up => {
                    //     if let Some(focused_text_input) = self.get_focused_text_input() {

                    //     }
                    // },
                    CursorDirection::Right => {
                        if let Some(focused_text_input) = self.get_focused_text_input() {
                            focused_text_input.move_cursor_right();
                        }
                    },
                    // CursorDirection::Down => {
                    //     if let Some(focused_text_input) = self.get_focused_text_input() {

                    //     }
                    // },
                }
            }
            EditingAction::AppendToKey(c) => {
                self.key_input.enter_char(c);
            },
            EditingAction::AppendToValue(c) => {
                self.value_input.enter_char(c);
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
                self.insert_new_pair_from_input();
            },
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
                        // self.update(Action::Editing(EditingAction::AppendToKey(c)));
                    }
                }
                CurrentlyEditing::Value => {
                    if self.value_input.is_focused {
                        return Some(&mut self.value_input);
                        // self.update(Action::Editing(EditingAction::AppendToValue(c)));
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

impl Default for App {
    fn default() -> Self {
        Self {
            running: false,
            key_input: TextInput::new(Some("Key")),
            value_input: TextInput::new(Some("Value")),
            // json: IndexMap::<String, Value>::new(),
            json: Value::default(),
            lines_count: 0,
            message_to_report: RefCell::new(ReportedMessage {
                message: "".to_string(),
                show_time: Instant::now(),
                kind: ReportedMessageKinds::Info,
                show_duration: Duration::from_secs(0),
            }),
            current_screen: CurrentScreen::ViewingFile,
            currently_editing: None,
            line_at_cursor: 0,
            json_pairs: vec![],
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
            app.insert_new_pair_from_input();

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
}
