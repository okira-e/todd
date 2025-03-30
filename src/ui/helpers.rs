use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Helper function to create a centered rect using up certain percentage of the available rect.
pub fn get_centered_rect(percent_x: u16, percent_y: u16, rect: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(rect);

    // Then cut the middle vertical piece into three width-wise pieces
    return Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(layout[1])[1]; // Return the middle chunk
}