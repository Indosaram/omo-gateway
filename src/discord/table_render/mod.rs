#[derive(Debug, Clone)]
pub struct RenderedTable {
    pub original_text: String,
    pub png_bytes: Vec<u8>,
    pub filename: String,
}

const MAX_TABLE_ROWS: usize = 300;
const MAX_TABLE_PIXELS: u64 = 32_000_000;

fn system_font_db() -> std::sync::Arc<usvg::fontdb::Database> {
    static FONT_DB: std::sync::OnceLock<std::sync::Arc<usvg::fontdb::Database>> =
        std::sync::OnceLock::new();
    FONT_DB
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            std::sync::Arc::new(db)
        })
        .clone()
}

/// Splits a table row body on unescaped `|` that sit outside backtick code
/// spans, and unescapes `\|` to `|`.
fn split_table_cells(inner: &str) -> Vec<String> {
    // With an odd number of backticks there is no well-formed code span, so
    // fall back to splitting on every unescaped pipe.
    let code_aware = inner.matches('`').count().is_multiple_of(2);
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut in_code = false;
    let mut chars = inner.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' if chars.peek() == Some(&'|') => {
                current.push('|');
                chars.next();
            }
            '`' if code_aware => {
                in_code = !in_code;
                current.push(ch);
            }
            '|' if !in_code => cells.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    cells.push(current);
    cells
}

fn parse_table_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = match inner.strip_suffix('|') {
        Some(stripped) if !stripped.ends_with('\\') => stripped,
        _ => inner,
    };
    split_table_cells(inner)
        .into_iter()
        .map(|col| col.trim().replace("<br>", "\n").replace("<br/>", "\n"))
        .collect()
}

fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return false;
    }
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner.split('|').all(|col| {
        let col = col.trim();
        !col.is_empty() && col.chars().all(|c| c == '-' || c == ':' || c == ' ')
    })
}

#[derive(Debug, Clone)]
pub struct MarkdownTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub raw: String,
}

pub fn extract_markdown_tables(text: &str) -> Vec<MarkdownTable> {
    extract_markdown_table_spans(text)
        .into_iter()
        .map(|(_, table)| table)
        .collect()
}

fn extract_markdown_table_spans(text: &str) -> Vec<(std::ops::Range<usize>, MarkdownTable)> {
    let mut tables = Vec::new();
    let mut offset = 0;
    let mut starts = Vec::new();
    let lines: Vec<&str> = text
        .split_inclusive('\n')
        .map(|line| {
            starts.push(offset);
            offset += line.len();
            line.trim_end_matches(['\r', '\n'])
        })
        .collect();
    let mut i = 0;
    let mut fence: Option<(char, usize)> = None;

    while i < lines.len() {
        let line = lines[i].trim();
        let marker = line.chars().next().unwrap_or(' ');
        let marker_len = line.chars().take_while(|&c| c == marker).count();
        if let Some((open, length)) = fence {
            if marker == open && marker_len >= length && line[marker_len..].trim().is_empty() {
                fence = None;
            }
            i += 1;
            continue;
        }
        if matches!(marker, '`' | '~') && marker_len >= 3 {
            fence = Some((marker, marker_len));
            i += 1;
            continue;
        }
        if line.starts_with('|') && line.ends_with('|') && i + 1 < lines.len() {
            let next_line = lines[i + 1].trim();
            if is_table_separator(next_line) {
                let headers = parse_table_row(line);
                let mut rows = Vec::new();
                let mut j = i + 2;

                while j < lines.len() {
                    let row_line = lines[j].trim();
                    if row_line.starts_with('|') && row_line.ends_with('|') {
                        rows.push(parse_table_row(row_line));
                        j += 1;
                    } else {
                        break;
                    }
                }

                if !rows.is_empty() {
                    let span = starts[i]..starts[j - 1] + lines[j - 1].len();
                    tables.push((
                        span.clone(),
                        MarkdownTable {
                            headers,
                            rows,
                            raw: text[span].to_string(),
                        },
                    ));
                    i = j;
                    continue;
                }
            }
        }
        i += 1;
    }

    tables
}

pub fn render_table_to_svg(table: &MarkdownTable) -> String {
    let font_family = "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Apple SD Gothic Neo', 'Noto Sans KR', sans-serif";
    let num_cols = table.headers.len().max(1);

    let cell_padding_x = 16.0;
    let cell_padding_y = 12.0;
    let line_height = 20.0;
    let font_size = 14.0;
    let header_font_size = 14.0;

    let mut max_chars_per_col = vec![0usize; num_cols];
    for (c, h) in table.headers.iter().enumerate() {
        if c < num_cols {
            let len = h.lines().map(|l| l.chars().count()).max().unwrap_or(0);
            max_chars_per_col[c] = max_chars_per_col[c].max(len);
        }
    }
    for row in &table.rows {
        for (c, cell) in row.iter().enumerate() {
            if c < num_cols {
                let len = cell.lines().map(|l| l.chars().count()).max().unwrap_or(0);
                max_chars_per_col[c] = max_chars_per_col[c].max(len);
            }
        }
    }

    let total_weights: usize = max_chars_per_col.iter().sum::<usize>().max(1);
    let target_total_width = 860.0f64;
    let col_widths: Vec<f64> = max_chars_per_col
        .iter()
        .map(|&w| {
            let ratio = (w as f64) / (total_weights as f64);
            (ratio * target_total_width).clamp(160.0, 480.0)
        })
        .collect();
    let actual_total_width: f64 = col_widths.iter().sum();

    let wrap_text = |text: &str, width: f64| -> Vec<String> {
        let mut wrapped = Vec::new();
        for paragraph in text.split('\n') {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }
            // A full em accommodates wide CJK glyphs as well as Latin text.
            let max_line_chars = ((width - cell_padding_x * 2.0) / font_size).max(1.0) as usize;
            let mut current = String::new();
            for word in paragraph.split_whitespace() {
                if current.chars().count() + word.chars().count() + 1 > max_line_chars
                    && !current.is_empty()
                {
                    wrapped.push(current.clone());
                    current.clear();
                }
                if !current.is_empty() {
                    current.push(' ');
                }
                for ch in word.chars() {
                    if current.chars().count() == max_line_chars {
                        wrapped.push(std::mem::take(&mut current));
                    }
                    current.push(ch);
                }
            }
            if !current.is_empty() {
                wrapped.push(current);
            }
        }
        if wrapped.is_empty() {
            wrapped.push(String::new());
        }
        wrapped
    };

    let header_lines: Vec<Vec<String>> = table
        .headers
        .iter()
        .enumerate()
        .map(|(c, h)| wrap_text(h, col_widths[c]))
        .collect();
    let header_height = (header_lines.iter().map(|l| l.len()).max().unwrap_or(1) as f64
        * line_height)
        + cell_padding_y * 2.0;

    let mut row_data = Vec::new();
    let mut total_height = header_height + 1.0;

    for row in &table.rows {
        let cells: Vec<Vec<String>> = (0..num_cols)
            .map(|c| {
                let text = row.get(c).map(String::as_str).unwrap_or("");
                wrap_text(text, col_widths[c])
            })
            .collect();
        let height = (cells.iter().map(|l| l.len()).max().unwrap_or(1) as f64 * line_height)
            + cell_padding_y * 2.0;
        total_height += height;
        row_data.push((cells, height));
    }

    let mut svg = String::new();
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" viewBox=\"0 0 {:.0} {:.0}\">\n",
        actual_total_width, total_height, actual_total_width, total_height
    ));
    svg.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#2b2d31\" rx=\"8\" ry=\"8\"/>\n");

    let mut curr_x = 0.0;
    for (c, &col_w) in col_widths.iter().enumerate() {
        svg.push_str(&format!(
            "<rect x=\"{:.1}\" y=\"0\" width=\"{:.1}\" height=\"{:.1}\" fill=\"#1e1f22\"/>\n",
            curr_x, col_w, header_height
        ));
        let lines = &header_lines[c];
        let mut curr_y = cell_padding_y + 14.0;
        for line in lines {
            svg.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"{}\" font-size=\"{:.1}\" font-weight=\"bold\" fill=\"#f2f3f5\">{}</text>\n",
                curr_x + cell_padding_x,
                curr_y,
                font_family,
                header_font_size,
                html_escape(line)
            ));
            curr_y += line_height;
        }
        curr_x += col_w;
    }

    svg.push_str(&format!(
        "<line x1=\"0\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#3f4147\" stroke-width=\"1\"/>\n",
        header_height, actual_total_width, header_height
    ));

    let mut curr_y = header_height;
    for (r, (cells, row_h)) in row_data.iter().enumerate() {
        let bg_color = if r % 2 == 0 { "#313338" } else { "#2b2d31" };
        let mut cell_x = 0.0;

        for (c, &col_w) in col_widths.iter().enumerate() {
            svg.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"{}\"/>\n",
                cell_x, curr_y, col_w, row_h, bg_color
            ));

            let lines = &cells[c];
            let mut text_y = curr_y + cell_padding_y + 14.0;
            for line in lines {
                svg.push_str(&format!(
                    "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"{}\" font-size=\"{:.1}\" fill=\"#dbdee1\">{}</text>\n",
                    cell_x + cell_padding_x,
                    text_y,
                    font_family,
                    font_size,
                    html_escape(line)
                ));
                text_y += line_height;
            }
            cell_x += col_w;
        }

        svg.push_str(&format!(
            "<line x1=\"0\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#383a40\" stroke-width=\"1\"/>\n",
            curr_y + row_h,
            actual_total_width,
            curr_y + row_h
        ));
        curr_y += row_h;
    }

    svg.push_str("</svg>");
    svg
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn svg_to_png(svg_str: &str, scale: f32) -> Result<Vec<u8>, String> {
    let opt = usvg::Options {
        font_family: "sans-serif".to_string(),
        fontdb: system_font_db(),
        ..Default::default()
    };

    let tree = usvg::Tree::from_str(svg_str, &opt).map_err(|e| e.to_string())?;

    let pixmap_size = tree
        .size()
        .to_int_size()
        .scale_by(scale)
        .ok_or_else(|| "invalid scale size".to_string())?;
    if u64::from(pixmap_size.width()) * u64::from(pixmap_size.height()) > MAX_TABLE_PIXELS {
        return Err("table image exceeds pixel budget".to_string());
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
        .ok_or_else(|| "failed to allocate pixmap".to_string())?;

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap.encode_png().map_err(|e| e.to_string())
}

pub fn transform_markdown_tables_to_images(content: &str) -> (String, Vec<RenderedTable>) {
    let tables = extract_markdown_table_spans(content);
    if tables.is_empty() {
        return (content.to_string(), Vec::new());
    }

    let mut rendered = Vec::new();
    let mut modified = String::new();
    let mut cursor = 0;

    for (idx, (span, table)) in tables.into_iter().enumerate() {
        // Keep the raw text when the table cannot be drawn faithfully.
        if table.rows.len() > MAX_TABLE_ROWS
            || table.rows.iter().any(|row| row.len() > table.headers.len())
        {
            tracing::warn!("Table too large or malformed for image rendering; keeping raw text");
            continue;
        }
        let svg = render_table_to_svg(&table);
        match svg_to_png(&svg, 2.0) {
            Ok(png_bytes) => {
                let filename = format!("table_{}.png", idx + 1);
                let placeholder = format!("\n*(📊 아래 첨부된 표 [{filename}] 참조)*\n");
                modified.push_str(&content[cursor..span.start]);
                modified.push_str(&placeholder);
                cursor = span.end;

                rendered.push(RenderedTable {
                    original_text: table.raw,
                    png_bytes,
                    filename,
                });
            }
            Err(err) => {
                tracing::warn!(%err, "Failed to rasterize table SVG to PNG; keeping raw text");
            }
        }
    }

    modified.push_str(&content[cursor..]);
    (modified, rendered)
}

/// Plain-text rendering of tables whose image upload failed, so the table
/// content is never lost: each original table in a fenced code block.
pub fn fallback_text(tables: &[RenderedTable]) -> String {
    tables
        .iter()
        .map(|table| {
            let longest_run = table
                .original_text
                .split(|c| c != '`')
                .map(str::len)
                .max()
                .unwrap_or(0);
            let fence = "`".repeat((longest_run + 1).max(3));
            format!("{fence}\n{}\n{fence}", table.original_text)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaped_pipes_and_code_span_pipes_stay_in_one_cell() {
        let raw = "| Cmd | Note |\n|---|---|\n| `cat f | wc -l` | a \\| b |";
        let tables = extract_markdown_tables(raw);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].headers, vec!["Cmd", "Note"]);
        assert_eq!(tables[0].rows[0], vec!["`cat f | wc -l`", "a | b"]);
        let (_, images) = transform_markdown_tables_to_images(raw);
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn malformed_rows_keep_raw_text() {
        let raw = "| A |\n|---|\n| x | y |";
        let (output, images) = transform_markdown_tables_to_images(raw);
        assert_eq!(output, raw);
        assert!(images.is_empty());
    }

    #[test]
    fn fallback_text_fences_original_tables() {
        let table = RenderedTable {
            original_text: "| A |\n|---|\n| ``` |".to_string(),
            png_bytes: Vec::new(),
            filename: "table_1.png".to_string(),
        };
        let text = fallback_text(&[table]);
        assert!(text.starts_with("````\n| A |"));
        assert!(text.ends_with("| ``` |\n````"));
    }

    #[test]
    fn fences_and_unbroken_cells_preserve_content() {
        let cell = "한".repeat(100);
        let raw = format!("| Value |\n|---|\n| {cell} |");
        let fenced = format!("```markdown\n{raw}\n```\n");
        let payload = format!("{fenced}\n{raw}\nEND");
        let (modified, images) = transform_markdown_tables_to_images(&payload);
        let preserved = modified.starts_with(&fenced) && images.len() == 1;
        let svg = render_table_to_svg(&extract_markdown_tables(&raw)[0]);
        assert_eq!(svg.matches('한').count(), 100);
        let mut options = usvg::Options::default();
        options.fontdb_mut().load_system_fonts();
        let tree = usvg::Tree::from_str(&svg, &options).unwrap();
        let mut text_nodes = 0;
        let mut inside = true;
        for node in tree.root().children() {
            if let usvg::Node::Text(_) = node {
                text_nodes += 1;
                let bounds = node.abs_bounding_box();
                inside &= bounds.left() >= 16.0
                    && bounds.right() <= tree.size().width() - 16.0
                    && bounds.top() >= 0.0
                    && bounds.bottom() <= tree.size().height();
                println!("text bounds: {bounds:?}; canvas: {:?}", tree.size());
            }
        }
        assert!(text_nodes >= 2, "must measure actual shaped text");
        println!(
            "fence_preserved={preserved}; glyphs_inside={inside}; images={}",
            images.len()
        );
        assert!(
            preserved && inside,
            "fenced example changed or cell glyphs clipped"
        );
        assert!(modified.ends_with("\nEND"));
        assert_eq!(images[0].original_text, raw);
        let png = resvg::tiny_skia::Pixmap::decode_png(&images[0].png_bytes).unwrap();
        assert_eq!(png.width(), tree.size().width() as u32 * 2);
        assert_eq!(png.height(), tree.size().height() as u32 * 2);
        if let Some(path) = std::env::var_os("U34_PNG") {
            std::fs::write(path, &images[0].png_bytes).unwrap();
        }
    }

    #[test]
    fn table_source_span_and_fence_controls() {
        let raw = "| A |\r\n|---|\r\n| B |";
        for (open, close) in [("```md", "```"), ("~~~~", "~~~~~"), ("   ```", "   ```")] {
            let example = format!("{open}\r\n{raw}\r\n{close}\r\n");
            let payload = format!("{example}{raw}\r\n\r\n{raw}\r\nTAIL");
            let (output, images) = transform_markdown_tables_to_images(&payload);
            assert!(output.starts_with(&example));
            assert!(output.ends_with("\r\nTAIL"));
            assert_eq!(images.len(), 2);
            assert!(images.iter().all(|image| image.original_text == raw));
            assert_eq!(output.matches("table_1.png").count(), 1);
            assert_eq!(output.matches("table_2.png").count(), 1);
        }
        for payload in [
            format!("````\n```\n{raw}"),
            format!("~~~\n```\n{raw}"),
            format!("```\n``` not a close\n{raw}"),
            "plain text without tables".to_string(),
            "| header |\n|---|".to_string(),
        ] {
            let (output, images) = transform_markdown_tables_to_images(&payload);
            assert_eq!(output, payload);
            assert!(images.is_empty());
        }
        let raw = format!(
            "| Wide | Small |\n|---|---|\n| {} | ok<br>end |",
            "W".repeat(100)
        );
        let svg = render_table_to_svg(&extract_markdown_tables(&raw)[0]);
        assert_eq!(svg.matches('W').count(), 101);
        assert!(svg.contains(">ok</text>") && svg.contains(">end</text>"));
    }

    #[test]
    fn test_extract_markdown_tables() {
        let sample = "Header\n\n| Col A | Col B |\n|---|---|\n| Val 1 | Val 2 |\n\nFooter";
        let tables = extract_markdown_tables(sample);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].headers, vec!["Col A", "Col B"]);
        assert_eq!(tables[0].rows.len(), 1);
        assert_eq!(tables[0].rows[0], vec!["Val 1", "Val 2"]);
    }

    #[test]
    fn test_render_table_to_svg_and_png() {
        let sample = "| Col A | Col B |\n|---|---|\n| Val 1 | Val 2<br>Line 2 |";
        let tables = extract_markdown_tables(sample);
        assert_eq!(tables.len(), 1);
        let svg = render_table_to_svg(&tables[0]);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Val 1"));

        let png = svg_to_png(&svg, 1.0).expect("should render png");
        assert!(!png.is_empty());
        assert_eq!(&png[0..8], b"\x89PNG\r\n\x1a\n");
    }
}
