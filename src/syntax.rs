use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// Lazy-loaded syntax set
fn get_syntax_set() -> &'static SyntaxSet {
    use std::sync::OnceLock;
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Lazy-loaded theme
fn get_theme() -> &'static Theme {
    use std::sync::OnceLock;
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        let theme_set = ThemeSet::load_defaults();
        // Use a dark theme that works well in terminals
        theme_set.themes["base16-ocean.dark"].clone()
    })
}

/// Extracts the file extension from a filename
fn get_extension(filename: &str) -> Option<&str> {
    Path::new(filename).extension()?.to_str()
}

/// Gets the syntax reference for a filename
fn get_syntax_for_file(filename: &str) -> &'static SyntaxReference {
    let syntax_set = get_syntax_set();

    // Try to find syntax by extension
    if let Some(extension) = get_extension(filename) {
        if let Some(syntax) = syntax_set.find_syntax_by_extension(extension) {
            return syntax;
        }
    }

    // Try to find syntax by filename (for files like Makefile, Dockerfile, etc.)
    if let Some(syntax) = syntax_set.find_syntax_by_first_line(filename) {
        return syntax;
    }

    // Fallback to plain text
    syntax_set.find_syntax_plain_text()
}

/// Converts syntect color to ratatui color
fn syntect_to_ratatui_color(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// Highlights diff content with syntax highlighting
/// Returns a vector of ratatui Lines with both syntax and diff coloring
pub fn highlight_diff(diff_content: &str, filename: &str) -> Vec<Line<'static>> {
    let syntax = get_syntax_for_file(filename);
    let theme = get_theme();
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut result_lines = Vec::new();

    for line in diff_content.lines() {
        let highlighted_line = if line.starts_with("@@") {
            // Hunk header - show in cyan
            Line::from(Span::styled(line.to_string(), Style::default().fg(Color::Cyan)))
        } else if let Some(code) = line.strip_prefix('+') {
            // Addition - apply syntax highlighting then overlay green
            highlight_line_with_diff_marker(code, &mut highlighter, '+')
        } else if let Some(code) = line.strip_prefix('-') {
            // Deletion - apply syntax highlighting then overlay red
            highlight_line_with_diff_marker(code, &mut highlighter, '-')
        } else if line.starts_with(' ') || line.is_empty() {
            // Context line - apply syntax highlighting
            let code = line.strip_prefix(' ').unwrap_or("");
            highlight_line_with_diff_marker(code, &mut highlighter, ' ')
        } else {
            // Other metadata (shouldn't happen with our parser, but handle it)
            Line::from(Span::styled(line.to_string(), Style::default().fg(Color::Gray)))
        };

        result_lines.push(highlighted_line);
    }

    result_lines
}

/// Highlights a single line and applies diff marker color
fn highlight_line_with_diff_marker(
    code: &str,
    highlighter: &mut HighlightLines,
    marker: char,
) -> Line<'static> {
    let syntax_set = get_syntax_set();

    // Highlight the code
    let highlighted = highlighter
        .highlight_line(code, syntax_set)
        .unwrap_or_else(|_| vec![]);

    let mut spans = Vec::new();

    // Add the diff marker first with appropriate background
    let marker_style = match marker {
        '+' => Style::default().fg(Color::Black).bg(Color::Green),
        '-' => Style::default().fg(Color::Black).bg(Color::Red),
        _ => Style::default(),
    };
    spans.push(Span::styled(marker.to_string(), marker_style));

    // Add syntax-highlighted code with background highlight for additions/deletions
    for (style, text) in highlighted {
        let fg_color = syntect_to_ratatui_color(style.foreground);

        // Use background color for diff highlighting, preserve syntax foreground colors
        let final_style = match marker {
            '+' => Style::default().fg(fg_color).bg(Color::Rgb(0, 64, 0)), // Dark green bg
            '-' => Style::default().fg(fg_color).bg(Color::Rgb(64, 0, 0)), // Dark red bg
            _ => Style::default().fg(fg_color),
        };

        spans.push(Span::styled(text.to_string(), final_style));
    }

    Line::from(spans)
}
