pub(super) fn extract_json_object(text: &str, accept: impl Fn(&str) -> bool) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.starts_with('{')
        && balanced_json_object_end(trimmed) == Some(trimmed.len())
        && accept(trimmed)
    {
        return Some(trimmed);
    }

    let mut fence_offset = 0;
    while let Some(start_rel) = trimmed[fence_offset..].find("```") {
        let fence_start = fence_offset + start_rel + 3;
        let after_open = &trimmed[fence_start..];
        let content_start = after_open
            .find('\n')
            .map_or(fence_start, |newline| fence_start + newline + 1);
        let Some(end_rel) = trimmed[content_start..].find("```") else {
            break;
        };
        let fenced = trimmed[content_start..content_start + end_rel].trim();
        if fenced.starts_with('{')
            && balanced_json_object_end(fenced) == Some(fenced.len())
            && accept(fenced)
        {
            return Some(fenced);
        }
        fence_offset = content_start + end_rel + 3;
    }

    for (start, _) in trimmed.match_indices('{').rev() {
        if let Some(end) = balanced_json_object_end(&trimmed[start..]) {
            let candidate = trimmed[start..start + end].trim();
            if accept(candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn balanced_json_object_end(input: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth = depth.checked_add(1)?,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index + ch.len_utf8());
                }
            }
            _ => {}
        }
    }

    None
}
