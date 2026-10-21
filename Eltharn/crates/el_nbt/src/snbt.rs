use crate::tag::Tag;
pub fn to_string(tag: &Tag) -> String {
    match tag {
        Tag::End => "".to_string(),
        Tag::Byte(v) => format!("{}b", v),
        Tag::Short(v) => format!("{}s", v),
        Tag::Int(v) => format!("{}", v),
        Tag::Long(v) => format!("{}L", v),
        Tag::Float(v) => format!("{}f", v),
        Tag::Double(v) => format!("{}d", v),
        Tag::ByteArray(v) => {
            let mut s = String::from("[B;");
            for (i, val) in v.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&format!("{}b", val));
            }
            s.push(']');
            s
        }
        Tag::String(v) => escape_string(v),
        Tag::List(v) => {
            let mut s = String::from("[");
            for (i, val) in v.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&to_string(val));
            }
            s.push(']');
            s
        }
        Tag::Compound(fields) => {
            let mut s = String::from("{");
            for (i, (k, v)) in fields.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&escape_key(k));
                s.push_str(": ");
                s.push_str(&to_string(v));
            }
            s.push('}');
            s
        }
        Tag::IntArray(v) => {
            let mut s = String::from("[I;");
            for (i, val) in v.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&format!("{}", val));
            }
            s.push(']');
            s
        }
        Tag::LongArray(v) => {
            let mut s = String::from("[L;");
            for (i, val) in v.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&format!("{}L", val));
            }
            s.push(']');
            s
        }
    }
}
fn escape_string(s: &str) -> String {
    let mut escaped = String::new();
    escaped.push('"');
    for c in s.chars() {
        if c == '"' || c == '\\' {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped.push('"');
    escaped
}
fn escape_key(s: &str) -> String {
    let mut needs_quotes = s.is_empty();
    for c in s.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.' && c != '+' {
            needs_quotes = true;
            break;
        }
    }
    if needs_quotes {
        escape_string(s)
    } else {
        s.to_string()
    }
}
struct Parser {
    chars: Vec<char>,
    pos: usize,
}
impl Parser {
    fn new(input: &str) -> Self {
        Parser {
            chars: input.chars().collect(),
            pos: 0,
        }
    }
    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }
    fn peek(&mut self) -> Option<char> {
        self.skip_whitespace();
        if self.pos < self.chars.len() {
            Some(self.chars[self.pos])
        } else {
            None
        }
    }
    fn next_char(&mut self) -> Option<char> {
        self.skip_whitespace();
        if self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }
    fn expect(&mut self, expected: char) -> Result<(), String> {
        match self.next_char() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(format!(
                "Expected '{}', found '{}' at position {}",
                expected, c, self.pos
            )),
            None => Err(format!("Expected '{}', found EOF", expected)),
        }
    }
    fn parse_tag(&mut self) -> Result<Tag, String> {
        let c = self.peek().ok_or_else(|| "Unexpected EOF".to_string())?;
        match c {
            '{' => self.parse_compound(),
            '[' => self.parse_array_or_list(),
            '"' | '\'' => self.parse_quoted_string(),
            _ => self.parse_literal(),
        }
    }
    fn parse_compound(&mut self) -> Result<Tag, String> {
        self.expect('{')?;
        let mut fields: Vec<(String, Tag)> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some('}') {
            self.expect('}')?;
            return Ok(Tag::Compound(fields));
        }
        loop {
            let key = self.parse_key()?;
            self.expect(':')?;
            let val = self.parse_tag()?;
            fields.push((key, val));
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.expect(',')?;
                }
                Some('}') => {
                    self.expect('}')?;
                    break;
                }
                Some(c) => return Err(format!("Expected ',' or '}}', found '{}'", c)),
                None => return Err("Unexpected EOF in Compound".to_string()),
            }
        }
        Ok(Tag::Compound(fields))
    }
    fn parse_key(&mut self) -> Result<String, String> {
        let c = self
            .peek()
            .ok_or_else(|| "Unexpected EOF reading key".to_string())?;
        if c == '"' || c == '\'' {
            let Tag::String(s) = self.parse_quoted_string()? else {
                unreachable!()
            };
            Ok(s)
        } else {
            let mut s = String::new();
            while self.pos < self.chars.len() {
                let c = self.chars[self.pos];
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '+' {
                    s.push(c);
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if s.is_empty() {
                Err(format!("Empty key at position {}", self.pos))
            } else {
                Ok(s)
            }
        }
    }
    fn parse_quoted_string(&mut self) -> Result<Tag, String> {
        let quote = self.next_char().unwrap();
        let mut s = String::new();
        let mut escaped = false;
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            self.pos += 1;
            if escaped {
                s.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == quote {
                return Ok(Tag::String(s));
            } else {
                s.push(c);
            }
        }
        Err("Unclosed quote".to_string())
    }
    fn parse_array_or_list(&mut self) -> Result<Tag, String> {
        self.expect('[')?;
        self.skip_whitespace();
        if self.pos + 1 < self.chars.len() && self.chars[self.pos + 1] == ';' {
            let prefix = self.chars[self.pos];
            self.pos += 2;
            match prefix {
                'B' | 'b' => self.parse_byte_array(),
                'I' | 'i' => self.parse_int_array(),
                'L' | 'l' => self.parse_long_array(),
                c => Err(format!("Unknown array type prefix: {}", c)),
            }
        } else {
            let mut list = Vec::new();
            if self.peek() == Some(']') {
                self.expect(']')?;
                return Ok(Tag::List(list));
            }
            loop {
                list.push(self.parse_tag()?);
                self.skip_whitespace();
                match self.peek() {
                    Some(',') => {
                        self.expect(',')?;
                    }
                    Some(']') => {
                        self.expect(']')?;
                        break;
                    }
                    Some(c) => return Err(format!("Expected ',' or ']', found '{}'", c)),
                    None => return Err("Unexpected EOF in List".to_string()),
                }
            }
            Ok(Tag::List(list))
        }
    }
    fn parse_byte_array(&mut self) -> Result<Tag, String> {
        let mut array = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.expect(']')?;
            return Ok(Tag::ByteArray(array));
        }
        loop {
            let tag = self.parse_literal()?;
            match tag {
                Tag::Byte(b) => array.push(b),
                Tag::Int(i) => array.push(i as i8),
                _ => return Err(format!("Expected Byte in ByteArray, found {:?}", tag)),
            }
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.expect(',')?;
                }
                Some(']') => {
                    self.expect(']')?;
                    break;
                }
                Some(c) => return Err(format!("Expected ',' or ']', found '{}'", c)),
                None => return Err("Unexpected EOF in ByteArray".to_string()),
            }
        }
        Ok(Tag::ByteArray(array))
    }
    fn parse_int_array(&mut self) -> Result<Tag, String> {
        let mut array = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.expect(']')?;
            return Ok(Tag::IntArray(array));
        }
        loop {
            let tag = self.parse_literal()?;
            match tag {
                Tag::Int(i) => array.push(i),
                _ => return Err(format!("Expected Int in IntArray, found {:?}", tag)),
            }
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.expect(',')?;
                }
                Some(']') => {
                    self.expect(']')?;
                    break;
                }
                Some(c) => return Err(format!("Expected ',' or ']', found '{}'", c)),
                None => return Err("Unexpected EOF in IntArray".to_string()),
            }
        }
        Ok(Tag::IntArray(array))
    }
    fn parse_long_array(&mut self) -> Result<Tag, String> {
        let mut array = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(']') {
            self.expect(']')?;
            return Ok(Tag::LongArray(array));
        }
        loop {
            let tag = self.parse_literal()?;
            match tag {
                Tag::Long(l) => array.push(l),
                Tag::Int(i) => array.push(i as i64),
                _ => return Err(format!("Expected Long in LongArray, found {:?}", tag)),
            }
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.expect(',')?;
                }
                Some(']') => {
                    self.expect(']')?;
                    break;
                }
                Some(c) => return Err(format!("Expected ',' or ']', found '{}'", c)),
                None => return Err("Unexpected EOF in LongArray".to_string()),
            }
        }
        Ok(Tag::LongArray(array))
    }
    fn parse_literal(&mut self) -> Result<Tag, String> {
        let mut s = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '+' {
                s.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        if s.is_empty() {
            return Err(format!("Empty literal at position {}", self.pos));
        }
        let lower = s.to_lowercase();
        if lower == "true" {
            return Ok(Tag::Byte(1));
        }
        if lower == "false" {
            return Ok(Tag::Byte(0));
        }
        if lower.ends_ok("b") {
            if let Ok(val) = s[..s.len() - 1].parse::<i8>() {
                return Ok(Tag::Byte(val));
            }
        }
        if lower.ends_ok("s") {
            if let Ok(val) = s[..s.len() - 1].parse::<i16>() {
                return Ok(Tag::Short(val));
            }
        }
        if lower.ends_ok("l") {
            if let Ok(val) = s[..s.len() - 1].parse::<i64>() {
                return Ok(Tag::Long(val));
            }
        }
        if lower.ends_ok("f") {
            if let Ok(val) = s[..s.len() - 1].parse::<f32>() {
                return Ok(Tag::Float(val));
            }
        }
        if lower.ends_ok("d") {
            if let Ok(val) = s[..s.len() - 1].parse::<f64>() {
                return Ok(Tag::Double(val));
            }
        }
        if let Ok(val) = s.parse::<i32>() {
            return Ok(Tag::Int(val));
        }
        if let Ok(val) = s.parse::<i64>() {
            return Ok(Tag::Long(val));
        }
        if s.contains('.') {
            if let Ok(val) = s.parse::<f64>() {
                return Ok(Tag::Double(val));
            }
        }
        Ok(Tag::String(s))
    }
}
trait EndsOk {
    fn ends_ok(&self, suffix: &str) -> bool;
}
impl EndsOk for String {
    fn ends_ok(&self, suffix: &str) -> bool {
        self.ends_with(suffix) && self.len() > suffix.len()
    }
}
pub fn from_str(s: &str) -> Result<Tag, String> {
    let mut parser = Parser::new(s);
    let tag = parser.parse_tag()?;
    parser.skip_whitespace();
    if parser.pos < parser.chars.len() {
        Err(format!(
            "Trailing characters starting at position {}",
            parser.pos
        ))
    } else {
        Ok(tag)
    }
}
