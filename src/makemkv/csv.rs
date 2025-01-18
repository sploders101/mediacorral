use std::{iter::Peekable, str::Chars};

pub struct CsvRowIter<'a> {
    chars: Peekable<Chars<'a>>,
    finished: bool,
}
impl<'a> CsvRowIter<'a> {
    pub fn new(row: &'a str) -> Self {
        return Self {
            chars: row.chars().peekable(),
            finished: false,
        };
    }
}
impl<'a> Iterator for CsvRowIter<'a> {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // We can assume this is the start of the cell.
        let mut cell = String::new();
        let quoted_cell = self.chars.next_if_eq(&'"').is_some();
        while let Some(char) = self.chars.next() {
            match char {
                '"' if quoted_cell && self.chars.next_if_eq(&'"').is_some() => {
                    // Escaped quote
                    cell.push('"');
                }
                '"' if quoted_cell && self.chars.next_if_eq(&',').is_some() => {
                    // End of cell
                    return Some(cell);
                }
                '"' if quoted_cell && self.chars.peek().is_none() => {
                    // End of cell (and row)
                    self.finished = true;
                    return Some(cell);
                }
                '"' => {
                    // Undefined behavior. Lets just record it as a literal.
                    cell.push('"');
                }
                ',' if !quoted_cell => {
                    // End of cell
                    return Some(cell);
                }
                this_char => {
                    cell.push(this_char);
                }
            }
        }
        self.finished = true;
        return Some(cell);
    }
}

#[cfg(test)]
mod tests {
    use futures::stream;

    use super::*;

    #[tokio::test]
    async fn test_csv_parser() {
        let row: Vec<String> = CsvRowIter::new("\"cell1\",cell2,,cell4,\"cell\"\"5").collect();
        assert_eq!(row, vec!["cell1", "cell2", "", "cell4", "cell\"5"]);
    }
}
