//! A format-neutral table grid and its Markdown rendering: a pipe table when the grid is rectangular enough,
//! otherwise an HTML fence that keeps the spans.

use crate::builder::{MdBuilder, collapse_ws};

#[derive(Debug, Clone)]
pub(crate) struct Cell {
    pub text: String,
    pub colspan: usize,
    pub rowspan: usize,
}

impl Cell {
    pub fn new(text: impl Into<String>, colspan: usize, rowspan: usize) -> Self {
        Self { text: collapse_ws(&text.into()), colspan: colspan.max(1), rowspan: rowspan.max(1) }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Grid {
    pub rows: Vec<Vec<Cell>>,
    /// Rows at the top that are header rows.
    pub header_rows: usize,
}

/// Spans beyond this are treated as a data error rather than allocated.
const MAX_SPAN: usize = 1000;

impl Grid {
    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(|r| r.iter().all(|c| c.text.is_empty())) || self.rows.is_empty()
    }

    /// Whether a data row has a span (a pipe table cannot say which cells are merged).
    fn has_body_spans(&self) -> bool {
        self.rows.iter().skip(self.header_rows).any(|r| r.iter().any(|c| c.colspan > 1 || c.rowspan > 1))
    }

    /// Rows with spans expanded: a colspan adds empty cells to the right, a rowspan adds empty cells below.
    pub fn expand(&self) -> Vec<Vec<String>> {
        let mut carry: Vec<usize> = Vec::new();
        let mut out = Vec::with_capacity(self.rows.len());
        for row in &self.rows {
            let mut cells: Vec<String> = Vec::new();
            let mut col = 0usize;
            let take_carried = |cells: &mut Vec<String>, col: &mut usize, carry: &mut Vec<usize>| {
                while *col < carry.len() && carry[*col] > 0 {
                    carry[*col] -= 1;
                    cells.push(String::new());
                    *col += 1;
                }
            };
            for cell in row {
                take_carried(&mut cells, &mut col, &mut carry);
                let colspan = cell.colspan.min(MAX_SPAN);
                if carry.len() < col + colspan {
                    carry.resize(col + colspan, 0);
                }
                for k in 0..colspan {
                    cells.push(if k == 0 { cell.text.clone() } else { String::new() });
                    if cell.rowspan > 1 {
                        carry[col + k] = cell.rowspan.min(MAX_SPAN) - 1;
                    }
                }
                col += colspan;
            }
            take_carried(&mut cells, &mut col, &mut carry);
            out.push(cells);
        }
        out
    }

    pub fn emit(&self, b: &mut MdBuilder) {
        if self.is_empty() {
            return;
        }
        if self.has_body_spans() {
            b.html_table(self.html_lines());
        } else {
            b.pipe_table(&self.expand(), self.header_rows);
        }
    }

    fn html_lines(&self) -> Vec<String> {
        let mut lines = vec!["<table>".to_string()];
        for (i, row) in self.rows.iter().enumerate() {
            let tag = if i < self.header_rows.max(1) && self.header_rows > 0 { "th" } else { "td" };
            let mut s = String::from("<tr>");
            for c in row {
                s.push('<');
                s.push_str(tag);
                if c.colspan > 1 {
                    s.push_str(&format!(" colspan=\"{}\"", c.colspan));
                }
                if c.rowspan > 1 {
                    s.push_str(&format!(" rowspan=\"{}\"", c.rowspan));
                }
                s.push('>');
                s.push_str(&html_escape(&c.text));
                s.push_str("</");
                s.push_str(tag);
                s.push('>');
            }
            s.push_str("</tr>");
            lines.push(s);
        }
        lines.push("</table>".to_string());
        lines
    }
}

pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(t: &str) -> Cell {
        Cell::new(t, 1, 1)
    }

    #[test]
    fn colspan_in_header_is_expanded_into_a_pipe_table() {
        let g = Grid { rows: vec![vec![Cell::new("Group", 2, 1), c("N")], vec![c("a"), c("b"), c("3")]], header_rows: 1 };
        let mut b = MdBuilder::new();
        g.emit(&mut b);
        assert_eq!(b.finish().markdown, "| Group |  | N |\n| --- | --- | --- |\n| a | b | 3 |\n");
    }

    #[test]
    fn rowspan_pushes_empty_cells_down() {
        let g = Grid {
            rows: vec![vec![c("h1"), c("h2")], vec![Cell::new("x", 1, 2), c("1")], vec![c("2")]],
            header_rows: 1,
        };
        assert_eq!(g.expand(), vec![vec!["h1", "h2"], vec!["x", "1"], vec!["", "2"]]);
        // Body spans fall back to HTML so the merge is not lost.
        let mut b = MdBuilder::new();
        g.emit(&mut b);
        let f = b.finish();
        assert!(f.markdown.starts_with("```html\n<table>\n<tr><th>h1</th><th>h2</th></tr>\n<tr><td rowspan=\"2\">x</td><td>1</td></tr>"), "{}", f.markdown);
        assert_eq!(f.blocks.len(), 1);
    }

    #[test]
    fn rows_are_padded_and_cells_are_escaped() {
        let g = Grid { rows: vec![vec![c("a|b"), c("c")], vec![c("only")]], header_rows: 0 };
        let mut b = MdBuilder::new();
        g.emit(&mut b);
        assert_eq!(b.finish().markdown, "| a\\|b | c |\n| --- | --- |\n| only |  |\n");
    }

    #[test]
    fn empty_grid_emits_nothing() {
        let mut b = MdBuilder::new();
        Grid { rows: vec![vec![c(""), c("")]], header_rows: 1 }.emit(&mut b);
        assert!(b.finish().markdown.is_empty());
    }

    #[test]
    fn absurd_spans_are_capped() {
        let g = Grid { rows: vec![vec![Cell::new("x", usize::MAX, 1)]], header_rows: 1 };
        assert_eq!(g.expand()[0].len(), MAX_SPAN);
    }
}
