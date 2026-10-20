//! Syntax highlighter for PowerShell (.ps1) and Windows Command Prompt (.cmd / .bat) scripts.
//! Produces formatted `egui::text::LayoutJob` with token styling matched to studio tokens.

use egui::{Color32, FontId, TextFormat};
use egui::text::LayoutJob;
use crate::gui::theme::{self, Tokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptLanguage {
    PowerShell,
    Batch,
    Lua,
}

impl ScriptLanguage {
    pub fn badge_name(&self) -> &'static str {
        match self {
            Self::PowerShell => "POWERSHELL",
            Self::Batch => "CMD / BATCH",
            Self::Lua => "LUA 5.4",
        }
    }

    #[allow(dead_code)]
    pub fn localized_subtitle(&self, loc: &crate::i18n::Strings) -> &'static str {
        match self {
            Self::PowerShell => loc.modal_pwsh_subtitle,
            Self::Batch => loc.modal_cmd_subtitle,
            Self::Lua => "Embedded Lua Automation Script",
        }
    }
}

const PWSH_KEYWORDS: &[&str] = &[
    "begin", "break", "catch", "class", "continue", "data", "do", "dynamicparam",
    "else", "elseif", "end", "exit", "filter", "finally", "for", "foreach", "from",
    "function", "if", "in", "inlinescript", "parallel", "param", "process", "return",
    "sequence", "switch", "throw", "trap", "try", "until", "using", "var", "while",
    "workflow",
];

const PWSH_OPERATORS: &[&str] = &[
    "-eq", "-ne", "-gt", "-ge", "-lt", "-le", "-like", "-notlike",
    "-match", "-notmatch", "-replace", "-split", "-contains", "-notcontains",
    "-in", "-notin", "-is", "-isnot", "-and", "-or", "-xor", "-not", "-band",
    "-bor", "-bxor", "-bnot", "-shl", "-shr",
];

const PWSH_CMDLETS: &[&str] = &[
    "write-host", "write-output", "write-warning", "write-error", "write-verbose", "write-debug",
    "get-process", "start-process", "stop-process", "wait-process",
    "get-service", "start-service", "stop-service", "restart-service",
    "get-item", "set-item", "new-item", "remove-item", "copy-item", "move-item", "rename-item",
    "get-childitem", "test-path", "resolve-path", "split-path", "join-path",
    "get-content", "set-content", "add-content", "clear-content",
    "select-object", "where-object", "foreach-object", "sort-object", "measure-object",
    "out-file", "out-null", "out-string", "export-csv", "import-csv",
    "invoke-webrequest", "invoke-restmethod", "invoke-expression", "invoke-command",
    "start-sleep", "get-date", "set-location", "get-location", "push-location", "pop-location",
    "get-command", "get-help", "get-member", "get-variable", "set-variable",
];

const PWSH_ALIASES: &[&str] = &[
    "ls", "dir", "cd", "cat", "ps", "kill", "echo", "cp", "mv", "rm", "rmdir",
    "curl", "wget", "man", "sleep", "clear", "cls",
];

const CMD_KEYWORDS: &[&str] = &[
    "echo", "off", "on", "set", "setlocal", "endlocal", "enabledelayedexpansion",
    "disabledelayedexpansion", "if", "else", "exist", "defined", "errorlevel",
    "for", "in", "do", "goto", "call", "start", "exit", "pause", "rem", "cls",
    "cd", "chdir", "md", "mkdir", "rd", "rmdir", "dir", "del", "erase", "copy",
    "xcopy", "robocopy", "move", "ren", "rename", "type", "find", "findstr",
    "tasklist", "taskkill", "net", "sc", "reg", "ping", "powershell", "cmd",
    "timeout", "shutdown", "choice", "shift", "pushd", "popd", "title", "color",
];

fn append_span(
    job: &mut LayoutJob,
    text: &str,
    color: Color32,
    bold: bool,
    font_size: f32,
) {
    let font_family = if bold {
        theme::mono_bold()
    } else {
        theme::mono()
    };
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: FontId::new(font_size, font_family),
            color,
            ..Default::default()
        },
    );
}

fn highlight_powershell(text: &str, t: &Tokens, font_size: f32, job: &mut LayoutJob) {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let n = chars.len();
    let mut i = 0;

    let byte_end = |idx: usize| -> usize {
        if idx < n {
            chars[idx].0
        } else {
            text.len()
        }
    };

    while i < n {
        let (start_byte, ch) = chars[i];

        // 1. Block comment <# ... #>
        if ch == '<' && i + 1 < n && chars[i + 1].1 == '#' {
            let start = start_byte;
            i += 2;
            while i < n {
                if chars[i].1 == '#' && i + 1 < n && chars[i + 1].1 == '>' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            append_span(job, &text[start..byte_end(i)], t.muted.gamma_multiply(0.7), false, font_size);
            continue;
        }

        // 2. Line comment # ...
        if ch == '#' {
            let start = start_byte;
            while i < n && chars[i].1 != '\n' {
                i += 1;
            }
            append_span(job, &text[start..byte_end(i)], t.muted.gamma_multiply(0.7), false, font_size);
            continue;
        }

        // 3. String literal "..." or '...'
        if ch == '"' || ch == '\'' {
            let quote = ch;
            let start = start_byte;
            i += 1;
            while i < n {
                let cur = chars[i].1;
                if cur == '`' && quote == '"' && i + 1 < n {
                    // Backtick escape inside double quotes
                    i += 2;
                    continue;
                }
                if cur == quote {
                    if i + 1 < n && chars[i + 1].1 == quote {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                if cur == '\n' {
                    i += 1;
                    continue;
                }
                i += 1;
            }
            append_span(job, &text[start..byte_end(i)], t.warn, false, font_size);
            continue;
        }

        // 4. Variables $var, $_, $env:VAR, ${...}
        if ch == '$' {
            let start = start_byte;
            i += 1;
            if i < n && chars[i].1 == '{' {
                while i < n && chars[i].1 != '}' {
                    i += 1;
                }
                if i < n {
                    i += 1;
                }
            } else {
                while i < n {
                    let c = chars[i].1;
                    if c.is_alphanumeric() || c == '_' || c == ':' || c == '?' {
                        i += 1;
                    } else {
                        break;
                    }
                }
            }
            append_span(job, &text[start..byte_end(i)], t.accent_ink, false, font_size);
            continue;
        }

        // 5. Parameter or Comparison operator -Param, -eq, etc.
        if ch == '-' && i + 1 < n && (chars[i + 1].1.is_alphabetic() || chars[i + 1].1 == '-') {
            let start = start_byte;
            i += 1;
            while i < n && (chars[i].1.is_alphanumeric() || chars[i].1 == '_' || chars[i].1 == '-') {
                i += 1;
            }
            let word = &text[start..byte_end(i)];
            let lower = word.to_ascii_lowercase();
            if PWSH_OPERATORS.contains(&lower.as_str()) {
                append_span(job, word, t.ink_strong, true, font_size);
            } else {
                let color = if t.dark { Color32::from_rgb(120, 215, 255) } else { Color32::from_rgb(20, 120, 200) };
                append_span(job, word, color, false, font_size);
            }
            continue;
        }

        // 6. Number literals
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < n && chars[i + 1].1.is_ascii_digit()) {
            let start = start_byte;
            if ch == '0' && i + 1 < n && (chars[i + 1].1 == 'x' || chars[i + 1].1 == 'X') {
                i += 2;
                while i < n && (chars[i].1.is_ascii_hexdigit() || chars[i].1 == '_') {
                    i += 1;
                }
            } else {
                while i < n && (chars[i].1.is_ascii_digit() || chars[i].1 == '.' || chars[i].1 == '_' || chars[i].1 == 'e' || chars[i].1 == 'E') {
                    i += 1;
                }
            }
            let num_color = if t.dark { Color32::from_rgb(190, 155, 255) } else { Color32::from_rgb(120, 75, 200) };
            append_span(job, &text[start..byte_end(i)], num_color, false, font_size);
            continue;
        }

        // 7. Identifiers / Keywords / Cmdlets
        if ch.is_alphabetic() || ch == '_' {
            let start = start_byte;
            while i < n && (chars[i].1.is_alphanumeric() || chars[i].1 == '_' || chars[i].1 == '-') {
                i += 1;
            }
            let word = &text[start..byte_end(i)];
            let lower = word.to_ascii_lowercase();

            if PWSH_KEYWORDS.contains(&lower.as_str()) {
                append_span(job, word, t.ink_strong, true, font_size);
            } else if PWSH_CMDLETS.contains(&lower.as_str()) || PWSH_ALIASES.contains(&lower.as_str()) || (word.contains('-') && word.split('-').count() == 2) {
                append_span(job, word, t.good, false, font_size);
            } else {
                append_span(job, word, t.ink, false, font_size);
            }
            continue;
        }

        // 8. Delimiters & Operators
        if "{}()[]|,;=<>+*/%".contains(ch) {
            append_span(job, &text[start_byte..byte_end(i + 1)], t.faint, false, font_size);
            i += 1;
            continue;
        }

        // 9. Whitespace and other characters
        append_span(job, &text[start_byte..byte_end(i + 1)], t.ink, false, font_size);
        i += 1;
    }
}

fn highlight_batch(text: &str, t: &Tokens, font_size: f32, job: &mut LayoutJob) {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let n = chars.len();
    let mut i = 0;

    let byte_end = |idx: usize| -> usize {
        if idx < n {
            chars[idx].0
        } else {
            text.len()
        }
    };

    let mut at_line_start = true;

    while i < n {
        let (start_byte, ch) = chars[i];

        // 1. Comments starting with :: or REM at line start
        if at_line_start {
            if ch == ':' && i + 1 < n && chars[i + 1].1 == ':' {
                let start = start_byte;
                while i < n && chars[i].1 != '\n' {
                    i += 1;
                }
                append_span(job, &text[start..byte_end(i)], t.muted.gamma_multiply(0.7), false, font_size);
                at_line_start = true;
                if i < n && chars[i].1 == '\n' {
                    append_span(job, "\n", t.ink, false, font_size);
                    i += 1;
                }
                continue;
            }
            if (ch == 'R' || ch == 'r') && i + 2 < n {
                let next_two = (chars[i + 1].1.to_ascii_uppercase(), chars[i + 2].1.to_ascii_uppercase());
                if next_two == ('E', 'M') && (i + 3 == n || chars[i + 3].1.is_whitespace()) {
                    let start = start_byte;
                    while i < n && chars[i].1 != '\n' {
                        i += 1;
                    }
                    append_span(job, &text[start..byte_end(i)], t.muted.gamma_multiply(0.7), false, font_size);
                    at_line_start = true;
                    if i < n && chars[i].1 == '\n' {
                        append_span(job, "\n", t.ink, false, font_size);
                        i += 1;
                    }
                    continue;
                }
            }
        }

        // 2. Labels :LABEL
        if at_line_start && ch == ':' && i + 1 < n && chars[i + 1].1.is_alphanumeric() {
            let start = start_byte;
            while i < n && !chars[i].1.is_whitespace() {
                i += 1;
            }
            append_span(job, &text[start..byte_end(i)], t.good, true, font_size);
            at_line_start = false;
            continue;
        }

        if !ch.is_whitespace() {
            at_line_start = false;
        }
        if ch == '\n' {
            at_line_start = true;
            append_span(job, "\n", t.ink, false, font_size);
            i += 1;
            continue;
        }

        // 3. String literals "..."
        if ch == '"' {
            let start = start_byte;
            i += 1;
            while i < n && chars[i].1 != '"' && chars[i].1 != '\n' {
                i += 1;
            }
            if i < n && chars[i].1 == '"' {
                i += 1;
            }
            append_span(job, &text[start..byte_end(i)], t.warn, false, font_size);
            continue;
        }

        // 4. Variables %VAR%, %~dp0, !VAR!
        if ch == '%' {
            let start = start_byte;
            i += 1;
            if i < n && chars[i].1 == '%' {
                i += 1;
                while i < n && chars[i].1.is_alphanumeric() {
                    i += 1;
                }
            } else if i < n && chars[i].1 == '~' {
                while i < n && !chars[i].1.is_whitespace() && chars[i].1 != '%' {
                    i += 1;
                }
            } else {
                while i < n && chars[i].1 != '%' && chars[i].1 != '\n' && !chars[i].1.is_whitespace() {
                    i += 1;
                }
                if i < n && chars[i].1 == '%' {
                    i += 1;
                }
            }
            append_span(job, &text[start..byte_end(i)], t.accent_ink, false, font_size);
            continue;
        }
        if ch == '!' {
            let start = start_byte;
            i += 1;
            while i < n && chars[i].1 != '!' && chars[i].1 != '\n' {
                i += 1;
            }
            if i < n && chars[i].1 == '!' {
                i += 1;
            }
            append_span(job, &text[start..byte_end(i)], t.accent_ink, false, font_size);
            continue;
        }

        // 5. Switches /s /q /y
        if ch == '/' && i + 1 < n && chars[i + 1].1.is_alphanumeric() {
            let start = start_byte;
            i += 1;
            while i < n && chars[i].1.is_alphanumeric() {
                i += 1;
            }
            let color = if t.dark { Color32::from_rgb(120, 215, 255) } else { Color32::from_rgb(20, 120, 200) };
            append_span(job, &text[start..byte_end(i)], color, false, font_size);
            continue;
        }

        // 6. Numbers
        if ch.is_ascii_digit() {
            let start = start_byte;
            while i < n && chars[i].1.is_ascii_digit() {
                i += 1;
            }
            let num_color = if t.dark { Color32::from_rgb(190, 155, 255) } else { Color32::from_rgb(120, 75, 200) };
            append_span(job, &text[start..byte_end(i)], num_color, false, font_size);
            continue;
        }

        // 7. Words / Commands
        if ch.is_alphabetic() || ch == '_' {
            let start = start_byte;
            while i < n && (chars[i].1.is_alphanumeric() || chars[i].1 == '_') {
                i += 1;
            }
            let word = &text[start..byte_end(i)];
            let lower = word.to_ascii_lowercase();
            if CMD_KEYWORDS.contains(&lower.as_str()) {
                append_span(job, word, t.ink_strong, true, font_size);
            } else {
                append_span(job, word, t.ink, false, font_size);
            }
            continue;
        }

        // 8. Operators
        if "&|><^=()[]".contains(ch) {
            append_span(job, &text[start_byte..byte_end(i + 1)], t.faint, false, font_size);
            i += 1;
            continue;
        }

        append_span(job, &text[start_byte..byte_end(i + 1)], t.ink, false, font_size);
        i += 1;
    }
}

const LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function",
    "goto", "if", "in", "local", "nil", "not", "or", "repeat", "return", "then",
    "true", "until", "while",
];

const LUA_GLOBALS: &[&str] = &[
    "ph", "log", "run_cmd", "get", "set", "print", "assert", "error", "ipairs",
    "pairs", "pcall", "xpcall", "select", "tonumber", "tostring", "type", "string",
    "table", "math", "io", "os", "coroutine", "package", "utf8", "debug",
];

fn highlight_lua(text: &str, t: &Tokens, font_size: f32, job: &mut LayoutJob) {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let n = chars.len();
    let mut i = 0;

    let byte_end = |idx: usize| -> usize {
        if idx < n {
            chars[idx].0
        } else {
            text.len()
        }
    };

    while i < n {
        let (start_byte, ch) = chars[i];

        // 1. Block comment --[[ ... ]] or line comment -- ...
        if ch == '-' && i + 1 < n && chars[i + 1].1 == '-' {
            let start = start_byte;
            i += 2;
            if i + 1 < n && chars[i].1 == '[' && chars[i + 1].1 == '[' {
                i += 2;
                while i < n {
                    if chars[i].1 == ']' && i + 1 < n && chars[i + 1].1 == ']' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            } else {
                while i < n && chars[i].1 != '\n' {
                    i += 1;
                }
            }
            append_span(job, &text[start..byte_end(i)], t.muted.gamma_multiply(0.7), false, font_size);
            continue;
        }

        // 2. Multiline raw string literal [[ ... ]]
        if ch == '[' && i + 1 < n && chars[i + 1].1 == '[' {
            let start = start_byte;
            i += 2;
            while i < n {
                if chars[i].1 == ']' && i + 1 < n && chars[i + 1].1 == ']' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            let str_color = if t.dark { Color32::from_rgb(165, 214, 167) } else { Color32::from_rgb(46, 125, 50) };
            append_span(job, &text[start..byte_end(i)], str_color, false, font_size);
            continue;
        }

        // 3. String literal "..." or '...'
        if ch == '"' || ch == '\'' {
            let quote = ch;
            let start = start_byte;
            i += 1;
            while i < n {
                let cur = chars[i].1;
                if cur == '\\' && i + 1 < n {
                    i += 2;
                    continue;
                }
                if cur == quote {
                    i += 1;
                    break;
                }
                if cur == '\n' {
                    break;
                }
                i += 1;
            }
            let str_color = if t.dark { Color32::from_rgb(165, 214, 167) } else { Color32::from_rgb(46, 125, 50) };
            append_span(job, &text[start..byte_end(i)], str_color, false, font_size);
            continue;
        }

        // 4. Numbers (hex 0x... or decimal / float)
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < n && chars[i + 1].1.is_ascii_digit()) {
            let start = start_byte;
            if ch == '0' && i + 1 < n && (chars[i + 1].1 == 'x' || chars[i + 1].1 == 'X') {
                i += 2;
                while i < n && chars[i].1.is_ascii_hexdigit() {
                    i += 1;
                }
            } else {
                while i < n && (chars[i].1.is_ascii_digit() || chars[i].1 == '.' || chars[i].1 == 'e' || chars[i].1 == 'E') {
                    i += 1;
                }
            }
            let num_color = if t.dark { Color32::from_rgb(255, 183, 77) } else { Color32::from_rgb(230, 81, 0) };
            append_span(job, &text[start..byte_end(i)], num_color, false, font_size);
            continue;
        }

        // 5. Identifiers (Keywords, globals, or normal identifiers)
        if ch.is_alphabetic() || ch == '_' {
            let start = start_byte;
            while i < n && (chars[i].1.is_alphanumeric() || chars[i].1 == '_') {
                i += 1;
            }
            let word = &text[start..byte_end(i)];
            if LUA_KEYWORDS.contains(&word) {
                append_span(job, word, t.accent_ink, true, font_size);
            } else if LUA_GLOBALS.contains(&word) {
                let glob_color = if t.dark { Color32::from_rgb(129, 212, 250) } else { Color32::from_rgb(2, 119, 189) };
                append_span(job, word, glob_color, false, font_size);
            } else {
                append_span(job, word, t.ink, false, font_size);
            }
            continue;
        }

        // 6. Operators & punctuation
        if "+-*/%^#=~<>;:,.{}()[]".contains(ch) {
            let start = start_byte;
            append_span(job, &text[start..byte_end(i + 1)], t.muted, false, font_size);
            i += 1;
            continue;
        }

        append_span(job, &text[start_byte..byte_end(i + 1)], t.ink, false, font_size);
        i += 1;
    }
}

pub fn highlight_script(
    text: &str,
    lang: ScriptLanguage,
    t: &Tokens,
    font_size: f32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    match lang {
        ScriptLanguage::PowerShell => highlight_powershell(text, t, font_size, &mut job),
        ScriptLanguage::Batch => highlight_batch(text, t, font_size, &mut job),
        ScriptLanguage::Lua => highlight_lua(text, t, font_size, &mut job),
    }
    job
}

pub fn create_script_layouter(
    lang: ScriptLanguage,
    t: Tokens,
    font_size: f32,
) -> impl FnMut(&egui::Ui, &dyn egui::text_edit::TextBuffer, f32) -> std::sync::Arc<egui::Galley> {
    move |ui: &egui::Ui, text: &dyn egui::text_edit::TextBuffer, wrap_width: f32| {
        let mut job = highlight_script(text.as_str(), lang, &t, font_size);
        job.wrap.max_width = wrap_width;
        ui.painter().layout_job(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_powershell_highlighter_preserves_text() {
        let t = Tokens::DARK;
        let script = "# Comment\n<# Block #>\n$proc = \"notepad.exe\"\nif ($true) {\n    Start-Process -FilePath $proc -Force\n}";
        let job = highlight_script(script, ScriptLanguage::PowerShell, &t, 12.0);
        assert!(!job.sections.is_empty());
        assert_eq!(job.text, script);
    }

    #[test]
    fn test_batch_highlighter_preserves_text() {
        let t = Tokens::DARK;
        let script = "@echo off\n:: comment\nrem comment\nset TARGET=app.exe\nif exist %TARGET% (\n    start /b \"\" \"%TARGET%\"\n)";
        let job = highlight_script(script, ScriptLanguage::Batch, &t, 12.0);
        assert!(!job.sections.is_empty());
        assert_eq!(job.text, script);
    }

    #[test]
    fn test_lua_highlighter_preserves_text() {
        let t = Tokens::DARK;
        let script = "-- Lua script\n--[[ block ]]\nlocal target = \"notepad.exe\"\nph.log(\"info\", \"Starting: \" .. target)\nif ph.get(\"status\") == nil then\n    ph.set(\"status\", 1)\nend";
        let job = highlight_script(script, ScriptLanguage::Lua, &t, 12.0);
        assert!(!job.sections.is_empty());
        assert_eq!(job.text, script);
    }
}

