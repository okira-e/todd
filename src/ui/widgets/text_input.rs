use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Paragraph, Widget},
};

/// A reusable text input widget that handles cursor movement and text editing
#[derive(Debug, Clone)]
pub struct TextInput {
    /// Current value of the input box
    content: String,
    /// Position of cursor in the editor area (character index, not byte index)
    character_index: usize,
    /// Whether the input has focus and the cursor should be shown
    pub is_focused: bool,
    /// The style to apply to the text input when focused
    focus_style: Style,
    /// The style to apply to the text input when not focused
    default_style: Style,
    /// Optional block to wrap the text input
    block: Option<Block<'static>>,
    /// Whether to show the cursor
    show_cursor: bool,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            character_index: 0,
            is_focused: false,
            focus_style: Style::default().fg(Color::Yellow),
            default_style: Style::default(),
            block: None,
            show_cursor: true,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.character_index = self.content.chars().count();
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.set_content(content);
        
        return self;
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.character_index = 0;
    }

    pub fn with_focus(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        
        return self;
    }

    pub fn with_focus_style(mut self, style: Style) -> Self {
        self.focus_style = style;
        
        return self;
    }

    pub fn with_default_style(mut self, style: Style) -> Self {
        self.default_style = style;
        
        return self;
    }

    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        
        return self;
    }

    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.content.insert(index, new_char);
        self.move_cursor_right();
    }

    pub fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.content.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.content.chars().skip(current_index);

            // Put all characters together except the selected one.
            self.content = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    pub fn move_cursor_to_start(&mut self) {
        self.character_index = 0;
    }

    pub fn move_cursor_to_end(&mut self) {
        self.character_index = self.content.chars().count();
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.content
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.content.len())
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.content.chars().count())
    }

    /// Get the cursor position for the frame renderer
    pub fn cursor_position(&self, area: Rect) -> Position {
        // Calculate the block border offset if we have a block
        let x_offset = if self.block.is_some() { 1 } else { 0 };
        let y_offset = if self.block.is_some() { 1 } else { 0 };

        Position::new(
            // Draw the cursor at the current position in the input field
            area.x + self.character_index as u16 + x_offset,
            // Position vertically
            area.y + y_offset,
        )
    }

    /// Render the widget to a frame
    pub fn render_to_frame(&self, frame: &mut ratatui::Frame, area: Rect) {
        frame.render_widget(self.clone(), area);

        // Set cursor position if focused and showing cursor
        if self.is_focused && self.show_cursor {
            frame.set_cursor_position(self.cursor_position(area));
        }
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for TextInput {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = if self.is_focused {
            self.focus_style
        } else {
            self.default_style
        };

        // Create a paragraph widget for the text content
        let mut paragraph = Paragraph::new(Span::raw(&self.content)).style(style);

        // Apply the block if we have one
        if let Some(block) = self.block {
            paragraph = paragraph.block(block);
        }

        // Render the paragraph
        paragraph.render(area, buf);
    }
}