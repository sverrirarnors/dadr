use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextDiffLine {
    pub kind: TextDiffKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextDiffKind {
    Context,
    Added,
    Removed,
}

pub(super) fn text_diff_lines(left: &str, right: &str) -> Vec<TextDiffLine> {
    let left_lines = left.lines().collect::<Vec<_>>();
    let right_lines = right.lines().collect::<Vec<_>>();
    let mut table = vec![vec![0usize; right_lines.len() + 1]; left_lines.len() + 1];

    for left_index in (0..left_lines.len()).rev() {
        for right_index in (0..right_lines.len()).rev() {
            table[left_index][right_index] = if left_lines[left_index] == right_lines[right_index] {
                table[left_index + 1][right_index + 1] + 1
            } else {
                table[left_index + 1][right_index].max(table[left_index][right_index + 1])
            };
        }
    }

    let mut diff = Vec::new();
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left_lines.len() && right_index < right_lines.len() {
        if left_lines[left_index] == right_lines[right_index] {
            diff.push(TextDiffLine {
                kind: TextDiffKind::Context,
                text: left_lines[left_index].to_owned(),
            });
            left_index += 1;
            right_index += 1;
        } else if table[left_index + 1][right_index] >= table[left_index][right_index + 1] {
            diff.push(TextDiffLine {
                kind: TextDiffKind::Removed,
                text: left_lines[left_index].to_owned(),
            });
            left_index += 1;
        } else {
            diff.push(TextDiffLine {
                kind: TextDiffKind::Added,
                text: right_lines[right_index].to_owned(),
            });
            right_index += 1;
        }
    }

    diff.extend(left_lines[left_index..].iter().map(|line| TextDiffLine {
        kind: TextDiffKind::Removed,
        text: (*line).to_owned(),
    }));
    diff.extend(right_lines[right_index..].iter().map(|line| TextDiffLine {
        kind: TextDiffKind::Added,
        text: (*line).to_owned(),
    }));
    diff
}
