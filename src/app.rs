use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;

use crate::config::{Config, Entry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    Command,
    Form,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Command,
    Heading,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Type,
    Name,
    Description,
    Command,
}

pub struct FormState {
    pub kind: EntryKind,
    pub name: String,
    pub description: String,
    pub command: String,
    pub field: FormField,
    pub editing: bool,
    pub edit_index: Option<usize>,
}

impl FormState {
    fn new_entry() -> FormState {
        FormState {
            kind: EntryKind::Command,
            name: String::new(),
            description: String::new(),
            command: String::new(),
            field: FormField::Name,
            editing: false,
            edit_index: None,
        }
    }

    pub fn fields(&self) -> Vec<FormField> {
        match self.kind {
            EntryKind::Heading => vec![FormField::Type, FormField::Name],
            EntryKind::Command => vec![
                FormField::Type,
                FormField::Name,
                FormField::Description,
                FormField::Command,
            ],
        }
    }

    fn field_text_mut(&mut self) -> Option<&mut String> {
        match self.field {
            FormField::Name => Some(&mut self.name),
            FormField::Description => Some(&mut self.description),
            FormField::Command => Some(&mut self.command),
            FormField::Type => None,
        }
    }

    fn move_field(&mut self, delta: isize) {
        let fields = self.fields();
        let cur = fields.iter().position(|f| *f == self.field).unwrap_or(0) as isize;
        let len = fields.len() as isize;
        let next = (cur + delta).rem_euclid(len) as usize;
        self.field = fields[next];
        self.editing = false;
    }

    fn title(&self) -> &str {
        match self.edit_index {
            Some(_) => "Edit entry",
            None => "New entry",
        }
    }

    pub fn title_str(&self) -> &str {
        self.title()
    }
}

pub struct App {
    pub config: Config,
    pub mode: Mode,
    pub query: String,
    pub cmdline: String,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub list_state: ListState,
    pub form: FormState,
    pub pending: Option<char>,
    pub status: String,
    pub show_help: bool,
    pub should_quit: bool,
    pub run_request: Option<String>,
}

impl App {
    pub fn new(config: Config) -> App {
        let mut app = App {
            config,
            mode: Mode::Normal,
            query: String::new(),
            cmdline: String::new(),
            filtered: Vec::new(),
            selected: 0,
            list_state: ListState::default(),
            form: FormState::new_entry(),
            pending: None,
            status: String::new(),
            show_help: false,
            should_quit: false,
            run_request: None,
        };
        app.recompute();
        app
    }

    pub fn entry_at_filtered(&self, fi: usize) -> Option<&Entry> {
        self.filtered.get(fi).map(|i| &self.config.entries[*i])
    }

    pub fn current_entry_index(&self) -> Option<usize> {
        self.filtered.get(self.selected).copied()
    }

    fn recompute(&mut self) {
        let q = self.query.trim().to_lowercase();
        self.filtered.clear();
        if q.is_empty() {
            self.filtered = (0..self.config.entries.len()).collect();
        } else {
            let words: Vec<&str> = q.split_whitespace().collect();
            let (mut exact, mut prefix, mut substr) = (Vec::new(), Vec::new(), Vec::new());
            for (i, e) in self.config.entries.iter().enumerate() {
                if !e.is_runnable() {
                    continue;
                }
                let hay = format!("{} {}", e.name_str(), e.description_str()).to_lowercase();
                if !words.iter().all(|w| hay.contains(*w)) {
                    continue;
                }
                if words.iter().all(|w| word_match(&hay, w, true)) {
                    exact.push(i);
                } else if words.iter().all(|w| word_match(&hay, w, false)) {
                    prefix.push(i);
                } else {
                    substr.push(i);
                }
            }
            self.filtered.extend(exact);
            self.filtered.extend(prefix);
            self.filtered.extend(substr);
        }
        self.ensure_valid_selection();
    }

    fn ensure_valid_selection(&mut self) {
        if self.filtered.is_empty() {
            self.selected = 0;
            self.list_state.select(None);
            return;
        }
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
        if !self.entry_at_filtered(self.selected).map(|e| e.is_runnable()).unwrap_or(false) {
            // snap to nearest runnable: search down then up
            let len = self.filtered.len();
            let mut found = None;
            for off in 0..len {
                let down = self.selected + off;
                if down < len && self.entry_at_filtered(down).map(|e| e.is_runnable()).unwrap_or(false) {
                    found = Some(down);
                    break;
                }
                if off <= self.selected {
                    let up = self.selected - off;
                    if self.entry_at_filtered(up).map(|e| e.is_runnable()).unwrap_or(false) {
                        found = Some(up);
                        break;
                    }
                }
            }
            if let Some(f) = found {
                self.selected = f;
            }
        }
        self.list_state.select(Some(self.selected));
    }

    fn move_selection(&mut self, delta: isize) {
        if self.filtered.is_empty() {
            return;
        }
        let len = self.filtered.len() as isize;
        let mut idx = self.selected as isize;
        loop {
            idx += delta;
            if idx < 0 || idx >= len {
                return;
            }
            if self.entry_at_filtered(idx as usize).map(|e| e.is_runnable()).unwrap_or(false) {
                self.selected = idx as usize;
                self.list_state.select(Some(self.selected));
                return;
            }
        }
    }

    fn jump(&mut self, delta: isize, times: usize) {
        for _ in 0..times {
            let before = self.selected;
            self.move_selection(delta);
            if self.selected == before {
                break;
            }
        }
    }

    fn go_edge(&mut self, first: bool) {
        if self.filtered.is_empty() {
            return;
        }
        // Set to the edge, then snap to the nearest runnable row.
        self.selected = if first { 0 } else { self.filtered.len() - 1 };
        self.ensure_valid_selection();
    }

    fn select_config_index(&mut self, ci: usize) {
        if let Some(pos) = self.filtered.iter().position(|i| *i == ci) {
            self.selected = pos;
            self.list_state.select(Some(pos));
        }
    }

    // ---- key handling ----

    pub fn on_key(&mut self, key: KeyEvent) {
        self.status.clear();
        if self.show_help {
            self.show_help = false;
            return;
        }
        match self.mode {
            Mode::Normal => self.on_key_normal(key),
            Mode::Search => self.on_key_search(key),
            Mode::Command => self.on_key_command(key),
            Mode::Form => self.on_key_form(key),
            Mode::Confirm => self.on_key_confirm(key),
        }
    }

    fn on_key_normal(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        // resolve pending two-key sequences
        if let Some(p) = self.pending.take() {
            match (p, key.code) {
                ('g', KeyCode::Char('g')) => {
                    self.go_edge(true);
                    return;
                }
                ('d', KeyCode::Char('d')) => {
                    self.begin_delete();
                    return;
                }
                _ => {} // fall through to handle this key fresh
            }
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('d') if ctrl => self.jump(1, 8),
            KeyCode::Char('u') if ctrl => self.jump(-1, 8),
            KeyCode::Char('g') => self.pending = Some('g'),
            KeyCode::Char('G') => self.go_edge(false),
            KeyCode::Char('d') => self.pending = Some('d'),
            KeyCode::Char('/') => {
                self.mode = Mode::Search;
            }
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.cmdline.clear();
            }
            KeyCode::Enter | KeyCode::Char('l') => self.activate(),
            KeyCode::Char('o') => self.open_create(),
            KeyCode::Char('c') => self.open_edit(),
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc if !self.query.is_empty() => {
                self.query.clear();
                self.recompute();
            }
            _ => {}
        }
    }

    fn on_key_search(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => {
                self.query.clear();
                self.recompute();
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => self.mode = Mode::Normal,
            KeyCode::Backspace => {
                self.query.pop();
                self.recompute();
            }
            KeyCode::Char('u') if ctrl => {
                self.query.clear();
                self.recompute();
            }
            KeyCode::Char('w') if ctrl => {
                delete_last_word(&mut self.query);
                self.recompute();
            }
            KeyCode::Char(c) if !ctrl => {
                self.query.push(c);
                self.recompute();
            }
            _ => {}
        }
    }

    fn on_key_command(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.cmdline.clear();
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                self.cmdline.pop();
            }
            KeyCode::Enter => {
                let cmd = self.cmdline.trim().to_string();
                self.cmdline.clear();
                self.mode = Mode::Normal;
                match cmd.as_str() {
                    "q" | "q!" | "wq" | "x" => self.should_quit = true,
                    "" => {}
                    other => self.status = format!("Unknown command: :{other}"),
                }
            }
            KeyCode::Char(c) => self.cmdline.push(c),
            _ => {}
        }
    }

    fn on_key_form(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('s') {
            self.save_form();
            return;
        }
        if self.form.editing {
            match key.code {
                KeyCode::Esc => self.form.editing = false,
                KeyCode::Enter => self.form.editing = false,
                KeyCode::Backspace => {
                    if let Some(s) = self.form.field_text_mut() {
                        s.pop();
                    }
                }
                KeyCode::Char(c) if !ctrl => {
                    if let Some(s) = self.form.field_text_mut() {
                        s.push(c);
                    }
                }
                _ => {}
            }
            return;
        }
        // field-normal
        if let Some('Z') = self.pending.take() {
            if key.code == KeyCode::Char('Z') {
                self.save_form();
                return;
            }
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Tab => self.form.move_field(1),
            KeyCode::Char('k') | KeyCode::Up | KeyCode::BackTab => self.form.move_field(-1),
            KeyCode::Char('Z') => self.pending = Some('Z'),
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char('i') | KeyCode::Char('a') | KeyCode::Enter => {
                if self.form.field == FormField::Type {
                    self.toggle_kind();
                } else {
                    self.form.editing = true;
                }
            }
            KeyCode::Char(' ') | KeyCode::Char('h') | KeyCode::Char('l')
                if self.form.field == FormField::Type =>
            {
                self.toggle_kind();
            }
            _ => {}
        }
    }

    fn on_key_confirm(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_delete(),
            _ => self.mode = Mode::Normal,
        }
    }

    fn toggle_kind(&mut self) {
        self.form.kind = match self.form.kind {
            EntryKind::Command => EntryKind::Heading,
            EntryKind::Heading => EntryKind::Command,
        };
        // keep field valid for the new kind
        let fields = self.form.fields();
        if !fields.contains(&self.form.field) {
            self.form.field = FormField::Name;
        }
    }

    fn activate(&mut self) {
        if let Some(i) = self.current_entry_index() {
            let e = &self.config.entries[i];
            if e.is_runnable() {
                self.run_request = Some(e.command_str().to_string());
            }
        }
    }

    fn open_create(&mut self) {
        self.form = FormState::new_entry();
        self.mode = Mode::Form;
    }

    fn open_edit(&mut self) {
        let Some(i) = self.current_entry_index() else {
            return;
        };
        let e = self.config.entries[i].clone();
        let mut form = FormState::new_entry();
        form.edit_index = Some(i);
        if e.is_heading() {
            form.kind = EntryKind::Heading;
            form.name = e.heading_str().to_string();
        } else {
            form.kind = EntryKind::Command;
            form.name = e.name_str().to_string();
            form.description = e.description_str().to_string();
            form.command = e.command_str().to_string();
        }
        form.field = FormField::Name;
        self.form = form;
        self.mode = Mode::Form;
    }

    fn save_form(&mut self) {
        let entry = match self.form.kind {
            EntryKind::Heading => {
                let title = self.form.name.trim();
                if title.is_empty() {
                    self.status = "Heading title cannot be empty".to_string();
                    return;
                }
                Entry::heading(title)
            }
            EntryKind::Command => {
                let name = self.form.name.trim();
                let cmd = self.form.command.trim();
                if name.is_empty() || cmd.is_empty() {
                    self.status = "Name and command are required".to_string();
                    return;
                }
                Entry::command(name, self.form.description.trim(), cmd)
            }
        };

        let target_index = match self.form.edit_index {
            Some(i) => {
                self.config.entries[i] = entry;
                i
            }
            None => {
                let at = match self.current_entry_index() {
                    Some(i) => i + 1,
                    None => self.config.entries.len(),
                };
                self.config.entries.insert(at, entry);
                at
            }
        };

        self.persist();
        self.mode = Mode::Normal;
        self.recompute();
        self.select_config_index(target_index);
    }

    fn begin_delete(&mut self) {
        if self.current_entry_index().is_some() {
            self.mode = Mode::Confirm;
        }
    }

    fn confirm_delete(&mut self) {
        if let Some(i) = self.current_entry_index() {
            self.config.entries.remove(i);
            self.persist();
            self.recompute();
        }
        self.mode = Mode::Normal;
    }

    fn persist(&mut self) {
        if let Err(e) = self.config.save() {
            self.status = format!("Save failed: {e}");
        }
    }
}

fn word_match(hay: &str, word: &str, exact: bool) -> bool {
    hay.split(|c: char| !c.is_alphanumeric()).any(|tok| {
        if exact {
            tok == word
        } else {
            tok.starts_with(word)
        }
    })
}

fn delete_last_word(s: &mut String) {
    while s.ends_with(|c: char| !c.is_alphanumeric()) {
        s.pop();
    }
    while s.ends_with(|c: char| c.is_alphanumeric()) {
        s.pop();
    }
}

/// Run a shell command outside the alternate screen (called by main after teardown).
pub fn run_command_blocking(command: &str) -> Result<()> {
    use std::io::{self, BufRead, Write};
    use std::process::Command;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    println!("\n$ {command}\n");
    let _ = Command::new(shell).arg("-c").arg(command).status();
    print!("\nPress <Enter> to continue.");
    io::stdout().flush().ok();
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line).ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Entry, Settings};

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }
    fn code(k: KeyCode) -> KeyEvent {
        KeyEvent::new(k, KeyModifiers::NONE)
    }
    fn type_str(app: &mut App, s: &str) {
        for c in s.chars() {
            app.on_key(key(c));
        }
    }

    fn sample() -> Config {
        Config {
            settings: Settings::default(),
            entries: vec![
                Entry::heading("Alpha"),
                Entry::command("apple", "red fruit", "echo apple"),
                Entry::command("apricot", "orange fruit", "echo apricot"),
                Entry::heading("Beta"),
                Entry::command("banana", "yellow fruit", "echo banana"),
            ],
        }
    }

    fn isolate_home() {
        let tmp = std::env::temp_dir().join(format!("tuimenu-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("HOME", &tmp);
    }

    #[test]
    fn navigation_skips_headings() {
        let mut app = App::new(sample());
        assert_eq!(app.current_entry_index(), Some(1)); // snaps past Alpha heading
        app.on_key(key('j'));
        assert_eq!(app.current_entry_index(), Some(2));
        app.on_key(key('j')); // skips Beta heading -> banana
        assert_eq!(app.current_entry_index(), Some(4));
        app.on_key(key('j')); // at bottom, stays
        assert_eq!(app.current_entry_index(), Some(4));
        app.on_key(key('g'));
        app.on_key(key('g'));
        assert_eq!(app.current_entry_index(), Some(1));
    }

    #[test]
    fn search_filters_and_hides_headings() {
        let mut app = App::new(sample());
        app.on_key(key('/'));
        type_str(&mut app, "apr");
        assert_eq!(app.filtered.len(), 1);
        assert_eq!(app.entry_at_filtered(0).unwrap().name_str(), "apricot");
        app.on_key(code(KeyCode::Enter));
        assert_eq!(app.mode, Mode::Normal);
        app.on_key(code(KeyCode::Esc));
        assert!(app.query.is_empty());
        assert_eq!(app.filtered.len(), 5);
    }

    #[test]
    fn delete_persists_to_disk() {
        isolate_home();
        let mut app = App::new(sample());
        let before = app.config.entries.len();
        app.on_key(key('d'));
        app.on_key(key('d'));
        assert_eq!(app.mode, Mode::Confirm);
        app.on_key(key('y'));
        assert_eq!(app.config.entries.len(), before - 1);

        let txt = std::fs::read_to_string(crate::config::config_path()).unwrap();
        assert!(txt.contains("[[entries]]"));
    }

    #[test]
    fn create_entry_via_form() {
        isolate_home();
        let mut app = App::new(sample());
        app.on_key(key('o'));
        assert_eq!(app.mode, Mode::Form);
        app.on_key(code(KeyCode::Enter)); // edit Name field
        type_str(&mut app, "zeta");
        app.on_key(code(KeyCode::Esc));
        app.on_key(key('j')); // -> Description
        app.on_key(key('j')); // -> Command
        app.on_key(code(KeyCode::Enter));
        type_str(&mut app, "echo zeta");
        app.on_key(code(KeyCode::Esc));
        app.on_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(app.mode, Mode::Normal);
        assert!(app
            .config
            .entries
            .iter()
            .any(|e| e.name_str() == "zeta" && e.command_str() == "echo zeta"));
    }
}
