pub enum Action {
    AppNavigation(AppNavigationAction),
    MainView(MainViewActions),
    Editing(EditingAction),
    Searching(SearchingAction),
    App(SystemAction),
}

pub enum AppNavigationAction {
    ToViewingScreen,
    ToEditingScreen,
    ToSearchingWidget,
}

pub enum MainViewActions {
    MoveDown,
    MoveUp,
    MoveToTop,
    MoveToBottom,
    MoveHalfPageDown,
    MoveHalfPageUp,
}

pub enum EditingAction {
    SwitchToKey,
    SwitchToValue,
    AppendChar(char),
    AppendToKey(char),
    AppendToValue(char),
    MoveCursor(CursorDirection),
    PopFromKey,
    PopFromValue,
    Submit,
}

pub enum SearchingAction {
    AppendChar(char),
    MoveCursor(CursorDirection),
    PopChar,
    ClearSearch,
    GoToPrevMatch,
    GoToNextMatch,
    ClearMatches,
    ReportResults,
}

pub enum CursorDirection {
    Left,
    // Up,
    Right,
    // Down,
}

pub enum SystemAction {
    Quit,
}
