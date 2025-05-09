//
// The main view.
//

use std::rc::Rc;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect}, style::{Color, Style}, symbols::scrollbar, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation}, Frame
};

use crate::{app::{App, CurrentScreen, CurrentlyEditing, ReportedMessageKinds}, helpers::get_centered_rect};


impl<'a> App<'a> {
    pub fn draw_main_view(&mut self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.area());
        
        self.viewport_lines_count = layout[0].height as usize;
        
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
        
        if json_length == 0 {
            let list_paragraph_widget = Paragraph::new(
                vec![
                    Line::from("Object is empty."),
                ],
            );

            let centered_layout = get_centered_rect(10, 10, layout[0]);
            
            frame.render_widget(list_paragraph_widget, centered_layout);
        } else {
            let mut pairs = vec![];
            let lines_count = self.insert_data_to_tree(&mut pairs, &self.json, 0);
            self.lines_count = lines_count;
            self.json_pairs = pairs.clone();

            let focused_pair_style = Style::default().bg(Color::Green).fg(Color::Black);

            let mut lines: Vec<Line> = vec![];
            let mut array_key_index = 0;
            let mut last_indentation = 0;

            self.search_matches.clear();

            for (current_line, mut pair) in pairs.into_iter().enumerate() {
                let indentation_padding: String = (0..pair.indentation - 1).map(|_| "    ").collect();
                
                // Set highlighting for this key if it matches the current active search term.
                let mut highlight_key = false;
                if !self.search_widget.content().is_empty() && pair.key.to_lowercase().contains(self.search_widget.content()) {
                    self.search_matches.push(current_line);
                    highlight_key = true;
                }

                // If the current indentation is smaller, then we went up in the json. Reset the array index.
                if last_indentation > pair.indentation {
                    last_indentation = pair.indentation;
                    array_key_index = 0;
                }
                
                // If the current indentation is equal, then we are in the same array. Increment the array index.
                if pair.key == "" {
                    if last_indentation != pair.indentation {
                        last_indentation = pair.indentation;
                        array_key_index = 0;
                    }

                    array_key_index += 1;
                    pair.key = format!("{}", array_key_index);
                }

                let is_line_focused = self.line_at_cursor == current_line;

                let mut line = Line::from(
                    match &pair.value { // A Line is returned here.
                        Some(value) => {
                            // Colorize the value part of the line/pair based on the type of the value. Kinda like syntax highlighting.
                            let mut value_span = Span::from(format!("{}", value));
                            if !is_line_focused { // Do not set the colored text if the we are hovering over this line because there's a bg color applied in that case.
                                if value.is_boolean() {
                                    value_span = value_span.style(Style::default().fg(Color::Red));
                                } else if value.is_number() {
                                    value_span = value_span.style(Style::default().fg(Color::Rgb(212, 188, 125))); // yellowish color.
                                } else if value.is_null() {
                                    value_span = value_span.style(Style::default().fg(Color::Rgb(243, 139, 168))); // pinkish color.
                                } else {
                                    value_span = value_span.style(Style::default().fg(Color::Green));
                                }
                            }
                            
                            // Highlight search matches if found for the value.
                            if !self.search_widget.content().is_empty() && value.to_string().to_lowercase().contains(self.search_widget.content()) {
                                // Check if this current wasn't already added by matching the key of the pair. If not, 
                                // save it to the matches.
                                match self.search_matches.last() {
                                    Some(last) => {
                                        if *last != current_line {
                                            self.search_matches.push(current_line);
                                        }
                                    }
                                    None => {
                                        self.search_matches.push(current_line);
                                    }
                                }
                                
                                value_span.style.bg = Some(Color::Rgb(246, 118, 111));
                                value_span.style.fg = Some(Color::default());
                            }
                            
                            // Highlight search matches if found for the key.
                            let mut key_span = Span::from(pair.key);
                            if highlight_key {
                                key_span = key_span.style(Style::default().bg(Color::Rgb(246, 118, 111))); // Reddish
                                key_span.style.fg = Some(Color::default());
                            }
                            
                            Span::from(indentation_padding) + key_span + Span::from(": ") + value_span // Concatenating two `Span`s makeup a `Line`.
                        },
                        None => {
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

                // Fill up the line till the end of the terminal's width to have the hover background
                // span the entire line in the terminal and not just cover the text characters.
                // Purely cosmetic.
                if is_line_focused {
                    let terminal_width = self.size.width as usize;
                    let content_length: usize = line.iter().map(|span| span.width()).sum(); 

                    if content_length < terminal_width {
                        let padding = " ".repeat(terminal_width - content_length);
                        let padding_span = Span::styled(padding, focused_pair_style);
                        line.push_span(padding_span);
                    }
                }

                lines.push(line);
            }

            self.vertical_scroll_state = self.vertical_scroll_state.content_length(lines.len());
            
            let list_paragraph_widget = Paragraph::new(lines)
                .block(Block::default().padding(Padding::horizontal(2)))
                .scroll((self.vertical_scroll as u16, 0));
            
            frame.render_widget(list_paragraph_widget, layout[0]);
        };

        // Render the scrollbar.
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .symbols(scrollbar::VERTICAL)
                .begin_symbol(None)
                .track_symbol(None)
                .end_symbol(None),
            layout[0].inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.vertical_scroll_state,
        );
    }
    
    fn draw_footer_widget(&mut self, frame: &mut Frame, layout: &Rc<[Rect]>) {
        let footer_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(layout[1]);
        
        match self.current_screen {
            CurrentScreen::ViewingFile => {
                let span = Span::from(
                    "(q) to quit, (i) to make new pair, (/) to search",
                );
                
                let paragraph = Paragraph::new(
                    Line::from(span)
                ).block(Block::default().borders(Borders::ALL).padding(Padding::left(1)));
                
                frame.render_widget(paragraph, footer_layout[0]);
            },
            CurrentScreen::Editing => {
                let span = Span::from(
                    "(ESC) to cancel/(Tab) to switch boxes/enter to complete",
                );
                
                let paragraph = Paragraph::new(
                    Line::from(span)
                ).block(Block::default().borders(Borders::ALL).padding(Padding::left(1)));
                
                frame.render_widget(paragraph, footer_layout[0]);
            },
            CurrentScreen::Searching => {
                self.search_widget.is_focused = true;
                self.search_widget.render_to_frame(frame, footer_layout[0]);
            },
        };
        
        // Check if we have a fresh (unexpired) message to report to the user.
        // that message is displayed in the footer in place of the usual keymap footer.
        
        let root_len = match &self.json {
            serde_json::Value::Array(values) => values.len(),
            serde_json::Value::Object(map) => map.len(),
            _ => 0,
        };
        let file_info_footer = if self.message_to_report.borrow().show_time.elapsed() >= self.message_to_report.borrow().show_duration { 
            Paragraph::new(
                Line::from(vec![
                    if let Some(metadata) = &self.file_metadata {
                        let size = if metadata.len() > 1024 {
                            format!("{} KB", metadata.len() / 1024)
                        } else {
                            format!("{} Bytes", metadata.len())
                        };
                        
                        Span::from(format!("File size: {} Bytes", size))
                    } else {
                        Span::from("File size: N/A")
                    },
                    Span::from(format!(", Parent length: {}", root_len)),
                    Span::from(format!(", Total lines: {}", self.lines_count)),
                    Span::from(format!(", Current line: {}", self.line_at_cursor.saturating_add(1))),
                ])
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .padding(Padding::left(1))
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
                Block::default()
                    .borders(Borders::ALL)
                    .padding(Padding::left(1))
            )
        };

        frame.render_widget(file_info_footer, footer_layout[1]);
    }
    
    fn draw_insert_popup_widget(&mut self, frame: &mut Frame) {
        let title_text = if !self.is_inside_array() {
            "Enter a new key-value pair"
        } else {
            "Add a new value"
        };
        
        let editing_popup = Block::default()
            .title(title_text)
            .title_alignment(Alignment::Center)
            .borders(Borders::NONE)
            .style(Style::default().fg(Color::default()).bg(Color::default()));
        
        let centered_area = get_centered_rect(50, 9, frame.area());

        if !self.is_inside_array() {
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
        } else {
            let layout = Layout::default()
                .margin(1)
                .constraints([
                    Constraint::Percentage(100), 
                ])
                .split(centered_area);
            
            if let Some(editing) = &self.currently_editing {
                frame.render_widget(editing_popup, centered_area);
                
                self.value_input.is_focused = *editing == CurrentlyEditing::Value;
                
                self.value_input.render_to_frame(frame, layout[0]);
            }
        }
    }
}
