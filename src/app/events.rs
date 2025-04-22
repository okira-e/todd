use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crate::actions::{Action, AppNavigationAction, CurrentScreen, CurrentlyEditing, CursorDirection, EditingAction, MainViewActions, SystemAction};

use super::state::App;


impl<'a> App<'a> {
    pub fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.handle_key_events(key)?,
            Event::Mouse(_) => { }
            Event::Resize(columns, rows) => {
                self.size.height = rows;
                self.size.width = columns;
            }
            _ => { }
        };

        return Ok(());
    }
    
    /// Handles the key events based on the current screen and updates the state.
    fn handle_key_events(&mut self, key: KeyEvent) -> Result<()> {
        match self.current_screen {
            CurrentScreen::ViewingFile => {
                match (key.modifiers, key.code) {
                    (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        self.update(Action::App(SystemAction::Quit));
                    }
                    (_, KeyCode::Char('i')) => {
                        self.update(Action::AppNavigation(AppNavigationAction::ToEditingScreen));
                    }
                    (_, KeyCode::Char('j') | KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                        self.update(Action::MainView(MainViewActions::MoveDown));
                    }
                    (_, KeyCode::Char('k') | KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                        self.update(Action::MainView(MainViewActions::MoveUp));
                    }
                    (_, KeyCode::Char('g')) => {
                        self.update(Action::MainView(MainViewActions::MoveToTop));
                    }
                    (_, KeyCode::Char('G')) => {
                        self.update(Action::MainView(MainViewActions::MoveToBottom));
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('d')) | (KeyModifiers::META, KeyCode::Char('v')) => {
                        self.update(Action::MainView(MainViewActions::MoveHalfPageDown));
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('u')) | (KeyModifiers::CONTROL, KeyCode::Char('v')) => {
                        self.update(Action::MainView(MainViewActions::MoveHalfPageUp));
                    }
                    _ => { }
                }
            }
            
            CurrentScreen::Editing => match (key.modifiers, key.code) {
                (_, KeyCode::Enter) => {
                    self.update(Action::Editing(EditingAction::Submit));
                }
                
                (_, KeyCode::Backspace) => {
                    if let Some(currently_editing) = &self.currently_editing {
                        match currently_editing {
                            CurrentlyEditing::Key => {
                                if self.key_input.is_focused {
                                    self.update(Action::Editing(EditingAction::PopFromKey));
                                }
                            }
                            CurrentlyEditing::Value => {
                                if self.value_input.is_focused {
                                    self.update(Action::Editing(EditingAction::PopFromValue));
                                }
                            }
                        }
                    }
                }
                
                (_, KeyCode::Esc) => {
                    self.update(Action::AppNavigation(AppNavigationAction::ToViewingScreen));
                }
                
                (_, KeyCode::Tab) => {
                    self.update(Action::AppNavigation(AppNavigationAction::ToEditingScreen)); // Has the logic of switching between the inputs.
                }
                
                (_, KeyCode::Char(value)) => {
                    self.update(Action::Editing(EditingAction::AppendChar(value)));
                }
                
                (_, KeyCode::Left) => {
                    self.update(Action::Editing(EditingAction::MoveCursor(CursorDirection::Left)));
                }
                (_, KeyCode::Right) => {
                    self.update(Action::Editing(EditingAction::MoveCursor(CursorDirection::Right)));
                }
                _ => {
                    {}
                }
            }
        };
        
        return Ok(());
    } 
}