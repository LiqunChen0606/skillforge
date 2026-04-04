use aif_core::ast::Attrs;

/// Parse attribute string like "id=intro, src=data.csv" or just "intro" (bare id)
pub fn parse_attrs(input: &str) -> Attrs {
    let input = input.trim();
    if input.is_empty() {
        return Attrs::new();
    }

    let mut attrs = Attrs::new();
    let parts: Vec<&str> = input.split(',').map(|s| s.trim()).collect();

    for (i, part) in parts.iter().enumerate() {
        if let Some((key, value)) = part.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                &value[1..value.len() - 1]
            } else {
                value
            };
            if key == "id" {
                attrs.id = Some(value.to_string());
            } else {
                attrs.pairs.insert(key.to_string(), value.to_string());
            }
        } else if i == 0 && !part.contains('=') {
            // First bare value is treated as id
            let bare = part.trim();
            let bare = if (bare.starts_with('"') && bare.ends_with('"'))
                || (bare.starts_with('\'') && bare.ends_with('\''))
            {
                &bare[1..bare.len() - 1]
            } else {
                bare
            };
            attrs.id = Some(bare.to_string());
        }
    }

    attrs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_attrs() {
        let attrs = parse_attrs("");
        assert_eq!(attrs.id, None);
        assert!(attrs.pairs.is_empty());
    }

    #[test]
    fn parse_bare_id() {
        let attrs = parse_attrs("intro");
        assert_eq!(attrs.id, Some("intro".to_string()));
    }

    #[test]
    fn parse_key_value_attrs() {
        let attrs = parse_attrs("id=main, src=data.csv");
        assert_eq!(attrs.id, Some("main".to_string()));
        assert_eq!(attrs.get("src"), Some("data.csv"));
    }

    #[test]
    fn parse_mixed_attrs() {
        let attrs = parse_attrs("id=t1, src=data.csv, caption=true");
        assert_eq!(attrs.id, Some("t1".to_string()));
        assert_eq!(attrs.get("src"), Some("data.csv"));
        assert_eq!(attrs.get("caption"), Some("true"));
    }

    #[test]
    fn parse_double_quoted_values() {
        let attrs = parse_attrs("name=\"code-review\", version=\"1.0\"");
        assert_eq!(attrs.get("name"), Some("code-review"));
        assert_eq!(attrs.get("version"), Some("1.0"));
    }

    #[test]
    fn parse_single_quoted_values() {
        let attrs = parse_attrs("name='code-review'");
        assert_eq!(attrs.get("name"), Some("code-review"));
    }

    #[test]
    fn parse_mixed_quoted_unquoted() {
        let attrs = parse_attrs("name=\"test\", profile=migration");
        assert_eq!(attrs.get("name"), Some("test"));
        assert_eq!(attrs.get("profile"), Some("migration"));
    }

    #[test]
    fn parse_id_with_quotes() {
        let attrs = parse_attrs("id=\"main\"");
        assert_eq!(attrs.id, Some("main".to_string()));
    }
}
