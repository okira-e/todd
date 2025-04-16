//
// Main app rendering entry point.
//

use ratatui::
    Frame
;

use crate::app::state::App;

impl<'a> App<'a> {
    /// Draws a view based on the state.
    pub fn draw(&mut self, frame: &mut Frame) {
        // The only view there is for this app.
        self.draw_main_view(frame);
    }
}