const GAP: &str = "  ";
const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;

enum Align {
    Left,
    Right,
}

enum Width {
    Auto,
    Fixed(usize),
}

pub(crate) struct Column {
    align: Align,
    width: Width,
}

impl Column {
    pub(crate) fn left() -> Column {
        Column {
            align: Align::Left,
            width: Width::Auto,
        }
    }

    pub(crate) fn right() -> Column {
        Column {
            align: Align::Right,
            width: Width::Auto,
        }
    }

    pub(crate) fn fixed(self, n: usize) -> Column {
        Column {
            width: Width::Fixed(n),
            ..self
        }
    }
}

pub(crate) struct Table {
    columns: Vec<Column>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub(crate) fn new(columns: Vec<Column>) -> Table {
        Table {
            columns,
            rows: Vec::new(),
        }
    }

    pub(crate) fn rows(self, rows: impl IntoIterator<Item = Vec<String>>) -> Table {
        Table {
            rows: rows.into_iter().collect(),
            ..self
        }
    }

    pub(crate) fn render(&self) -> String {
        let widths = self.widths();
        self.rows
            .iter()
            .enumerate()
            .map(|(index, row)| self.render_row(index, row, &widths))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn widths(&self) -> Vec<usize> {
        self.columns
            .iter()
            .enumerate()
            .map(|(column, spec)| match spec.width {
                Width::Fixed(n) => n,
                Width::Auto => self
                    .rows
                    .iter()
                    .filter_map(|row| row.get(column))
                    .map(|cell| display_width(cell))
                    .max()
                    .unwrap_or(0),
            })
            .collect()
    }

    fn render_row(&self, index: usize, row: &[String], widths: &[usize]) -> String {
        assert_eq!(
            row.len(),
            self.columns.len(),
            "row {index} has {} cells, the layout has {}",
            row.len(),
            self.columns.len()
        );
        row.iter()
            .enumerate()
            .map(|(column, cell)| {
                if column + 1 == row.len() {
                    return cell.clone();
                }
                let padding = " ".repeat(widths[column].saturating_sub(display_width(cell)));
                match self.columns[column].align {
                    Align::Left => format!("{cell}{padding}"),
                    Align::Right => format!("{padding}{cell}"),
                }
            })
            .collect::<Vec<_>>()
            .join(GAP)
    }
}

pub(crate) fn strip_escapes(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != ESC {
            let c = s[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
            continue;
        }
        i = match bytes.get(i + 1) {
            Some(b']') => end_of_osc(bytes, i + 2),
            Some(b'[') => end_of_csi(bytes, i + 2),
            // An escape this grammar does not know is not a sequence: keep it.
            _ => {
                out.push('\u{1b}');
                i + 1
            }
        };
    }
    out
}

fn end_of_osc(bytes: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            BEL => return i + 1,
            ESC if bytes.get(i + 1) == Some(&b'\\') => return i + 2,
            _ => i += 1,
        }
    }
    bytes.len()
}

fn end_of_csi(bytes: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < bytes.len() {
        if (0x40..=0x7e).contains(&bytes[i]) {
            return i + 1;
        }
        i += 1;
    }
    bytes.len()
}

fn display_width(s: &str) -> usize {
    strip_escapes(s).chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_rows(columns: Vec<Column>, rows: &[&[&str]]) -> String {
        let rows: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.to_string())
                    .collect::<Vec<String>>()
            })
            .collect();
        Table::new(columns).rows(rows).render()
    }

    #[test]
    fn puts_two_spaces_between_every_adjacent_pair_of_columns() {
        let out = render_rows(
            vec![Column::right(), Column::left(), Column::right()],
            &[&["1234", "impressions", "5"], &["12", "likes", "0"]],
        );
        assert_eq!(out, "1234  impressions  5\n  12  likes        0");
    }

    #[test]
    fn right_aligns_a_numeric_column_so_its_widest_cell_defines_the_width() {
        let out = render_rows(
            vec![Column::right(), Column::left()],
            &[&["7", "likes"], &["1234", "impressions"]],
        );
        assert_eq!(out, "   7  likes\n1234  impressions");
    }

    #[test]
    fn left_aligns_and_pads_a_column_to_its_widest_cell() {
        let out = render_rows(
            vec![Column::left(), Column::left()],
            &[&["ab", "x"], &["longer", "y"]],
        );
        assert_eq!(out, "ab      x\nlonger  y");
    }

    #[test]
    fn pads_a_fixed_column_to_its_fixed_width_no_matter_what_the_other_rows_hold() {
        let out = render_rows(
            vec![Column::left().fixed(6), Column::right()],
            &[&["ab", "1"], &["abcdef", "22"]],
        );
        assert_eq!(out, "ab      1\nabcdef  22");
    }

    #[test]
    fn emits_a_cell_wider_than_its_fixed_width_whole() {
        let out = render_rows(
            vec![Column::left().fixed(4), Column::right()],
            &[&["abcdefgh", "1"]],
        );
        assert_eq!(out, "abcdefgh  1");
        assert!(!out.contains('…'));
    }

    #[test]
    fn measures_a_cell_by_visible_text() {
        let out = render_rows(
            vec![Column::left(), Column::right()],
            &[
                &["\x1b]8;;https://x.com/bobby\x1b\\@bobby\x1b]8;;\x1b\\", "1"],
                &["\x1b]8;;https://x.com/al\x1b\\@al\x1b]8;;\x1b\\", "2"],
            ],
        );
        assert_eq!(
            out,
            "\x1b]8;;https://x.com/bobby\x1b\\@bobby\x1b]8;;\x1b\\  1\n\x1b]8;;https://x.com/al\x1b\\@al\x1b]8;;\x1b\\     2"
        );
    }

    #[test]
    fn never_pads_the_last_cell() {
        let out = render_rows(
            vec![Column::left(), Column::left()],
            &[&["ab", "x"], &["cd", "yyy"]],
        );
        assert_eq!(out, "ab  x\ncd  yyy");
    }

    #[test]
    fn keeps_a_number_column_and_a_label_column_aligned_when_a_later_row_has_a_shorter_label() {
        let out = render_rows(
            vec![Column::right(), Column::left()],
            &[&["1234", "impressions"], &["5", "likes"]],
        );
        assert_eq!(out, "1234  impressions\n   5  likes");
    }

    #[test]
    fn renders_nothing_for_zero_rows() {
        let out = render_rows(vec![Column::right(), Column::left()], &[]);
        assert_eq!(out, "");
    }

    #[test]
    #[should_panic(expected = "row 0")]
    fn panics_when_a_row_arity_does_not_match_the_layout() {
        render_rows(vec![Column::right(), Column::left()], &[&["1234"]]);
    }

    #[test]
    fn strip_escapes_drops_osc8_and_csi() {
        let out = strip_escapes(
            "\x1b]8;;https://x.com/alice\x1b\\@alice\x1b]8;;\x1b\\\x1b[31m and \x1b[0mvisible",
        );
        assert_eq!(out, "@alice and visible");
    }
}
