//
// The main view.
//

use std::rc::Rc;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Style}, text::{Line, Span, Text}, widgets::{Block, Borders, List, ListItem, Paragraph}, Frame
};

use crate::{app::state::{App, CurrentScreen, CurrentlyEditing}, ui::{helpers::get_centered_rect, widgets::text_input::TextInput}};


impl App {
    pub fn render_main_view(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.area());
        
        self.render_title_widget(frame, &layout);
        self.render_pairs_widget(frame, &layout);
        self.render_footer_widget(frame, &layout);

        self.render_insert_popup_widget(frame);
    }

    fn render_pairs_widget(&self, frame: &mut Frame, layout: &Rc<[Rect]>) {
        let list_widget = if self.data.len() == 0 {
            List::new(
                vec![
                    ListItem::new(
                        Line::from(
                            "Object is empty."
                        ),
                    ),
                ],
            )
        } else {
            let mut pairs = vec![];
            self.insert_data_to_tree(&mut pairs, &self.data, 0);
    
            let mut list_items: Vec<ListItem> = vec![];
            for pair in pairs {
                let indentation_padding: String = (0..pair.indentation - 1).map(|_| "    ").collect();
    
                let list_item = ListItem::new(
                    Line::from(
                        match pair.value {
                            Some(value) => format!("{}{}: {}", indentation_padding, pair.key, value),
                            None => {
                                match pair.is_array_value {
                                    true => format!("{}{}", indentation_padding, pair.key),
                                    false => format!("{}{}:", indentation_padding, pair.key),
                                }
                            }
                        }
                    ),
                );
    
                list_items.push(list_item);
            }
    
            List::new(list_items)
        };
    
        frame.render_widget(list_widget, layout[1]);
    }
    
    fn render_title_widget(&self, frame: &mut Frame, layout: &Rc<[Rect]>) {
        let title = Paragraph::new(Text::styled(
            "Create New Json",
            Style::default().fg(Color::Green),
        ))
        .block(Block::default().borders(Borders::ALL));
    
        frame.render_widget(title, layout[0]);
    }
    
    fn render_footer_widget(&self, frame: &mut Frame, layout: &Rc<[Rect]>) {
        let keymap_footer = Paragraph::new(
            Line::from(
                match self.current_screen {
                    CurrentScreen::ViewingFile => Span::from(
                        "(q) to quit / (i) to make new pair",
                    ),
                    CurrentScreen::Editing => Span::from(
                        "(ESC) to cancel/(Tab) to switch boxes/enter to complete",
                    ),
                }
            )
        )
        .block(
            Block::default().borders(Borders::ALL)
        );
    
        let footer_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(layout[2]);
    
        frame.render_widget(keymap_footer, footer_layout[0]);
    }
    
    fn render_insert_popup_widget(&mut self, frame: &mut Frame) {
        let editing_popup = Block::default()
            .title("Enter a new key-value pair")
            .title_alignment(Alignment::Center)
            .borders(Borders::NONE)
            .style(Style::default());
        
        let centered_area = get_centered_rect(50, 8, frame.area());

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([
                Constraint::Percentage(50), 
                Constraint::Percentage(50),
            ])
            .split(centered_area);
        
        // Update focus based on currently_editing
        if let Some(editing) = &self.currently_editing {
            frame.render_widget(editing_popup, centered_area);
            
            self.key_input.is_focused = *editing == CurrentlyEditing::Key;
            self.value_input.is_focused = *editing == CurrentlyEditing::Value;
            
            // Prepare key input
            let key_input = self.key_input.clone()
                .with_focus_style(Style::default().fg(Color::Yellow))
                .block(Block::default()
                    .title("Key")
                    .borders(Borders::ALL)
                    .style(if *editing == CurrentlyEditing::Key {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }));
            
            // Prepare value input
            let value_input = self.value_input.clone()
                .with_focus_style(Style::default().fg(Color::Yellow))
                .block(Block::default()
                    .title("Value")
                    .borders(Borders::ALL)
                    .style(if *editing == CurrentlyEditing::Value {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    }));
            
            // Render the widgets
            key_input.render_to_frame(frame, layout[0]);
            value_input.render_to_frame(frame, layout[1]);
        }
    }
}
