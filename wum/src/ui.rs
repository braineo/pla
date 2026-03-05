use crate::types::MrState;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn draw_ui(f: &mut Frame, mr_list: &[MrState]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0)])
        .split(f.area());

    if mr_list.is_empty() {
        let empty_msg = Paragraph::new("No MRs found in tracking file. You can add more.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" wum - MR Watcher (Press 'q' to quit) ")
                    .borders(Borders::ALL),
            );
        f.render_widget(empty_msg, chunks[0]);
        return;
    }

    let mut items = Vec::new();
    for mr in mr_list {
        let elapsed = mr.completed_in.unwrap_or_else(|| mr.started_at.elapsed());
        let secs = elapsed.as_secs() % 60;
        let mins = (elapsed.as_secs() / 60) % 60;
        let hours = elapsed.as_secs() / 3600;
        let time_str = if hours > 0 {
            format!("{hours:02}:{mins:02}:{secs:02}")
        } else {
            format!("{mins:02}:{secs:02}")
        };

        let status_color = if mr.done {
            Color::Green
        } else if mr.status_text.contains("Error")
            || mr.status_text.contains("failed")
            || mr.status_text.contains("Manual")
        {
            Color::Red
        } else if mr.status_text.contains("Rebasing") {
            Color::Yellow
        } else {
            Color::Cyan
        };

        let state_line = Line::from(vec![
            Span::styled(
                format!("{} ", mr.status_text),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}!{}", mr.repo, mr.iid),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" <"),
            Span::styled(time_str, Style::default().fg(Color::DarkGray)),
            Span::raw(">\n"),
        ]);

        let title_line = Line::from(vec![Span::styled(
            &mr.title,
            Style::default().fg(Color::White),
        )]);

        let url_line = Line::from(vec![Span::styled(
            &mr.url,
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::UNDERLINED),
        )]);

        items.push(ListItem::new(vec![
            state_line,
            title_line,
            url_line,
            Line::raw(""),
        ]));
    }

    let list = List::new(items).block(
        Block::default()
            .title(" wum - MR Watcher (Press 'q' to quit) ")
            .borders(Borders::ALL),
    );

    f.render_widget(list, chunks[0]);
}
