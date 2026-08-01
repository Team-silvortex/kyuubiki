use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum CopyValue {
    String(String),
    Strings(Vec<String>),
}

pub(super) fn parse_copy_entries(source: &str) -> Result<BTreeMap<String, CopyValue>, String> {
    let export = source
        .find("export const ")
        .ok_or_else(|| "copy source is missing an exported const object".to_string())?;
    let object = source[export..]
        .find('{')
        .map(|offset| export + offset)
        .ok_or_else(|| "copy source is missing its root object".to_string())?;
    let mut parser = Parser::new(source, object);
    let mut entries = BTreeMap::new();
    parser.parse_object("", &mut entries)?;
    Ok(entries)
}

struct Parser<'a> {
    source: &'a str,
    offset: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, offset: usize) -> Self {
        Self { source, offset }
    }

    fn parse_object(
        &mut self,
        prefix: &str,
        entries: &mut BTreeMap<String, CopyValue>,
    ) -> Result<(), String> {
        self.expect('{')?;
        loop {
            self.skip_trivia();
            if self.consume('}') {
                return Ok(());
            }
            let key = self.parse_property_name()?;
            self.skip_trivia();
            self.expect(':')?;
            self.skip_trivia();
            let path = if prefix.is_empty() {
                key
            } else {
                format!("{prefix}.{key}")
            };
            match self.peek() {
                Some('{') => self.parse_object(&path, entries)?,
                Some('"' | '\'') => {
                    let value = self.parse_string()?;
                    entries.insert(path, CopyValue::String(value));
                }
                Some('[') => {
                    if let Some(values) = self.parse_string_array()? {
                        entries.insert(path, CopyValue::Strings(values));
                    }
                }
                Some(_) => self.skip_expression(),
                None => return Err("copy source ended inside an object".to_string()),
            }
            self.skip_trivia();
            if self.consume(',') {
                continue;
            }
            if self.consume('}') {
                return Ok(());
            }
            return Err(self.error("expected ',' or '}' after property"));
        }
    }

    fn parse_property_name(&mut self) -> Result<String, String> {
        match self.peek() {
            Some('"' | '\'') => self.parse_string(),
            Some(character) if is_identifier_start(character) => {
                let start = self.offset;
                self.advance();
                while self.peek().is_some_and(is_identifier_continue) {
                    self.advance();
                }
                Ok(self.source[start..self.offset].to_string())
            }
            _ => Err(self.error("expected a property name")),
        }
    }

    fn parse_string_array(&mut self) -> Result<Option<Vec<String>>, String> {
        let start = self.offset;
        self.expect('[')?;
        let mut values = Vec::new();
        loop {
            self.skip_trivia();
            if self.consume(']') {
                return Ok(Some(values));
            }
            if !matches!(self.peek(), Some('"' | '\'')) {
                self.offset = start;
                self.skip_expression();
                return Ok(None);
            }
            values.push(self.parse_string()?);
            self.skip_trivia();
            if self.consume(',') {
                continue;
            }
            if self.consume(']') {
                return Ok(Some(values));
            }
            return Err(self.error("expected ',' or ']' in string array"));
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        let quote = self
            .peek()
            .filter(|value| matches!(value, '"' | '\''))
            .ok_or_else(|| self.error("expected a quoted string"))?;
        self.advance();
        let mut output = String::new();
        while let Some(character) = self.advance() {
            if character == quote {
                return Ok(output);
            }
            if character != '\\' {
                output.push(character);
                continue;
            }
            let escaped = self
                .advance()
                .ok_or_else(|| self.error("unterminated string escape"))?;
            match escaped {
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                'b' => output.push('\u{0008}'),
                'f' => output.push('\u{000c}'),
                'v' => output.push('\u{000b}'),
                '0' => output.push('\0'),
                'u' => output.push(self.parse_unicode_escape()?),
                '\n' => {}
                other => output.push(other),
            }
        }
        Err(self.error("unterminated quoted string"))
    }

    fn parse_unicode_escape(&mut self) -> Result<char, String> {
        let start = self.offset;
        for _ in 0..4 {
            if !self.peek().is_some_and(|value| value.is_ascii_hexdigit()) {
                return Err(self.error("invalid unicode escape"));
            }
            self.advance();
        }
        let value = u32::from_str_radix(&self.source[start..self.offset], 16)
            .map_err(|error| format!("invalid unicode escape: {error}"))?;
        char::from_u32(value).ok_or_else(|| self.error("invalid unicode scalar"))
    }

    fn skip_expression(&mut self) {
        let mut braces = 0usize;
        let mut brackets = 0usize;
        let mut parentheses = 0usize;
        while let Some(character) = self.peek() {
            if matches!(character, '"' | '\'' | '`') {
                self.skip_quoted(character);
                continue;
            }
            if self.starts_with("//") || self.starts_with("/*") {
                self.skip_trivia();
                continue;
            }
            match character {
                '{' => braces += 1,
                '}' if braces > 0 => braces -= 1,
                '}' if brackets == 0 && parentheses == 0 => return,
                '[' => brackets += 1,
                ']' if brackets > 0 => brackets -= 1,
                '(' => parentheses += 1,
                ')' if parentheses > 0 => parentheses -= 1,
                ',' if braces == 0 && brackets == 0 && parentheses == 0 => return,
                _ => {}
            }
            self.advance();
        }
    }

    fn skip_quoted(&mut self, quote: char) {
        self.advance();
        while let Some(character) = self.advance() {
            if character == '\\' {
                self.advance();
            } else if character == quote {
                return;
            }
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.advance();
            }
            if self.starts_with("//") {
                while self.advance().is_some_and(|value| value != '\n') {}
            } else if self.starts_with("/*") {
                self.offset += 2;
                while self.offset < self.source.len() && !self.starts_with("*/") {
                    self.advance();
                }
                if self.starts_with("*/") {
                    self.offset += 2;
                }
            } else {
                return;
            }
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        if self.consume(expected) {
            Ok(())
        } else {
            Err(self.error(&format!("expected '{expected}'")))
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn starts_with(&self, value: &str) -> bool {
        self.source[self.offset..].starts_with(value)
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let value = self.peek()?;
        self.offset += value.len_utf8();
        Some(value)
    }

    fn error(&self, message: &str) -> String {
        format!("{message} at source byte {}", self.offset)
    }
}

fn is_identifier_start(value: char) -> bool {
    value == '_' || value == '$' || value.is_ascii_alphabetic()
}

fn is_identifier_continue(value: char) -> bool {
    is_identifier_start(value) || value.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::{CopyValue, parse_copy_entries};

    #[test]
    fn extracts_literal_copy_and_ignores_runtime_expressions() {
        let source = r#"
            import brand from "./brand.json";
            export const copy = {
              title: "Workbench",
              nested: { hint: 'Ready', rows: ["One", "Two"] },
              dynamic: brand.productName,
              template: `${brand.productName} ready`,
            } as const;
        "#;
        let entries = parse_copy_entries(source).expect("copy should parse");
        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries.get("nested.rows"),
            Some(&CopyValue::Strings(vec!["One".into(), "Two".into()]))
        );
        assert_eq!(
            entries.get("nested.hint"),
            Some(&CopyValue::String("Ready".into()))
        );
        assert!(!entries.contains_key("dynamic"));
        assert!(!entries.contains_key("template"));
    }
}
