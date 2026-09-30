use super::*;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tasker_core::model::now;
use tasker_core::store::Store;

#[test]
fn delete_dialog_shows_all_its_text_and_keys_at_any_width() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::at(dir.path()).unwrap();
    store.save(&mut Task::new(1, "A task with a fairly long title".into(), now())).unwrap();
    let mut app = App::new(store).unwrap();
    app.mode = Mode::Confirm(Confirm::Delete(1));
    for width in [40, 64, 80, 120] {
        let mut terminal = Terminal::new(TestBackend::new(width, 20)).unwrap();
        terminal.draw(|f| draw(f, &app)).unwrap();
        let screen: String = terminal.backend().buffer().content().iter().map(ratatui::buffer::Cell::symbol).collect();
        assert!(screen.contains("(y) delete"), "keys cut off at width {width}");
        assert!(screen.contains("removed."), "text cut off at width {width}");
    }
}
