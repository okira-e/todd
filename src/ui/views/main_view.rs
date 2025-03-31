//
// The main view.
//

use std::{rc::Rc, time::Duration};

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Style}, text::{Line, Span, Text}, widgets::{Block, Borders, List, ListItem, Padding, Paragraph}, Frame
};

use crate::{app::state::{App, CurrentScreen, CurrentlyEditing, ReportedMessageKinds}, ui::helpers::get_centered_rect};


impl App {
    pub fn draw_main_view(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.area());
        
        self.draw_title_widget(frame, &layout);
        self.draw_pairs_widget(frame, &layout);
        self.draw_footer_widget(frame, &layout);

        if self.currently_editing.is_some() {
            self.draw_insert_popup_widget(frame);
        }
    }

    fn draw_pairs_widget(&mut self, frame: &mut Frame, layout: &Rc<[Rect]>) {
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
            let lines_count = self.insert_data_to_tree(&mut pairs, &self.data, 0);
            self.lines_count = lines_count;
    
            let focused_pair_style = Style::default().bg(Color::Green).fg(Color::Black);
            let mut list_items: Vec<ListItem> = vec![];
            for (i, pair) in pairs.into_iter().enumerate() {
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
                    )
                    .style(
                        if self.line_at_cursor == i { focused_pair_style } else { Style::default() },
                    ),
                );
    
                list_items.push(list_item);
            }

            
            List::new(list_items)
                .block(
                    Block::default().padding(Padding::horizontal(2))
                )
        };
    
        frame.render_widget(list_widget, layout[1]);
    }
    
    fn draw_title_widget(&self, frame: &mut Frame, layout: &Rc<[Rect]>) {
        let title = Paragraph::new(Text::styled(
            "Create New Json",
            Style::default().fg(Color::Green),
        ))
        .block(Block::default().borders(Borders::ALL));
    
        frame.render_widget(title, layout[0]);
    }
    
    fn draw_footer_widget(&self, frame: &mut Frame, layout: &Rc<[Rect]>) {
        // Check if we have a fresh (unexpired) message to report to the user.
        // that message is displayed in the footer in place of the usual keymap footer.
        let footer = if self.message_to_report.borrow().show_time.elapsed() >= self.message_to_report.borrow().show_duration {
            Paragraph::new(
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
            )
        } else {
            Paragraph::new(
                Line::from(
                    Span::from(self.message_to_report.borrow().message.clone()),
                )
                .style(
                    Style::default().fg({
                        match self.message_to_report.borrow().kind {
                            ReportedMessageKinds::Error => Color::Red,
                            ReportedMessageKinds::Info => Color::default(),
                            ReportedMessageKinds::Debug => Color::Yellow,
                            ReportedMessageKinds::Warning => Color::Yellow,
                            ReportedMessageKinds::Success => Color::Green,
                        }
                    })
                )
            )
            .block(
                Block::default().borders(Borders::ALL)
            )
        };
        
        let footer_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(layout[2]);
        
        frame.render_widget(footer, footer_layout[0]);
    }
    
    fn draw_insert_popup_widget(&mut self, frame: &mut Frame) {
        let editing_popup = Block::default()
            .title("Enter a new key-value pair")
            .title_alignment(Alignment::Center)
            .borders(Borders::NONE)
            .style(Style::default());
        
        let centered_area = get_centered_rect(50, 10, frame.area());

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
