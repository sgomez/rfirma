//! La lista de `-certtui`: filtra al teclear, se desplaza y pinta cada certificado en cuatro líneas; no sabe de dónde vienen las teclas ni adónde va lo pintado.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{HighlightSpacing, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::desktop::ports::OfferedCertificate;

const PROMPT: &str = "Certificado: ";
const HELP: &str = "   ↑ ↓ para moverse · Intro elige · Esc cancela";
const LINES_PER_CERTIFICATE: usize = 4;
const CERTIFICATES_IN_SIGHT: usize = 4;

/// Lo que queda tras una tecla.
#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    Pending,
    Chosen(usize),
    Cancelled,
}

/// Las líneas que ocupa la lista bajo el prompt, con el prompt incluido.
pub fn height_for(certificates: usize) -> u16 {
    let rows = 1 + LINES_PER_CERTIFICATE * certificates.clamp(1, CERTIFICATES_IN_SIGHT);
    u16::try_from(rows).unwrap_or(u16::MAX)
}

/// El estado de la lista: lo tecleado, lo que deja ver y cuál está marcado.
pub struct Picker<'a> {
    offered: &'a [OfferedCertificate],
    searchable: Vec<String>,
    query: String,
    shown: Vec<usize>,
    state: ListState,
}

impl<'a> Picker<'a> {
    pub fn new(offered: &'a [OfferedCertificate], preselected: usize) -> Self {
        let mut picker = Self {
            offered,
            searchable: offered.iter().map(searchable_text_of).collect(),
            query: String::new(),
            shown: (0..offered.len()).collect(),
            state: ListState::default(),
        };
        let last = offered.len().saturating_sub(1);
        picker
            .state
            .select((!offered.is_empty()).then_some(preselected.min(last)));
        picker
    }

    pub fn on(&mut self, key: KeyEvent) -> Step {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => return Step::Cancelled,
            KeyCode::Char('c' | 'd') if control => return Step::Cancelled,
            KeyCode::Enter => {
                if let Some(chosen) = self.marked() {
                    return Step::Chosen(chosen);
                }
            }
            KeyCode::Up => self.state.select_previous(),
            KeyCode::Down => self.moved_down(),
            KeyCode::Backspace => {
                self.query.pop();
                self.refiltered();
            }
            KeyCode::Char(character) if !control => {
                self.query.push(character);
                self.refiltered();
            }
            _ => {}
        }
        Step::Pending
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let [prompt, list] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(frame.area());
        let typed = Line::from(vec![
            Span::raw(PROMPT),
            Span::raw(self.query.as_str()),
            Span::styled(HELP, Style::new().add_modifier(Modifier::DIM)),
        ]);
        frame.render_widget(Paragraph::new(typed), prompt);
        let column = Line::from(format!("{PROMPT}{}", self.query)).width();
        frame.set_cursor_position(Position::new(
            prompt.x + u16::try_from(column).unwrap_or(u16::MAX),
            prompt.y,
        ));
        if self.shown.is_empty() {
            let nothing = format!("  Ningún certificado coincide con «{}»", self.query);
            frame.render_widget(Paragraph::new(nothing), list);
            return;
        }
        let items: Vec<ListItem> = self
            .shown
            .iter()
            .map(|&index| item_of(&self.offered[index]))
            .collect();
        let rows = List::new(items)
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(rows, list, &mut self.state);
    }

    fn marked(&self) -> Option<usize> {
        self.state
            .selected()
            .and_then(|position| self.shown.get(position).copied())
    }

    fn moved_down(&mut self) {
        let last = self.shown.len().saturating_sub(1);
        let next = self
            .state
            .selected()
            .map_or(0, |position| (position + 1).min(last));
        self.state.select((!self.shown.is_empty()).then_some(next));
    }

    fn refiltered(&mut self) {
        let marked = self.marked();
        let words: Vec<String> = folded(&self.query)
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        self.shown = (0..self.offered.len())
            .filter(|&index| {
                words
                    .iter()
                    .all(|word| self.searchable[index].contains(word.as_str()))
            })
            .collect();
        let position = marked
            .and_then(|marked| self.shown.iter().position(|&index| index == marked))
            .unwrap_or(0);
        self.state
            .select((!self.shown.is_empty()).then_some(position));
    }
}

fn item_of(certificate: &OfferedCertificate) -> ListItem<'static> {
    ListItem::new(Text::from(vec![
        Line::from(certificate.headline.clone()).style(Style::new().add_modifier(Modifier::BOLD)),
        Line::from(format!("  {}", certificate.capacity)),
        Line::from(format!(
            "  {} · caduca el {}",
            certificate.issuer, certificate.expires
        )),
        Line::from(format!("  En {}", certificate.stores.join(", "))),
    ]))
}

fn searchable_text_of(certificate: &OfferedCertificate) -> String {
    folded(&format!(
        "{} {} {} {} {}",
        certificate.headline,
        certificate.capacity,
        certificate.issuer,
        certificate.expires,
        certificate.stores.join(" ")
    ))
}

/// En minúsculas y sin tildes, para que «perez» encuentre a «PÉREZ».
fn folded(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|character| match character {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests;
