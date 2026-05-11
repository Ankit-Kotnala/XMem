use std::collections::BTreeMap;

pub type Metadata = BTreeMap<String, String>;

pub fn extract_structured_metadata(content: &str) -> Metadata {
    let mut result = Metadata::new();

    if let Some((key_part, memo)) = content.split_once(" = ") {
        if key_part.contains(" / ") {
            let key = key_part
                .trim()
                .replace(" / ", "_")
                .replace(' ', "_")
                .to_lowercase();

            result.insert("main_content".to_string(), key);
            result.insert("subcontent".to_string(), memo.trim().to_string());
            return result;
        }
    }

    result.insert("main_content".to_string(), String::new());
    result.insert("subcontent".to_string(), content.trim().to_string());
    result
}

pub fn parse_temporal_content(content: &str) -> Metadata {
    let mut result = Metadata::new();
    let parts: Vec<&str> = content.split('|').map(str::trim).collect();

    insert_part(&mut result, "date", &parts, 0, true);
    insert_part(&mut result, "event_name", &parts, 1, true);
    insert_part(&mut result, "desc", &parts, 2, true);
    insert_part(&mut result, "year", &parts, 3, false);
    insert_part(&mut result, "time", &parts, 4, false);
    insert_part(&mut result, "date_expression", &parts, 5, false);

    result
}

pub fn parse_snippet_content(content: &str) -> Metadata {
    let mut result = Metadata::new();
    let parts: Vec<&str> = content.split(" | ").map(str::trim).collect();

    match parts.len() {
        5.. => {
            result.insert("content".to_string(), parts[0].to_string());
            result.insert("code_snippet".to_string(), parts[1].to_string());
            result.insert("language".to_string(), parts[2].to_string());
            result.insert("snippet_type".to_string(), parts[3].to_string());
            result.insert("tags".to_string(), parts[4].to_string());
        }
        3..=4 => {
            result.insert("content".to_string(), parts[0].to_string());
            result.insert("code_snippet".to_string(), parts[1].to_string());
            result.insert("language".to_string(), parts[2].to_string());
            result.insert("snippet_type".to_string(), "algorithm".to_string());
            result.insert("tags".to_string(), String::new());
        }
        _ => {
            result.insert("content".to_string(), content.to_string());
            result.insert("code_snippet".to_string(), String::new());
            result.insert("language".to_string(), String::new());
            result.insert("snippet_type".to_string(), "algorithm".to_string());
            result.insert("tags".to_string(), String::new());
        }
    }

    result
}

pub fn parse_code_annotation_content(content: &str) -> Metadata {
    let mut result = Metadata::new();
    let parts: Vec<&str> = content.split('|').map(str::trim).collect();

    if parts.len() >= 6 {
        result.insert(
            "annotation_type".to_string(),
            value_or_default(parts[0], "explanation"),
        );
        result.insert("target_symbol".to_string(), parts[1].to_string());
        result.insert("target_file".to_string(), parts[2].to_string());
        result.insert("repo".to_string(), parts[3].to_string());
        result.insert("severity".to_string(), parts[4].to_string());
        result.insert("content".to_string(), parts[5].to_string());
    } else if parts.len() >= 2 {
        result.insert(
            "annotation_type".to_string(),
            value_or_default(parts[0], "explanation"),
        );
        result.insert("content".to_string(), parts[1..].join(" | "));
    } else {
        result.insert("content".to_string(), content.to_string());
        result.insert("annotation_type".to_string(), "explanation".to_string());
    }

    result
}

fn insert_part(
    target: &mut Metadata,
    key: &str,
    parts: &[&str],
    index: usize,
    include_empty: bool,
) {
    if let Some(value) = parts.get(index) {
        if include_empty || !value.is_empty() {
            target.insert(key.to_string(), (*value).to_string());
        }
    }
}

fn value_or_default(value: &str, default: &str) -> String {
    if value.is_empty() {
        default.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_profile_metadata() {
        let meta = extract_structured_metadata("work / company = OpenAI");
        assert_eq!(meta["main_content"], "work_company");
        assert_eq!(meta["subcontent"], "OpenAI");
    }

    #[test]
    fn parses_temporal_pipe_content() {
        let event = parse_temporal_content("04-24 | Demo | Product demo | 2026 | 10:00 | today");
        assert_eq!(event["date"], "04-24");
        assert_eq!(event["event_name"], "Demo");
        assert_eq!(event["desc"], "Product demo");
        assert_eq!(event["year"], "2026");
    }
}
