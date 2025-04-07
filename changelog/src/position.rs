use crate::utils::UNRELEASED_SECTION_PATTERN;

pub fn find_unreleased_position(content: &str) -> Option<usize> {
    content
        .lines()
        .position(|line| UNRELEASED_SECTION_PATTERN.is_match(line))
}

pub fn find_next_section_position(lines: &[&str], start_idx: usize) -> usize {
    lines
        .iter()
        .skip(start_idx + 1)
        .position(|line| line.starts_with("## "))
        .map(|pos| pos + start_idx + 1)
        .unwrap_or(lines.len())
}

pub fn find_first_version_position(lines: &[&str]) -> (usize, bool) {
    let mut found_version = false;
    let position = lines
        .iter()
        .position(|line| {
            if line.starts_with("## [") {
                found_version = true;
                true
            } else {
                false
            }
        })
        .unwrap_or(lines.len());

    (position, found_version)
}
