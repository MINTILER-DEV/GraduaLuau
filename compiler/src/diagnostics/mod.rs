use std::fmt::{Display, Formatter};

use crate::source::{FileId, SourceManager, SourceSpan};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_error())
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn render(&self, sources: &SourceManager) -> String {
        self.diagnostics
            .iter()
            .map(|diagnostic| diagnostic.render(sources))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn emit_to_stderr(&self) {
        for diagnostic in &self.diagnostics {
            eprintln!("{diagnostic}");
        }
    }

    pub fn emit_to_stderr_with_sources(&self, sources: &SourceManager) {
        eprint!("{}", self.render(sources));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    severity: Severity,
    message: String,
    labels: Vec<DiagnosticLabel>,
    notes: Vec<DiagnosticNote>,
    help: Option<String>,
    suggestion: Option<String>,
}

impl Diagnostic {
    pub fn builder(severity: Severity) -> DiagnosticBuilder {
        DiagnosticBuilder::new(severity)
    }

    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self::builder(severity).message(message).build()
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(Severity::Error, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, message)
    }

    pub fn note(message: impl Into<String>) -> Self {
        Self::new(Severity::Note, message)
    }

    pub fn internal_compiler_error(message: impl Into<String>) -> Self {
        Self::builder(Severity::InternalCompilerError)
            .message(message)
            .note("please report this issue")
            .build()
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.labels.push(DiagnosticLabel::primary(span, ""));
        self
    }

    pub fn with_label(mut self, span: SourceSpan, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel::primary(span, message));
        self
    }

    pub fn with_secondary_label(mut self, span: SourceSpan, message: impl Into<String>) -> Self {
        self.labels
            .push(DiagnosticLabel::secondary(span, message));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(DiagnosticNote::plain(note));
        self
    }

    pub fn with_note_at(mut self, span: SourceSpan, note: impl Into<String>) -> Self {
        self.notes.push(DiagnosticNote::with_span(span, note));
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn render(&self, sources: &SourceManager) -> String {
        let mut output = String::new();

        output.push_str(&format!("{}:\n", self.severity));
        output.push_str(&format!("{}\n", self.message));

        if let Some(primary_label) = self.primary_label() {
            render_source_label(&mut output, sources, primary_label);
        }

        for label in self.labels.iter().filter(|label| !label.is_primary()) {
            render_related_label(&mut output, sources, label);
        }

        for note in &self.notes {
            match note.span {
                Some(span) => {
                    output.push_str("note:\n");
                    output.push_str(&format!("{}\n", note.message));
                    render_related_span(&mut output, sources, span);
                }
                None => output.push_str(&format!("note: {}\n", note.message)),
            }
        }

        if let Some(help) = &self.help {
            output.push_str(&format!("help: {help}\n"));
        }

        if let Some(suggestion) = &self.suggestion {
            output.push_str(&format!("suggestion: {suggestion}\n"));
        }

        output
    }

    fn primary_label(&self) -> Option<&DiagnosticLabel> {
        self.labels.iter().find(|label| label.is_primary())
    }
}

impl Display for Diagnostic {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(formatter, "{}: {}", self.severity, self.message)?;

        for note in &self.notes {
            writeln!(formatter, "  note: {}", note.message)?;
        }

        if let Some(help) = &self.help {
            writeln!(formatter, "  help: {help}")?;
        }

        if let Some(suggestion) = &self.suggestion {
            writeln!(formatter, "  suggestion: {suggestion}")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticBuilder {
    severity: Severity,
    message: Option<String>,
    labels: Vec<DiagnosticLabel>,
    notes: Vec<DiagnosticNote>,
    help: Option<String>,
    suggestion: Option<String>,
}

impl DiagnosticBuilder {
    fn new(severity: Severity) -> Self {
        Self {
            severity,
            message: None,
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
            suggestion: None,
        }
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn span(mut self, span: SourceSpan) -> Self {
        self.labels.push(DiagnosticLabel::primary(span, ""));
        self
    }

    pub fn label(mut self, span: SourceSpan, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel::primary(span, message));
        self
    }

    pub fn secondary_label(mut self, span: SourceSpan, message: impl Into<String>) -> Self {
        self.labels
            .push(DiagnosticLabel::secondary(span, message));
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(DiagnosticNote::plain(note));
        self
    }

    pub fn note_at(mut self, span: SourceSpan, note: impl Into<String>) -> Self {
        self.notes.push(DiagnosticNote::with_span(span, note));
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn build(self) -> Diagnostic {
        Diagnostic {
            severity: self.severity,
            message: self.message.unwrap_or_else(|| String::from("diagnostic")),
            labels: self.labels,
            notes: self.notes,
            help: self.help,
            suggestion: self.suggestion,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    span: SourceSpan,
    message: String,
    style: LabelStyle,
}

impl DiagnosticLabel {
    pub fn primary(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Primary,
        }
    }

    pub fn secondary(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            style: LabelStyle::Secondary,
        }
    }

    fn is_primary(&self) -> bool {
        self.style == LabelStyle::Primary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiagnosticNote {
    span: Option<SourceSpan>,
    message: String,
}

impl DiagnosticNote {
    fn plain(message: impl Into<String>) -> Self {
        Self {
            span: None,
            message: message.into(),
        }
    }

    fn with_span(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            span: Some(span),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LabelStyle {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    InternalCompilerError,
}

impl Severity {
    pub fn is_error(self) -> bool {
        matches!(self, Self::Error | Self::InternalCompilerError)
    }
}

impl Display for Severity {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => formatter.write_str("error"),
            Self::Warning => formatter.write_str("warning"),
            Self::Note => formatter.write_str("note"),
            Self::InternalCompilerError => formatter.write_str("internal compiler error"),
        }
    }
}

fn render_source_label(output: &mut String, sources: &SourceManager, label: &DiagnosticLabel) {
    let Some(file) = sources.get(label.span.file_id()) else {
        render_missing_file(output, label.span.file_id());
        return;
    };

    let Some(location) = file.location(label.span.start()) else {
        output.push_str(" --> <invalid source location>\n");
        return;
    };

    let line_text = file.line_text(location.line).unwrap_or("");
    let line_number_width = location.line.to_string().len();
    let column = location.column.max(1);
    let underline_width = underline_width(label.span, file);

    output.push_str(&format!(
        "\n --> {}:{}:{}\n",
        file.path().display(),
        location.line,
        location.column
    ));
    output.push_str(&format!("{:>width$} |\n", "", width = line_number_width));
    output.push_str(&format!(
        "{:>width$} | {}\n",
        location.line,
        line_text,
        width = line_number_width
    ));
    output.push_str(&format!(
        "{:>width$} | {}{}",
        "",
        " ".repeat(column.saturating_sub(1)),
        "^".repeat(underline_width),
        width = line_number_width
    ));

    if !label.message.is_empty() {
        output.push(' ');
        output.push_str(&label.message);
    }

    output.push('\n');
}

fn render_related_label(output: &mut String, sources: &SourceManager, label: &DiagnosticLabel) {
    output.push_str("related:\n");
    if !label.message.is_empty() {
        output.push_str(&format!("{}\n", label.message));
    }
    render_related_span(output, sources, label.span);
}

fn render_related_span(output: &mut String, sources: &SourceManager, span: SourceSpan) {
    let Some(file) = sources.get(span.file_id()) else {
        render_missing_file(output, span.file_id());
        return;
    };

    match file.location(span.start()) {
        Some(location) => output.push_str(&format!(
            " --> {}:{}:{}\n",
            file.path().display(),
            location.line,
            location.column
        )),
        None => output.push_str(" --> <invalid source location>\n"),
    }
}

fn render_missing_file(output: &mut String, file_id: FileId) {
    output.push_str(&format!(" --> <unknown file {:?}>\n", file_id));
}

fn underline_width(span: SourceSpan, sources: &crate::source::SourceFile) -> usize {
    let Some(location) = sources.location(span.start()) else {
        return 1;
    };
    let line_end = sources.line_end(location.line).unwrap_or(span.end());
    let end = span.end().min(line_end).max(span.start() + 1);

    end.saturating_sub(span.start()).max(1)
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, DiagnosticBag, Severity};
    use crate::source::{SourceManager, SourceSpan};
    use std::path::PathBuf;

    #[test]
    fn renders_line_column_and_span() {
        let mut sources = SourceManager::new();
        let file = sources.add_file(
            PathBuf::from("src/main.glu"),
            String::from("local x =\nprint \"hi\"\n"),
        );
        let span = SourceSpan::new(file, 8, 9);

        let diagnostic = Diagnostic::builder(Severity::Error)
            .message("Expected expression after '='.")
            .label(span, "expected expression")
            .help("add an expression after '='")
            .build();

        assert_eq!(
            diagnostic.render(&sources),
            "error:\nExpected expression after '='.\n\n --> src/main.glu:1:9\n |\n1 | local x =\n |         ^ expected expression\nhelp: add an expression after '='\n"
        );
    }

    #[test]
    fn renders_notes_and_suggestions() {
        let mut sources = SourceManager::new();
        let file = sources.add_file(PathBuf::from("src/main.glu"), String::from("pritn(\"hi\")"));
        let span = SourceSpan::new(file, 0, 5);

        let diagnostic = Diagnostic::error("Unknown identifier 'pritn'.")
            .with_label(span, "unknown identifier")
            .with_note("built-in functions are available without require")
            .with_suggestion("did you mean 'print'?");

        assert!(diagnostic.render(&sources).contains("note: built-in functions"));
        assert!(
            diagnostic
                .render(&sources)
                .contains("suggestion: did you mean 'print'?")
        );
    }

    #[test]
    fn collects_multiple_diagnostics() {
        let mut bag = DiagnosticBag::new();
        bag.push(Diagnostic::error("first error"));
        bag.push(Diagnostic::warning("unused variable"));

        assert_eq!(bag.len(), 2);
        assert!(bag.has_errors());
    }

    #[test]
    fn distinguishes_internal_compiler_errors() {
        let diagnostic = Diagnostic::internal_compiler_error("Unexpected MIR node.");

        assert_eq!(diagnostic.severity(), Severity::InternalCompilerError);
        assert!(diagnostic.severity().is_error());
        assert!(diagnostic.to_string().contains("please report this issue"));
    }
}
