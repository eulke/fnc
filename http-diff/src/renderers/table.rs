use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement,
    Table, TableComponent,
};

/// Builder for creating consistently styled tables across the application
#[derive(Clone)]
pub struct TableBuilder {
    table: Table,
}

/// Table styling presets for different use cases
#[derive(Debug, Clone, PartialEq)]
pub enum TableStyle {
    /// Clean diff table with minimal borders
    Diff,
}

impl TableBuilder {
    /// Create a new table builder with default styling
    pub fn new() -> Self {
        let mut table = Table::new();
        
        // Apply base configuration that works well in terminals
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic);
            
        Self { table }
    }

    /// Create a table with a specific style preset
    pub fn with_style(style: TableStyle) -> Self {
        let mut builder = Self::new();
        builder.apply_style(style);
        builder
    }

    /// Apply a style preset to the table
    pub fn apply_style(&mut self, style: TableStyle) -> &mut Self {
        match style {
            TableStyle::Diff => {
                self.table
                    .remove_style(TableComponent::HorizontalLines)
                    .remove_style(TableComponent::LeftBorderIntersections)
                    .remove_style(TableComponent::RightBorderIntersections)
                    .remove_style(TableComponent::MiddleIntersections);
            }
        }
        self
    }

    /// Set table headers with optional styling
    pub fn headers<I, S>(&mut self, headers: I) -> &mut Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let header_cells: Vec<Cell> = headers
            .into_iter()
            .map(|h| Cell::new(h.into()).add_attribute(Attribute::Bold))
            .collect();
        
        self.table.set_header(header_cells);
        self
    }

    /// Add a row to the table
    pub fn row<I, S>(&mut self, cells: I) -> &mut Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let row_cells: Vec<Cell> = cells
            .into_iter()
            .map(|cell| Cell::new(cell.into()))
            .collect();
        
        self.table.add_row(row_cells);
        self
    }

    /// Add a row with custom styled cells
    pub fn styled_row(&mut self, cells: Vec<Cell>) -> &mut Self {
        self.table.add_row(cells);
        self
    }

    /// Add a single line of text (useful for diffs)
    pub fn line<S: Into<String>>(&mut self, text: S) -> &mut Self {
        let cell = Cell::new(text.into());
        self.table.add_row(vec![cell]);
        self
    }

    /// Add multiple lines as individual rows (preserving formatting)
    pub fn lines<I, S>(&mut self, lines: I) -> &mut Self 
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for line in lines {
            self.line(line);
        }
        self
    }

    /// Build and return the formatted table as a string
    pub fn build(self) -> String {
        self.table.to_string()
    }

    /// Get a mutable reference to the underlying table for advanced customization
    pub fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }
}

impl Default for TableBuilder {
    fn default() -> Self {
        Self::new()
    }
}


/// Helper functions for creating styled cells
pub mod cells {
    use super::*;

    /// Create a bold cell
    pub fn bold<S: Into<String>>(text: S) -> Cell {
        Cell::new(text.into()).add_attribute(Attribute::Bold)
    }


    /// Create an error cell (red background)
    pub fn error<S: Into<String>>(text: S) -> Cell {
        Cell::new(text.into())
            .fg(Color::White)
            .bg(Color::Red)
    }

    /// Create a success cell (green background)
    pub fn success<S: Into<String>>(text: S) -> Cell {
        Cell::new(text.into())
            .fg(Color::White)
            .bg(Color::Green)
    }



    /// Create a muted/dimmed cell (gray text)
    pub fn muted<S: Into<String>>(text: S) -> Cell {
        Cell::new(text.into()).fg(Color::DarkGrey)
    }

    /// Create a cell for added diff lines (green text, no background)
    pub fn added<S: Into<String>>(text: S) -> Cell {
        Cell::new(text.into()).fg(Color::Green)
    }

    /// Create a cell for removed diff lines (red text, no background)
    pub fn removed<S: Into<String>>(text: S) -> Cell {
        Cell::new(text.into()).fg(Color::Red)
    }

    /// Create a cell for normal text (default color, no styling)
    pub fn normal<S: Into<String>>(text: S) -> Cell {
        Cell::new(text.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_builder_basic() {
        let mut builder = TableBuilder::new();
        builder.headers(vec!["Name", "Value"]);
        builder.row(vec!["test", "123"]);
        builder.row(vec!["another", "456"]);
        let table = builder.build();

        assert!(!table.is_empty());
        assert!(table.contains("Name"));
        assert!(table.contains("Value"));
        assert!(table.contains("test"));
        assert!(table.contains("123"));
    }

    #[test]
    fn test_diff_table_style() {
        let mut builder = TableBuilder::with_style(TableStyle::Diff);
        builder.line("Added line");
        builder.line("Removed line");
        let table = builder.build();

        assert!(table.contains("Added line"));
        assert!(table.contains("Removed line"));
    }

    #[test]
    fn test_summary_table_style() {
        let mut builder = TableBuilder::with_style(TableStyle::Diff);
        builder.headers(vec!["Environment", "Status"]);
        builder.row(vec!["dev", "200"]);
        builder.row(vec!["prod", "404"]);
        let table = builder.build();

        assert!(table.contains("Environment"));
        assert!(table.contains("Status"));
        assert!(table.contains("dev"));
        assert!(table.contains("404"));
    }

    #[test]
    fn test_styled_cells() {
        let mut builder = TableBuilder::new();
        builder.headers(vec!["Type", "Status"]);
        builder.styled_row(vec![
            cells::bold("Error"),
            cells::error("Failed")
        ]);
        builder.styled_row(vec![
            cells::bold("Success"),
            cells::success("OK")
        ]);
        let table = builder.build();

        assert!(table.contains("Error"));
        assert!(table.contains("Failed"));
        assert!(table.contains("Success"));
        assert!(table.contains("OK"));
    }

    #[test]
    fn test_multiple_lines() {
        let lines = vec![
            "Line 1",
            "Line 2", 
            "Line 3"
        ];

        let mut builder = TableBuilder::with_style(TableStyle::Diff);
        builder.lines(lines);
        let table = builder.build();

        assert!(table.contains("Line 1"));
        assert!(table.contains("Line 2"));
        assert!(table.contains("Line 3"));
    }
} 