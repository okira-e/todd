use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crate::{actions::{Action, AppNavigationAction, CursorDirection, EditingAction, MainViewActions, SearchingAction, SystemAction}, app::{CurrentScreen, CurrentlyEditing}};

use super::app::App;


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
                    (_, KeyCode::Char('e')) => {
                        self.update(Action::Editing(EditingAction::EditExisting));
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
                    (_, KeyCode::Char('n')) => {
                        self.update(Action::Searching(SearchingAction::GoToNextMatch));
                    }
                    (KeyModifiers::SHIFT, KeyCode::Char('N')) => {
                        self.update(Action::Searching(SearchingAction::GoToPrevMatch));
                    }
                    (_, KeyCode::Char('/')) => {
                        self.update(Action::Searching(SearchingAction::ClearSearch));
                        self.update(Action::AppNavigation(AppNavigationAction::ToSearchingWidget));
                    }
                    (_, KeyCode::Esc) => {
                        self.update(Action::Searching(SearchingAction::ClearSearch));
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
            
            CurrentScreen::Searching => match (key.modifiers, key.code) {
                (_, KeyCode::Backspace) => {
                    self.update(Action::Searching(SearchingAction::PopChar));
                }
                
                (_, KeyCode::Esc) => {
                    self.update(Action::AppNavigation(AppNavigationAction::ToViewingScreen));
                }
                
                (_, KeyCode::Enter) => {
                    self.update(Action::Searching(SearchingAction::ReportResults));
                    self.update(Action::AppNavigation(AppNavigationAction::ToViewingScreen));
                }
                
                (_, KeyCode::Left) => {
                    self.update(Action::Searching(SearchingAction::MoveCursor(CursorDirection::Left)));
                }

                (_, KeyCode::Right) => {
                    self.update(Action::Searching(SearchingAction::MoveCursor(CursorDirection::Right)));
                }
                
                (_, KeyCode::Char(value)) => {
                    self.update(Action::Searching(SearchingAction::AppendChar(value)));
                }
                
                _ => {
                    {}
                }
            }
        };
        
        return Ok(());
    } 
}