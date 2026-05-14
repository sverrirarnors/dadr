use std::fmt;

use serde::Serialize;

pub fn ascii_slug(input: &str, max_len: usize, fallback: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for ch in input.chars().flat_map(char::to_lowercase) {
        if slug.len() >= max_len {
            break;
        }
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        fallback.to_owned()
    } else {
        slug
    }
}

pub fn normalized_tag_key(input: &str) -> String {
    input.trim().trim_start_matches('#').to_lowercase()
}

pub fn debug_snake(value: &impl fmt::Debug) -> String {
    let debug = format!("{value:?}");
    let mut snake = String::new();
    for ch in debug.chars() {
        if ch.is_ascii_uppercase() {
            if !snake.is_empty() {
                snake.push('_');
            }
            snake.push(ch.to_ascii_lowercase());
        } else {
            snake.push(ch);
        }
    }
    snake
}

pub(crate) fn serde_label<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|encoded| encoded.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

pub(crate) fn write_serde_label<T: Serialize + ?Sized>(
    value: &T,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    formatter.write_str(&serde_label(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    enum SampleCode {
        MissingFrontMatter,
    }

    #[test]
    fn ascii_slug_collapses_separators_and_uses_fallback() {
        assert_eq!(ascii_slug("Hello, World!", 80, "untitled"), "hello-world");
        assert_eq!(ascii_slug("!!!", 80, "untitled"), "untitled");
    }

    #[test]
    fn normalized_tag_key_strips_hash_and_case() {
        assert_eq!(normalized_tag_key(" #Kind/Sample "), "kind/sample");
    }

    #[test]
    fn debug_names_convert_to_snake_case() {
        assert_eq!(
            debug_snake(&SampleCode::MissingFrontMatter),
            "missing_front_matter"
        );
    }
}
