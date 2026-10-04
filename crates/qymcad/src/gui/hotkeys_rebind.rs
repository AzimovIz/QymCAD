//! HOTKEYS CAN BE REBOUND.
//!
//! The reference sheet was for viewing only, and it could not have been otherwise: the handlers
//! matched on THE KEY (`match key { Key::E => extrude }`), that is, the letter and the meaning were
//! one and the same. Moving the letter would have meant rewriting the `match` — rebinding was
//! inexpressible by construction.
//!
//! Now THE ACTION comes first: a key leads to an action (`hotkey_action`), and the action to a branch
//! of the handler. The first link is what moves; the second is not touched at all.
#[cfg(test)]
mod tests {
    use super::super::hotkeys::{rebindable, HOTKEYS};
    use super::super::App;

    /// THE FACTORY LAYOUT IS IN FORCE while nobody has touched it.
    #[test]
    fn out_of_the_box_the_default_layout_is_in_force() {
        let app = App::default();
        assert_eq!(qymcad_ui_state::hotkey_action(&app.set, "part", egui::Key::E), Some("part.extrude"), "the factory E in a Part must extrude");
        assert_eq!(qymcad_ui_state::hotkey_action(&app.set, "sketch", egui::Key::L), Some("sketch.line"), "the factory L in a Sketch must draw a line");
        assert_eq!(qymcad_ui_state::hotkey_action(&app.set, "part", egui::Key::Z), None, "a free key is not obliged to mean anything");
    }

    /// THE POINT: after a rebind THE NEW key works and the old one stops.
    #[test]
    fn a_reassigned_key_takes_over_and_the_old_one_stops() {
        let mut app = App::default();
        app.set.hotkeys.insert("part.extrude".into(), "W".into());
        assert_eq!(qymcad_ui_state::hotkey_action(&app.set, "part", egui::Key::W), Some("part.extrude"), "the new key does not work — the rebinding is useless");
        assert_eq!(qymcad_ui_state::hotkey_action(&app.set, "part", egui::Key::E), None, "the old key still extrudes — now there are two of them");
    }

    /// A REBINDING LIVES IN THE SETTINGS, and so survives a restart.
    #[test]
    fn a_reassignment_survives_a_restart() {
        let mut app = App::default();
        app.set.hotkeys.insert("sketch.circle".into(), "Z".into());
        let ron = ron::ser::to_string(&app.set).expect("the settings serialise");
        let back: super::super::Settings = ron::from_str(&ron).expect("and read back");
        let restarted = App { set: back, ..Default::default() };
        assert_eq!(qymcad_ui_state::hotkey_action(&restarted.set, "sketch", egui::Key::Z), Some("sketch.circle"), "after a restart the key went back to the factory one — the edit is lost");
    }

    /// ONLY THE DIFFERENCES ARE STORED. A full layout in the record would mean that a new tool of the
    /// program never appears for anyone who has ever touched the keys: its action is simply not in the
    /// record.
    #[test]
    fn only_the_differences_are_stored() {
        let app = App::default();
        assert!(app.set.hotkeys.is_empty(), "the factory layout must not be stored — it is known anyway");
    }

    /// A TAKEN KEY IS REPORTED BEFORE IT IS ASSIGNED. Two commands on one key is not "the last one
    /// wins", it is a silently lost tool.
    #[test]
    fn a_taken_key_is_reported_before_it_is_assigned() {
        let app = App::default();
        assert_eq!(qymcad_ui_state::hotkey_taken_by(&app.set, "part", "F", "part.extrude"), Some("part.fillet"), "the key F being taken in a Part went unnoticed");
        assert_eq!(qymcad_ui_state::hotkey_taken_by(&app.set, "part", "Z", "part.extrude"), None, "a free key was called taken");
        // one letter in DIFFERENT workbenches is not a conflict: F in a Sketch and F in a Part live apart
        assert_eq!(qymcad_ui_state::hotkey_taken_by(&app.set, "sketch", "B", "sketch.line"), None, "a key of another workbench was counted as taken");
    }

    /// THE SYSTEM KEYS ARE NOT REBOUND — and that shows in the data, not in the good will of a window.
    #[test]
    fn the_system_keys_are_not_offered_for_rebinding() {
        assert!(!rebindable("general"), "the general area was given up for rebinding: Esc and Ctrl+Z belong to the system, not to us");
        for a in ["part", "sketch", "assembly"] {
            assert!(rebindable(a), "the \"{a}\" workbench must be rebindable");
        }
    }

    /// EVERY ACTION HAS A NAME OF ITS OWN, AND THE KEY LETTER IS NOT IN IT.
    ///
    /// The action name is the key of the settings record. Call it `part.e` and after a rebind to W the
    /// record "part.e = W" becomes a lie about itself; and a duplicate name would quietly glue two
    /// commands together.
    #[test]
    fn action_names_are_unique_and_say_nothing_about_the_key() {
        let mut seen: Vec<&str> = Vec::new();
        for r in HOTKEYS {
            assert!(!seen.contains(&r.action), "the action \"{}\" is declared twice", r.action);
            seen.push(r.action);
            let tail = r.action.split_once('.').map(|(_, t)| t).unwrap_or("");
            assert!(tail.len() > 1, "the action name \"{}\" is made of the key letter — after a rebind it will lie", r.action);
        }
    }

    /// AND THE FACTORY KEYS WITHIN ONE WORKBENCH DO NOT ARGUE WITH EACH OTHER.
    #[test]
    fn the_default_layout_has_no_conflicts() {
        for area in super::super::hotkeys::AREAS {
            let mut used: Vec<(&str, &str)> = Vec::new();
            for r in HOTKEYS.iter().filter(|r| r.area == area) {
                if let Some((k, other)) = used.iter().find(|(k, _)| *k == r.key) {
                    panic!("in \"{area}\" the key {k} is taken twice: \"{other}\" and \"{}\"", r.action);
                }
                used.push((r.key, r.action));
            }
        }
    }

    use super::super::hotkeys::{capture_outcome, Capture};
    use egui::{Key, Modifiers};
    use qymcad_ui_state::{resolve_hotkey_clash, Chord, ClashChoice, HotkeyClash};

    /// A CHORD READS BACK AS IT IS WRITTEN, and the order of the modifiers in a hand-edited record does not matter.
    #[test]
    fn a_chord_reads_back_as_written() {
        for s in ["W", "Shift+W", "Ctrl+W", "Ctrl+Shift+F5", "7"] {
            assert_eq!(Chord::parse(s).map(|c| c.name()).as_deref(), Some(s), "{s} did not survive a round trip");
        }
        assert_eq!(Chord::parse("Shift+Ctrl+W"), Chord::parse("Ctrl+Shift+W"), "the same chord written the other way round is another key");
        for s in ["", "Ctrl+", "Ctrl+Z / Ctrl+Y", "Hyper+W", "NoSuchKey"] {
            assert_eq!(Chord::parse(s), None, "{s:?} was read as a chord");
        }
    }

    /// THE WINDOW RECORDS A CHORD, NOT ONLY A LETTER.
    #[test]
    fn the_window_records_a_chord() {
        let app = App::default();
        let mods = Modifiers { shift: true, ..Modifiers::COMMAND };
        assert_eq!(capture_outcome(&app.set, "part", "part.extrude", Key::J, mods), Capture::Bind("Ctrl+Shift+J".into()));
        assert_eq!(capture_outcome(&app.set, "part", "part.extrude", Key::W, Modifiers::NONE), Capture::Bind("W".into()));
    }

    /// WHAT THE SYSTEM HOLDS IS REFUSED: undo, the clipboard, Space, F1 - and Alt, which reaches keys from a field.
    #[test]
    fn the_window_refuses_what_belongs_to_the_system() {
        let app = App::default();
        for (key, mods) in [
            (Key::Z, Modifiers::COMMAND),
            (Key::S, Modifiers::COMMAND),
            (Key::K, Modifiers::COMMAND),
            (Key::Space, Modifiers::NONE),
            (Key::F1, Modifiers::NONE),
            (Key::Enter, Modifiers::NONE),
            (Key::ArrowUp, Modifiers::NONE),
        ] {
            assert!(matches!(capture_outcome(&app.set, "part", "part.extrude", key, mods), Capture::Refused(_)), "{key:?} with {mods:?} was accepted for a tool");
        }
        assert_eq!(capture_outcome(&app.set, "part", "part.extrude", Key::W, Modifiers::ALT), Capture::Refused("hotkeys-no-alt"));
        // bare X belongs to the general area - except to the action whose factory key it is
        assert!(matches!(capture_outcome(&app.set, "sketch", "sketch.line", Key::X, Modifiers::NONE), Capture::Refused(_)), "bare X went to a tool, and it toggles construction everywhere");
        assert!(
            !matches!(capture_outcome(&app.set, "sketch", "sketch.construction", Key::X, Modifiers::NONE), Capture::Refused(_)),
            "the construction toggle cannot be put back on its own factory key"
        );
    }

    /// THE GENERAL CTRL LETTERS ARE REFUSED WITH SHIFT AS WELL. Their handlers do not look at Shift: Ctrl+Shift+A
    /// still selects all, Ctrl+Shift+K still opens the search, Ctrl+Shift+Y still redoes - a tool there would
    /// fire together with them. Shift+X is free: the construction toggle asks for a bare X.
    #[test]
    fn the_general_ctrl_letters_are_refused_with_shift_too() {
        let app = App::default();
        let ctrl_shift = Modifiers { shift: true, ..Modifiers::COMMAND };
        for key in [Key::A, Key::C, Key::K, Key::S, Key::V, Key::X, Key::Y, Key::Z] {
            for mods in [Modifiers::COMMAND, ctrl_shift] {
                assert_eq!(capture_outcome(&app.set, "part", "part.extrude", key, mods), Capture::Refused("hotkeys-reserved"), "{key:?} with {mods:?} went to a tool");
            }
        }
        assert_eq!(capture_outcome(&app.set, "part", "part.extrude", Key::X, Modifiers::SHIFT), Capture::Bind("Shift+X".into()));
    }

    /// ON LINUX AND WINDOWS THE LETTERS A FIELD EDITS WITH ARE REFUSED UNDER CTRL: egui erases with Ctrl+H, Ctrl+U
    /// and Ctrl+W inside a field, Shift or not, and a tool there would run while the expression lost a word.
    #[test]
    fn on_linux_and_windows_the_letters_a_field_edits_with_are_refused() {
        use qymcad_ui_state::{hotkey_refusal_on, platform_keys::Os};
        for os in [Os::Linux, Os::Windows] {
            for key in [Key::H, Key::U, Key::W] {
                for shift in [false, true] {
                    let chord = Chord { ctrl: true, shift, ..Chord::from(key) };
                    assert_eq!(hotkey_refusal_on(os, "part.extrude", &chord), Some("hotkeys-field-edits"), "{os:?}: {} went to a tool", chord.name());
                }
            }
            // the bare letters stay free: a field types them, and Alt reaches the tool from there
            assert_eq!(hotkey_refusal_on(os, "part.extrude", &Chord::from(Key::W)), None);
        }
    }

    /// ON A MAC THE BINDING'S CTRL IS CMD, which a field does not edit with: Cmd+U and Cmd+W are free there. What the
    /// window takes is Cmd+H (hide) and Cmd+Q (quit) - each exactly, so Shift+Cmd+H is free.
    #[test]
    fn on_a_mac_cmd_h_and_cmd_q_are_kept_and_the_field_letters_are_free() {
        use qymcad_ui_state::{hotkey_refusal_on, platform_keys::Os};
        let cmd = |key| Chord { ctrl: true, ..Chord::from(key) };
        assert_eq!(hotkey_refusal_on(Os::Mac, "part.extrude", &cmd(Key::H)), Some("hotkeys-os-hide"));
        assert_eq!(hotkey_refusal_on(Os::Mac, "part.extrude", &cmd(Key::Q)), Some("hotkeys-os-quit"));
        for key in [Key::U, Key::W] {
            assert_eq!(hotkey_refusal_on(Os::Mac, "part.extrude", &cmd(key)), None, "Cmd+{key:?} edits nothing on a Mac and was refused");
        }
        assert_eq!(hotkey_refusal_on(Os::Mac, "part.extrude", &Chord { ctrl: true, shift: true, ..Chord::from(Key::H) }), None, "Shift+Cmd+H is no menu item");
        // the General letters are the program's own on every system
        assert_eq!(hotkey_refusal_on(Os::Mac, "part.extrude", &cmd(Key::S)), Some("hotkeys-reserved"));
    }

    /// EVERY SYSTEM HAS A TABLE OF ITS OWN, and every reason in a table has words.
    #[test]
    fn every_system_has_its_own_table() {
        use qymcad_ui_state::platform_keys::{platform_keys, Os};
        for os in Os::ALL {
            let t = platform_keys(os);
            assert_eq!(t.os, os, "{os:?} reads the table of {:?}", t.os);
            for k in t.rows() {
                assert!(Chord::parse(k.chord).is_some(), "{os:?}: the row {} is no chord and refuses nothing", k.chord);
                assert_ne!(crate::i18n::tr(k.why), k.why, "{os:?}: the reason {} has no words", k.why);
            }
        }
    }

    /// THE MAC'S CONTROL KEY IS RECORDED AS ITSELF: Control+J, written apart from Cmd+J, refused nowhere on a Mac but
    /// the letters its field edits with, and refused on the systems that have no such key.
    #[test]
    fn the_macs_control_key_is_recorded_as_itself() {
        use qymcad_ui_state::{hotkey_refusal_on, platform_keys::Os};
        let control = Chord::of_press(Modifiers { ctrl: true, ..Modifiers::NONE }, Key::J);
        assert_eq!(control.name(), "Control+J");
        let both = Chord::of_press(Modifiers { ctrl: true, mac_cmd: true, command: true, ..Modifiers::NONE }, Key::J);
        assert_eq!(both.name(), "Control+Ctrl+J");
        let cmd = Chord::of_press(Modifiers { mac_cmd: true, command: true, ..Modifiers::NONE }, Key::J);
        assert_eq!(cmd.name(), "Ctrl+J");
        // off a Mac the Ctrl key is the command key, and nothing else
        assert_eq!(Chord::of_press(Modifiers::COMMAND, Key::J).name(), "Ctrl+J");
        assert_eq!(Chord::parse("Control+Ctrl+J"), Some(both));
        assert_eq!(hotkey_refusal_on(Os::Mac, "part.extrude", &control), None);
        assert_eq!(hotkey_refusal_on(Os::Mac, "part.extrude", &both), None);
        for os in [Os::Linux, Os::Windows] {
            assert_eq!(hotkey_refusal_on(os, "part.extrude", &control), Some("hotkeys-mac-only"), "{os:?} has no Control key apart from Ctrl");
        }
    }

    /// ON A MAC THE FIELD'S CONTROL LETTERS AND THE SYSTEM'S COMBINATIONS ARE KEPT, each as egui and macOS answer
    /// them: the erasers with Shift too, the caret keys without it, the menu and system items exactly.
    #[test]
    fn on_a_mac_the_field_and_the_system_keep_their_combinations() {
        use qymcad_ui_state::{hotkey_refusal_on, platform_keys::Os};
        let on_mac = |s: &str| hotkey_refusal_on(Os::Mac, "part.extrude", &Chord::parse(s).expect(s));
        for s in ["Control+H", "Control+Shift+W", "Control+Ctrl+U", "Control+A", "Control+N"] {
            assert_eq!(on_mac(s), Some("hotkeys-mac-field-edits"), "{s} edits a field and went to a tool");
        }
        assert_eq!(on_mac("Control+Shift+A"), None, "Shift with Control+A moves no caret");
        for s in ["Ctrl+Shift+3", "Ctrl+Shift+4", "Ctrl+Shift+5", "Ctrl+Shift+Q", "Control+Ctrl+Q", "Control+F3", "Control+F8"] {
            assert_eq!(on_mac(s), Some("hotkeys-os-system"), "{s} belongs to macOS and went to a tool");
        }
        for s in ["Ctrl+3", "Ctrl+Shift+6", "Control+F9", "Control+J", "Control+Shift+F3"] {
            assert_eq!(on_mac(s), None, "{s} is free on a Mac and was refused");
        }
    }

    /// THE FACTORY LAYOUT IS UNIVERSAL: no factory key of a workbench is refused on any system. Its own action is
    /// exempt from refusal, so this is asked of the tables directly.
    #[test]
    fn the_factory_keys_are_free_on_every_system() {
        use qymcad_ui_state::platform_keys::{platform_keys, Os};
        for os in Os::ALL {
            for r in HOTKEYS.iter().filter(|r| rebindable(r.area)) {
                let chord = Chord::parse(r.key).expect(r.key);
                let why = platform_keys(os).refusal(&chord);
                // bare X: the sketch's construction toggle is the General X itself
                assert!(why.is_none() || r.key == "X", "{os:?}: the factory key {} of {} is refused ({why:?})", r.key, r.action);
            }
        }
    }

    /// THE GENERAL CTRL LETTERS ARE THE ONES ITS HANDLERS LISTEN TO. A shortcut added to the frame and forgotten
    /// here would be offered to a tool, and both would fire; one dropped from the frame would stay refused for
    /// nothing. So the list is read against every `Ctrl + letter` the handlers ask for.
    #[test]
    fn the_general_ctrl_letters_are_the_ones_the_handlers_hear() {
        let mut heard: Vec<String> = Vec::new();
        for src in [include_str!("input.rs"), include_str!("../gui.rs"), include_str!("../../../qymcad-ui-state/src/lib.rs")] {
            for line in src.lines().filter(|l| !l.trim_start().starts_with("//") && (l.contains("modifiers.command") || l.contains("cmd &&"))) {
                for part in line.split("key_pressed(egui::Key::").skip(1) {
                    let name: String = part.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
                    if name.len() == 1 && !heard.contains(&name) {
                        heard.push(name);
                    }
                }
            }
        }
        heard.sort();
        let mut listed: Vec<String> = qymcad_ui_state::platform_keys::GENERAL.iter().filter_map(|k| Chord::parse(k.chord)).filter(|c| c.ctrl).map(|c| c.key.name().to_string()).collect();
        listed.sort();
        assert_eq!(heard, listed, "the Ctrl letters the frame handles and the ones refused to a tool have parted");
    }

    /// ESC LEAVES, BACKSPACE ERASES.
    #[test]
    fn escape_leaves_and_backspace_erases() {
        let app = App::default();
        assert_eq!(capture_outcome(&app.set, "part", "part.extrude", Key::Escape, Modifiers::NONE), Capture::Cancel);
        assert_eq!(capture_outcome(&app.set, "part", "part.extrude", Key::Backspace, Modifiers::NONE), Capture::Bind(String::new()));
    }

    /// A TAKEN KEY IS A QUESTION, NOT A REFUSAL.
    #[test]
    fn a_taken_key_becomes_a_question() {
        let app = App::default();
        assert_eq!(capture_outcome(&app.set, "part", "part.extrude", Key::F, Modifiers::NONE), Capture::Clash(HotkeyClash { action: "part.extrude".into(), chord: "F".into(), holder: "part.fillet" }));
    }

    /// SWAP: each gets the other's key, and the record holds both as differences from the factory.
    #[test]
    fn a_swap_gives_each_the_others_key() {
        let mut app = App::default();
        let clash = HotkeyClash { action: "part.extrude".into(), chord: "F".into(), holder: "part.fillet" };
        resolve_hotkey_clash(&mut app.set, &clash, ClashChoice::Swap);
        assert_eq!(qymcad_ui_state::hotkey_action(&app.set, "part", Key::F), Some("part.extrude"));
        assert_eq!(qymcad_ui_state::hotkey_action(&app.set, "part", Key::E), Some("part.fillet"));
        // and swapping back leaves the record clean
        let back = HotkeyClash { action: "part.extrude".into(), chord: "E".into(), holder: "part.fillet" };
        resolve_hotkey_clash(&mut app.set, &back, ClashChoice::Swap);
        assert!(app.set.hotkeys.is_empty(), "swapped back to the factory keys and the record still holds {:?}", app.set.hotkeys);
    }

    /// TAKE: the asking action gets the key, the holder is left with none - and no key at all runs it.
    #[test]
    fn taking_a_key_leaves_the_holder_without_one() {
        let mut app = App::default();
        let clash = HotkeyClash { action: "part.extrude".into(), chord: "F".into(), holder: "part.fillet" };
        resolve_hotkey_clash(&mut app.set, &clash, ClashChoice::Unbind);
        assert_eq!(qymcad_ui_state::hotkey_action(&app.set, "part", Key::F), Some("part.extrude"));
        assert_eq!(qymcad_ui_state::hotkey_key(&app.set, "part.fillet"), "", "the fillet kept a key");
        assert_eq!(qymcad_ui_state::hotkey_action(&app.set, "part", Key::E), None, "the old key of the extrusion still runs something");
    }

    /// EVERY FACTORY KEY OF A WORKBENCH IS A CHORD THE DISPATCHER CAN HEAR. A factory key the parser does not
    /// read would be a tool that never runs.
    #[test]
    fn every_factory_workbench_key_is_a_bindable_chord() {
        for r in HOTKEYS.iter().filter(|r| rebindable(r.area)) {
            let c = Chord::parse(r.key).unwrap_or_else(|| panic!("the factory key {} of {} is not a chord", r.key, r.action));
            assert!(c.bindable_key(), "the factory key {} of {} is one the dispatcher never hears", r.key, r.action);
        }
    }

    /// ON A MAC THE KEYS ARE WRITTEN AS A MAC WRITES THEM: ⌘ for the command key the binding's "Ctrl" stands
    /// for, ⌥ for Alt, in Apple's order - and the general rows, written as text, change with the rest.
    #[test]
    fn a_mac_writes_keys_with_symbols() {
        use qymcad_ui_state::{key_label_in, KeyStyle::MacSymbols};
        assert_eq!(key_label_in("Ctrl+W", MacSymbols), "⌘W");
        assert_eq!(key_label_in("Ctrl+Shift+F5", MacSymbols), "⇧⌘F5", "Shift goes before Command, as on every Mac menu");
        assert_eq!(key_label_in("Alt+U", MacSymbols), "⌥U");
        assert_eq!(key_label_in("Ctrl+Z / Ctrl+Y", MacSymbols), "⌘Z / ⌘Y");
        assert_eq!(key_label_in("E", MacSymbols), "E");
        assert_eq!(key_label_in("", MacSymbols), "", "an action without a key stays without a label");
    }

    /// WITHOUT THE SYMBOL FONT, WORDS - a box where ⌥ should be tells nobody anything.
    #[test]
    fn a_mac_without_the_symbols_writes_words() {
        use qymcad_ui_state::{key_label_in, KeyStyle::MacWords};
        assert_eq!(key_label_in("Ctrl+Shift+W", MacWords), "Shift+Cmd+W");
        assert_eq!(key_label_in("Alt+U", MacWords), "Option+U");
    }

    /// ONLY THE LABEL CHANGES. Off a Mac it is the stored spelling as it is, and the symbols a Mac is given are
    /// the four the program asks Apple Symbols for - nothing that would turn out a box.
    #[test]
    fn off_a_mac_the_label_is_the_stored_spelling() {
        use qymcad_ui_state::{key_label_in, KeyStyle};
        assert_eq!(key_label_in("Ctrl+Shift+W", KeyStyle::Plain), "Ctrl+Shift+W");
        if !cfg!(target_os = "macos") {
            assert_eq!(qymcad_ui_state::key_style(), KeyStyle::Plain, "keys off a Mac are written in Mac style");
        }
        for r in HOTKEYS {
            let shown = key_label_in(r.key, KeyStyle::MacSymbols);
            assert!(shown.chars().all(|c| c.is_ascii() || "⌃⌘⇧⌥".contains(c)), "{} is shown on a Mac with a glyph no loaded font promises: {shown}", r.key);
        }
    }
}
