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
            if key == "id" {
                attrs.id = Some(value.to_string());
            } else {
                attrs.pairs.insert(key.to_string(), value.to_string());
            }
        } else if i == 0 && !part.contains('=') {
            // First bare value is treated as id
            attrs.id = Some(part.to_string());
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
}
