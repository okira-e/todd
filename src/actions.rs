use std::{io::Seek, time::Duration};

use crate::app::{App, CurrentScreen, CurrentlyEditing, EditingMode, ReportedMessageKinds};

pub enum Action {
    AppNavigation(AppNavigationAction),
    MainView(JsonTreeAction),
    Editing(EditingAction),
    Searching(SearchingAction),
    App(AppAction),
}

pub enum AppNavigationAction {
    ToViewingScreen,
    ToEditingScreen,
    ToSearchingWidget,
}

pub enum JsonTreeAction {
    MoveDown,
    MoveUp,
    MoveToTop,
    MoveToBottom,
    MoveHalfPageDown,
    MoveHalfPageUp,
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
    EditExisting,
}

pub enum SearchingAction {
    AppendChar(char),
    MoveCursor(CursorDirection),
    PopChar,
    ClearSearch,
    GoToPrevMatch,
    GoToNextMatch,
    ClearMatches,
}

pub enum CursorDirection {
    Left,
    Right,
}

pub enum AppAction {
    Quit,
}


impl<'a> App<'a> {
    pub fn handle_app_navigation_actions(&mut self, action: AppNavigationAction) {
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
    
    pub fn handle_searching_actions(&mut self, action: SearchingAction) {
        match action {
            SearchingAction::AppendChar(c) => {
                self.handle_searching_actions(SearchingAction::ClearMatches); // Clear previous matches before the new ones with the new character.
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
                
                let next_match = self
                    .search_matches
                    .iter()
                    .copied()
                    .enumerate()
                    .find(|(_, lm)| *lm > self.json_tree.line_at_cursor)
                    .map(|(idx, lm)| (idx, lm));
                
                if let Some(next_match) = next_match {
                    self.report(
                        format!("Match {} out of {}", next_match.0 + 1, self.search_matches.len()),
                        ReportedMessageKinds::Info,
                        Duration::from_secs(1)
                    );

                    let line_diff = next_match.1.saturating_sub(self.json_tree.vertical_scroll);
                    let threshold = self.json_tree.viewport_lines_count.saturating_sub(self.json_tree.viewport_lines_count / 4);
                    if line_diff > threshold {
                        self.json_tree.vertical_scroll += line_diff.saturating_sub(self.json_tree.viewport_lines_count / 2);
                        self.json_tree.set_vertical_scroll_state();
                        self.json_tree.scrolled_so_far = self.json_tree.vertical_scroll;
                    }
                    self.json_tree.line_at_cursor = next_match.1;
                }
            }
            SearchingAction::GoToPrevMatch => {
                if self.search_widget.content().is_empty() {
                    return;
                }
                
                let prev_match = self
                    .search_matches
                    .iter()
                    .rev()
                    .copied()
                    .enumerate()
                    .find(|(_, lm)| *lm < self.json_tree.line_at_cursor)
                    .map(|(idx, lm)| (idx, lm));
                
                if let Some(prev_match) = prev_match {
                    self.report(
                        format!("Match {} out of {}", self.search_matches.len() - prev_match.0, self.search_matches.len()),
                        ReportedMessageKinds::Info,
                        Duration::from_secs(1)
                    );

                    self.json_tree.vertical_scroll = prev_match.1.saturating_sub(self.json_tree.viewport_lines_count / 2);
                    self.json_tree.set_vertical_scroll_state();
                    self.json_tree.scrolled_so_far = self.json_tree.vertical_scroll;
                    self.json_tree.line_at_cursor = prev_match.1;
                }
            }
            SearchingAction::ClearMatches => {
                self.search_matches.clear();
            }
        }
    }
    
    pub fn handle_editing_actions(&mut self, edit_msg: EditingAction) {
        match edit_msg {
            EditingAction::SwitchToKey => {
                self.currently_editing = Some(CurrentlyEditing::Key);
            },
            EditingAction::SwitchToValue => {
                self.currently_editing = Some(CurrentlyEditing::Value);
            },
            EditingAction::EditExisting => {
                // self.json_tree.start_editing_existing_value(
                //     self.key_input.content().to_string(),
                //     self.value_input.content().to_string(),
                // );
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
                    EditingMode::Inserting => {
                        self.json_tree.insert_new_data_from_user_input(
                            self.key_input.content().to_string(),
                            self.value_input.content().to_string(),
                        );
                        
                        self.report(
                            format!("Inserted new key-value pair: {} -> {}",
                                self.key_input.content(),
                                self.value_input.content(),
                            ),
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
                            if let Err(err) = serde_json::to_writer_pretty(file, &self.json_tree.json) {
                                self.report(
                                    format!("Failed to save changes: {}", err), 
                                    ReportedMessageKinds::Error, 
                                    Duration::from_secs(3)
                                );
                            }
                        }
                        
                        self.update(Action::AppNavigation(AppNavigationAction::ToViewingScreen));
                    },
                    EditingMode::Editing => {
                        // self.json_tree.update_existing_data_from_user_input();
                        // self.update(Action::AppNavigation(AppNavigationAction::ToViewingScreen));
                    },
                }
            },
        }
    }
    
    pub fn handle_app_actions(&mut self, sys_msg: AppAction) {
        match sys_msg {
            AppAction::Quit => {
                self.quit();
            },
        }
    }
}