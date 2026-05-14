pub(crate) fn is_fenced_code_fence(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

pub(crate) fn inline_code_spans(line: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut offset = 0;
    while let Some(open_rel) = line[offset..].find('`') {
        let open = offset + open_rel;
        let ticks = line[open..]
            .bytes()
            .take_while(|byte| *byte == b'`')
            .count();
        let marker = &line[open..open + ticks];
        let content_start = open + ticks;
        let Some(close_rel) = line[content_start..].find(marker) else {
            break;
        };
        let close = content_start + close_rel + ticks;
        spans.push((open, close));
        offset = close;
    }
    spans
}

pub(crate) fn byte_in_spans(index: usize, spans: &[(usize, usize)]) -> bool {
    spans
        .iter()
        .any(|(start, end)| *start <= index && index < *end)
}
