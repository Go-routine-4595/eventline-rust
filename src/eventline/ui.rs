use std::cell::RefCell;
use super::eventline;

use std::time::Duration;

use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect, Size},
    style::{Style, Stylize},
    text::{Text, Line, Span},
    widgets::{Block, List, ListDirection, Paragraph},
};

use crossterm::event::{Event as CEvent, KeyCode, poll};
use ratatui::layout::Alignment;

pub struct UI {
    terminal: RefCell<DefaultTerminal>,
    term_size: Size,
}

impl UI {
    pub fn new() -> Self {
        let term = ratatui::init();
        let size = term.size();
        return UI{
            terminal: RefCell::new(term),
            term_size: size.unwrap(),
        }
    }

    pub fn render(&self, app: &eventline::EventLine) {
        self.terminal.borrow_mut().draw(|frame| render(frame, app));
    }
}

pub fn process_keypress() -> Result<bool, String> {
    if let Some(key_code) = check_for_keypress()? {
        match key_code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(true);
            }
            _ => return Ok(false), // Ignore other keys
        }
    }
    return Ok(false);
}
fn check_for_keypress() -> Result<Option<KeyCode>, String> {
    if poll(Duration::from_millis(100)).map_err(|e| format!("error: {}", e))? {
        match crossterm::event::read().map_err(|e| format!("error: {}", e))? {
            CEvent::Key(key) => Ok(Some(key.code)),
            _ => Ok(None),
        }
    } else {
        // No event within timeout - this is normal behavior
        Ok(None)
    }
}

pub fn render(frame: &mut Frame, app: &eventline::EventLine) {

    let l_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(6), Constraint::Min(0)])
        .split(frame.area());

    draw_list(frame, app, l_layout[1]);
    draw_title3(frame, app, l_layout[0]);
}

fn draw_list(frame: &mut Frame, app: &eventline::EventLine, area: Rect) {
    let list = List::new(app.data_list().clone())
        .block(Block::bordered().title("Events"))
        .style(Style::new().white())
        .highlight_style(Style::new().italic())
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .direction(ListDirection::TopToBottom);

    frame.render_widget(list, area);
}

fn draw_title3(frame: &mut Frame, app: &eventline::EventLine, area: Rect) {
    let title = app.title();
    let text = vec![
        Line::from(vec![
            Span::styled("Current time", Style::new().green().italic()),
            Span::raw(" ".repeat(5)),
            Span::raw(chrono::Local::now().with_timezone(&chrono::Local).to_rfc3339()),
        ]),
        Line::from( vec![
            Span::styled("Global Counter: ", Style::new().green().italic()),
            Span::raw(format!("{:>5}", app.global_counter())),
            Span::raw(" ".repeat(5)),
            Span::styled("Local Counter: ", Style::new().green().italic()),
            Span::raw(format!("{:>5}", app.events_map_size())),
        ]),
        Line::from( vec![
            Span::styled("Last update:", Style::new().green().italic()),
            Span::raw(" ".repeat(5)),
            Span::raw(format!("{}", app.last_update())),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(text)
            .block(Block::bordered().title(title))
            .style(Style::new().white().on_black()),
        area,
    )
}

fn draw_title(frame: &mut Frame, app: &mut eventline::EventLine, area: Rect) {
    let title = app.title();
    frame.render_widget(
        Paragraph::new(Text::from(title))
            .block(Block::bordered().title(title))
            .style(Style::new().white().on_black()),
        area,
    )
}
fn validate_terminal_size(size: Size) -> Result<Size, String> {
    const MIN_HEIGHT: u16 = 15;
    const MIN_WIDTH: u16 = 80;

    if size.height < MIN_HEIGHT || size.width < MIN_WIDTH {
        return Err(String::from("terminal too small"));
    }

    Ok(size)
}
