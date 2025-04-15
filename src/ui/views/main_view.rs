//
// The main view.
//

use std::rc::Rc;

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
        let json_length = if self.json.is_object() {
            self.json.as_object().unwrap().len()
        } else {
            self.json.as_array().unwrap().len()
        };
        
        let list_widget = if json_length == 0 {
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
            let lines_count = self.insert_data_to_tree(&mut pairs, &self.json, 0);
            self.lines_count = lines_count;
            self.json_pairs = pairs.clone();

            let focused_pair_style = Style::default().bg(Color::Green).fg(Color::Black);

            let mut list_items: Vec<ListItem> = vec![];
            let mut array_key_index = 0;
            let mut last_indentation = 0;
            for (i, mut pair) in pairs.into_iter().enumerate() {
                let indentation_padding: String = (0..pair.indentation - 1).map(|_| "    ").collect();
                
                let is_line_focused = self.line_at_cursor == i;
                
                let list_item = ListItem::new(
                    match &pair.value { // A Line is returned here.
                        Some(value) => {
                            if pair.key == "" {
                                if last_indentation != pair.indentation {
                                    last_indentation = pair.indentation;
                                    array_key_index = 0;
                                }

                                array_key_index += 1;
                                pair.key = format!("{}", array_key_index);
                            }
                            
                            let indentation_and_key_span = Span::from(format!("{}{}: ", indentation_padding, pair.key));

                            // Colorize the value part of the line/pair based on the type of the value. Kinda like syntax highlighting.
                            let mut value_span = Span::from(format!("{}", value));
                            if !is_line_focused { // Do not set the colored text if the we are hovering over this line because there's a bg color applied in that case.
                                if value.is_boolean() {
                                    value_span = value_span.style(Style::default().fg(Color::Red));
                                } else if value.is_number() {
                                    value_span = value_span.style(Style::default().fg(Color::Rgb(212, 188, 125))); // yellowish color.
                                } else {
                                    value_span = value_span.style(Style::default().fg(Color::Green));
                                }
                            }
                            
                            indentation_and_key_span + value_span // Concatenating two `Span`s makeup a `Line`.
                        },
                        None => {
                            if last_indentation != pair.indentation {
                                last_indentation = pair.indentation;
                                array_key_index = 0;
                            }

                            if pair.key == "" {
                                array_key_index += 1;
                                pair.key = format!("{}", array_key_index);
                            }
                            
                            // Match against if this key's value is an array or another object.
                            match pair.is_array_value {
                                true => {
                                    let text = format!("{}{}", indentation_padding, pair.key);

                                    Line::from(Span::from(text).style(Style::default()))
                                },
                                false => {
                                    let text = format!("{}{}:", indentation_padding, pair.key);

                                    Line::from(Span::from(text).style(Style::default()))
                                },
                            }
                        }
                    }
                    .style(
                        if is_line_focused { focused_pair_style } else { Style::default() },
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
            .style(Style::default().fg(Color::default()).bg(Color::default()));
        
        let centered_area = get_centered_rect(50, 9, frame.area());

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
            
            self.key_input.render_to_frame(frame, layout[0]);
            self.value_input.render_to_frame(frame, layout[1]);
        }
    }
}
