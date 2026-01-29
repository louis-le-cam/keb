use colored::{Color, Colorize};

use crate::token::{Token, Tokens, token_length};

pub enum DiagnosticLevel {
    Error,
    Warning,
}

pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub spans: Vec<Span>,
    pub notes: Vec<String>,
}

pub enum SpanKind {
    Error,
    Warning,
    Note,
}

pub struct Span {
    pub kind: SpanKind,
    pub start: Token,
    pub end: Token,
    pub message: String,
}

struct ComputedSpan {
    pub width: usize,
    pub first_line: usize,
    pub first_column: usize,
    pub lines_start: usize,
    pub lines_end: usize,
    pub max_line_number_width: usize,
}

fn computed_span(source: &str, tokens: &Tokens, span: &Span) -> ComputedSpan {
    let start = tokens.offsets[span.start];
    let end = tokens.offsets[span.end] + token_length(source, tokens, span.end);

    let first_line = source[..start].lines().count();
    let line_count = source[start..end].lines().count();
    let last_line = first_line + line_count;
    let max_line_number_width = (last_line + 1).ilog10() as usize + 1;

    let first_column = source[..start]
        .lines()
        .next_back()
        .map(|line| line.chars().count())
        .unwrap_or(0);
    let last_column = source[..end]
        .lines()
        .next_back()
        .map(|line| line.chars().count())
        .unwrap_or(0);

    let width = last_column.saturating_sub(first_column).max(1);

    let lines_start = start - first_column;
    let lines_end = source[end..]
        .lines()
        .next()
        .map(|line| end + line.chars().count())
        .unwrap_or(source.len());

    ComputedSpan {
        width,
        first_line,
        first_column,
        lines_start,
        lines_end,
        max_line_number_width,
    }
}

// TODO: Display multiline spans in a decent way
pub fn print_diagnostic(source: &str, tokens: &Tokens, diagnostic: &Diagnostic) {
    let spans = diagnostic
        .spans
        .iter()
        .map(|span| computed_span(source, tokens, span))
        .collect::<Vec<_>>();

    let max_line_number_width = spans
        .iter()
        .map(|span| span.max_line_number_width)
        .max()
        .unwrap();

    let level = match diagnostic.level {
        DiagnosticLevel::Error => "error".bright_red().bold(),
        DiagnosticLevel::Warning => "warning".yellow().bold(),
    };

    println!("{level}{}{}", ": ".bold(), diagnostic.message.bold());
    println!(
        "{}{} input.keb:{}:{}",
        " ".repeat(max_line_number_width),
        "-->".bright_blue().bold(),
        spans[0].first_line + 1,
        spans[0].first_column + 1,
    );

    for (span, computed_span) in diagnostic.spans.iter().zip(spans) {
        let (symbol, color) = match span.kind {
            SpanKind::Error => ("^", Color::BrightRed),
            SpanKind::Warning => ("^", Color::Yellow),
            SpanKind::Note => ("-", Color::BrightBlue),
        };

        println!(
            "{} {}",
            " ".repeat(max_line_number_width),
            "|".bright_blue().bold(),
        );

        for (i, line) in source[computed_span.lines_start..computed_span.lines_end]
            .lines()
            .enumerate()
        {
            println!(
                "{: <width$} {} {}",
                (computed_span.first_line + i + 1)
                    .to_string()
                    .bright_blue()
                    .bold(),
                "|".bright_blue().bold(),
                line,
                width = max_line_number_width,
            )
        }

        println!(
            "{} {} {}{} {}",
            " ".repeat(max_line_number_width),
            "|".bright_blue().bold(),
            " ".repeat(computed_span.first_column),
            symbol.repeat(computed_span.width).color(color).bold(),
            span.message.color(color).bold(),
        );
    }

    if diagnostic.notes.is_empty() {
        println!(
            "{} {}",
            " ".repeat(max_line_number_width),
            "|".bright_blue().bold(),
        );

        for note in &diagnostic.notes {
            println!(
                "{} {} {} {}",
                " ".repeat(max_line_number_width),
                "=".bright_blue().bold(),
                "note:".bold(),
                note,
            );
        }
    }
}
