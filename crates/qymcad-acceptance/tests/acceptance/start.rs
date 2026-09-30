//! THE PROGRAM STARTS as a person starts it for the first time.
use qymcad::Session;
use qymcad_acceptance::{build, probe};

probe! {
    /// A FIRST START COMES UP ON THE SCREEN OF WHERE TO START, with its ways to begin written on it.
    fn a_first_start_offers_where_to_start() {
        let mut session = Session::start();
        for key in ["start-title", "start-new-part", "start-new-assembly", "start-open"] {
            let word = session.word(key);
            assert!(session.shows(&word), "a first start does not show {word:?}; on screen: {:?}", session.words());
        }
    }
}

probe! {
    /// A FIRST START HAS NOTHING TO SAVE: the title marks no unsaved work, and opening a project asks nothing about
    /// it.
    fn a_first_start_has_nothing_to_save() {
        let mut s = Session::start();
        let title = s.title();
        assert!(!title.ends_with('*'), "a program just started, with nothing done in it, calls its document unsaved: {title:?}");
        let file = s.word("menu-file");
        let file = s.find(&file, qymcad::pos2(0.0, 0.0)).expect("the File menu is on screen");
        s.click(file.center());
        let open = s.word("file-open");
        let open = s.find(&open, file.center()).expect("Open project is in the File menu");
        s.click(open.center());
        let question = s.word("nav-unsaved-title");
        assert!(!s.shows(&question), "opening a project right after the start asks what to do with unsaved changes");
    }
}

probe! {
    /// A FIRST START SPEAKS THE MACHINE'S LANGUAGE EVERYWHERE, the status line included.
    fn a_first_start_speaks_the_machine_s_language_in_the_status_line() {
        let mut s = Session::start_on(qymcad::Machine { locale: "ru-RU".into(), ..qymcad::Machine::default() });
        let want = s.word("g-start-hint");
        assert_eq!(s.status(), want, "a first start on a Russian machine says its status line in another language");
    }
}

probe! {
    /// THE BAR OF A SKETCH SPEAKS THE PERSON'S LANGUAGE: on a Russian machine every word of it comes from the
    /// catalogue, and the switch of the snapping is a word like any other.
    fn the_bar_of_a_sketch_speaks_the_machine_s_language() {
        let mut s = Session::start_on(qymcad::Machine { locale: "ru-RU".into(), ..qymcad::Machine::default() });
        build::into_the_first_part(&mut s);
        let xy = s.word("plane-xy-table");
        s.press_word(&xy);
        let switch = s.widgets().into_iter().find(|w| w.label.ends_with("Snap")).map(|w| w.label);
        assert!(switch.is_none(), "the bar of the sketch holds a word of another language: {switch:?}");
    }
}
