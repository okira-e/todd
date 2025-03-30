use color_eyre::Result;
use indexmap::IndexMap;
use ratatui::
    DefaultTerminal
;
use serde_json::Value;

use crate::ui::widgets::text_input::TextInput;


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

#[derive(Debug, PartialEq)]
pub struct ValuePair {
    pub indentation: usize,
    pub key: String,
    pub value: Option<String>,
    pub is_array_value: bool,
}

pub enum Action {
    Navigation(NavigationAction),
    Editing(EditingAction),
    System(SystemAction),
}

pub enum NavigationAction {
    ToViewingScreen,
    ToEditingScreen,
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

/// The main application which holds the state and logic of the application.
#[derive(Debug)]
pub struct App {
    running: bool,
    // the currently being edited json key.
    pub key_input: TextInput,
    // the currently being edited json value.
    pub value_input: TextInput,
    // The representation of our key and value pairs with serde Serialize support
    pub data: IndexMap<String, Value>,
    // the current screen the user is looking at, and will later determine what is rendered.
    pub current_screen: CurrentScreen,
    // the optional state containing which of the key or value pair the user is editing. It is an option, because when the user is not directly editing a key-value pair, this will be set to `None`.
    pub currently_editing: Option<CurrentlyEditing>,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new(json_content: &str) -> Result<Self> {
        let mut app = Self::default();

        app.data = serde_json::from_str(json_content)?;

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

    pub fn update(&mut self, message: Action) {
        match message {
            Action::Navigation(nav_msg) => self.handle_navigation_messages(nav_msg),
            Action::Editing(edit_msg) => self.handle_editing_messages(edit_msg),
            Action::System(sys_msg) => self.handle_system_messages(sys_msg),
        }
    }
    
    fn handle_navigation_messages(&mut self, nav_msg: NavigationAction) {
        match nav_msg {
            NavigationAction::ToViewingScreen => {
                self.key_input.is_focused = false;
                self.value_input.is_focused = false;
                self.currently_editing = None;
                self.key_input.clear();
                self.value_input.clear();
                self.current_screen = CurrentScreen::ViewingFile;
            },
            NavigationAction::ToEditingScreen => {
                self.toggle_editing();
            },
        }
    }
    
    fn handle_editing_messages(&mut self, edit_msg: EditingAction) {
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

    fn handle_system_messages(&mut self, sys_msg: SystemAction) {
        match sys_msg {
            SystemAction::Quit => {
                self.quit();
            },
        }
    }
    
    pub fn insert_new_pair_from_input(&mut self) {
        if self.key_input.is_focused && self.value_input.is_focused {
            // self.key & self.value inputs become None after this.
            self.data.insert(
                self.key_input.content().to_string(),
                serde_json::to_value(self.value_input.content()).unwrap(),
            );
        }

        self.currently_editing = None;
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

    pub fn insert_data_to_tree(
        &self, 
        pairs: &mut Vec<ValuePair>,
        data: &IndexMap<String, Value>,
        mut indentation_counter: usize,
    ) {
        indentation_counter += 1;

        for (key, value) in data {
            match value.is_object() {
                false => {
                    if value.is_array() {
                        pairs.push(
                            ValuePair { 
                                indentation: indentation_counter, 
                                key: key.clone(), 
                                value: None,
                                is_array_value: false,
                            }
                        );
                        
                        // Insert all values in the array at once with one more indentation level. No recursion
                        // needed.
                        for it in value.as_array().unwrap() {
                            pairs.push(
                                ValuePair {
                                    indentation: indentation_counter + 1, 
                                    key: serde_json::from_value(it.clone()).unwrap(), 
                                    value: None,
                                    is_array_value: true,
                                }
                            );
                        }
                    } else {
                        pairs.push(
                            ValuePair {
                                indentation: indentation_counter, 
                                key: key.clone(), 
                                value: Some(serde_json::from_value(value.clone()).unwrap()),
                                is_array_value: false,
                            }
                        );
                    }
                }
                true => {
                    pairs.push(
                        ValuePair {
                            indentation: indentation_counter, 
                            key: key.clone(), 
                            value: None,
                            is_array_value: false,
                        }
                    );

                    // Convert the `value: Value` to a HashMap.
                    // @Bug @Improvement: Currently, the original order is being lost.
                    let new_data: IndexMap<String, Value> = serde_json::from_value(value.clone()).unwrap();
                    
                    self.insert_data_to_tree(
                        pairs,
                        &new_data,
                        indentation_counter
                    );
                }
            }
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
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: false,
            key_input: TextInput::default(),
            value_input: TextInput::default(),
            data: IndexMap::<String, Value>::new(),
            current_screen: CurrentScreen::ViewingFile,
            currently_editing: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_to_indented_values() {
        let app = App::default();
        let mut pairs = vec![];

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
            }
        }
        "#;

        let data: IndexMap<String, Value> = serde_json::from_str(data).unwrap();

        app.insert_data_to_tree(&mut pairs, &data, 0);

        assert_eq!(pairs.len(), 10);
        assert_eq!(
            pairs[0],
            ValuePair {
                indentation: 1,
                key: "name".to_string(),
                value: Some("Omar".to_string()),
                is_array_value: false,
            }
        );
    }
}
