pub(super) fn normalize_search_token(value: &str) -> String {
    value.trim().to_lowercase()
}

pub(super) fn tokenize_search_query(query: Option<&str>) -> Vec<String> {
    query.unwrap_or_default()
        .split(|char: char| !char.is_ascii_alphanumeric())
        .map(normalize_search_token)
        .filter(|token| !token.is_empty())
        .collect()
}

pub(super) fn matches_search_tokens(fields: &[String], tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return true;
    }
    tokens
        .iter()
        .all(|token| fields.iter().any(|field| field.contains(token)))
}

pub(super) fn score_search_tokens(weighted_fields: &[(String, usize)], tokens: &[String]) -> usize {
    if tokens.is_empty() {
        return 0;
    }
    tokens
        .iter()
        .map(|token| {
            weighted_fields
                .iter()
                .filter_map(|(field, weight)| {
                    if field == token {
                        Some(weight * 6)
                    } else if field.starts_with(token) {
                        Some(weight * 4)
                    } else if field.contains(token) {
                        Some(*weight)
                    } else {
                        None
                    }
                })
                .max()
                .unwrap_or_default()
        })
        .sum()
}
