use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use super::state::{Action, App, CurrentScreen, CurrentlyEditing, CursorDirection, EditingAction, NavigationAction, SystemAction};


impl App {
    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    pub fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.handle_key_events(key)?,
            Event::Mouse(_) => { }
            Event::Resize(_, _) => { }
            _ => { }
        };

        return Ok(());
    }
    
    /// Handles the key events based on the current screen and updates the state.
    fn handle_key_events(&mut self, key: KeyEvent) -> Result<()> {
        match self.current_screen {
            CurrentScreen::ViewingFile => {
                match (key.modifiers, key.code) {
                    // q, c, C
                    (_, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        self.update(Action::System(SystemAction::Quit));
                    }
                    // e
                    (_, KeyCode::Char('i')) => {
                        self.update(Action::Navigation(NavigationAction::ToEditingScreen));
                    }
                    _ => { }
                }
            }
            
            CurrentScreen::Editing => match (key.modifiers, key.code) {
                (_, KeyCode::Enter) => {
                    if let Some(currently_editing) = &self.currently_editing {
                        match currently_editing {
                            CurrentlyEditing::Key => {
                                self.update(Action::Navigation(NavigationAction::ToEditingScreen));
                            }
                            CurrentlyEditing::Value => {
                                self.update(Action::Editing(EditingAction::Submit));
                                self.update(Action::Navigation(NavigationAction::ToViewingScreen));
                            }
                        }
                    }
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
                    self.update(Action::Navigation(NavigationAction::ToViewingScreen));
                }
                
                (_, KeyCode::Tab) => {
                    if let Some(currently_editing) = &self.currently_editing {
                        match currently_editing {
                            CurrentlyEditing::Key => {
                                self.update(Action::Editing(EditingAction::SwitchToValue));
                            },
                            CurrentlyEditing::Value => {
                                self.update(Action::Editing(EditingAction::SwitchToKey));
                            },
                        }
                    }
                }
                
                (_, KeyCode::Char(value)) => {
                    self.update(Action::Editing(EditingAction::AppendChar(value)));
                }
                
                (_, KeyCode::Left) => {
                    self.update(Action::Editing(EditingAction::MoveCursor(CursorDirection::Left)));
                }
                // (_, KeyCode::Up) => {
                //     self.update(Action::Editing(EditingAction::MoveCursor(CursorDirection::Up)));
                // }
                (_, KeyCode::Right) => {
                    self.update(Action::Editing(EditingAction::MoveCursor(CursorDirection::Right)));
                }
                // (_, KeyCode::Down) => {
                //     self.update(Action::Editing(EditingAction::MoveCursor(CursorDirection::Down)));
                // }
                _ => {
                    {}
                }
            }
            _ => {
                {}
            }
        };
        
        return Ok(());
    } 
}