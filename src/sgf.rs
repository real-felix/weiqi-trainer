use crate::domain::*;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
struct SgfNode {
    props: BTreeMap<String, Vec<String>>,
    children: Vec<SgfNode>,
}
struct Parser {
    chars: Vec<char>,
    at: usize,
    count: usize,
}
impl Parser {
    fn whitespace(&mut self) {
        while self.chars.get(self.at).is_some_and(|c| c.is_whitespace()) {
            self.at += 1;
        }
    }
    fn expect(&mut self, c: char) -> Result<()> {
        self.whitespace();
        if self.chars.get(self.at) != Some(&c) {
            return Err(format!("Invalid SGF: expected {c}"));
        }
        self.at += 1;
        Ok(())
    }
    fn tree(&mut self, depth: usize) -> Result<SgfNode> {
        if depth > 100 {
            return Err("SGF variations are nested too deeply".into());
        }
        self.expect('(')?;
        let mut sequence = vec![];
        loop {
            self.whitespace();
            if self.chars.get(self.at) != Some(&';') {
                break;
            }
            self.at += 1;
            self.count += 1;
            if self.count > 5000 {
                return Err("SGF has too many nodes".into());
            }
            let mut n = SgfNode::default();
            loop {
                self.whitespace();
                let start = self.at;
                while self
                    .chars
                    .get(self.at)
                    .is_some_and(|c| c.is_ascii_uppercase())
                {
                    self.at += 1;
                }
                if start == self.at {
                    break;
                }
                let key = self.chars[start..self.at].iter().collect::<String>();
                let mut values = vec![];
                loop {
                    self.whitespace();
                    if self.chars.get(self.at) != Some(&'[') {
                        break;
                    }
                    self.at += 1;
                    let mut value = String::new();
                    loop {
                        let c = *self.chars.get(self.at).ok_or("Unclosed SGF value")?;
                        self.at += 1;
                        if c == ']' {
                            break;
                        }
                        if c == '\\' {
                            let c = *self.chars.get(self.at).ok_or("Unclosed SGF escape")?;
                            self.at += 1;
                            if c == '\r' {
                                if self.chars.get(self.at) == Some(&'\n') {
                                    self.at += 1;
                                }
                            } else if c != '\n' {
                                value.push(c);
                            }
                        } else {
                            value.push(c);
                        }
                    }
                    values.push(value);
                }
                if values.is_empty() {
                    return Err("SGF property needs a value".into());
                }
                n.props.entry(key).or_default().extend(values);
            }
            sequence.push(n);
        }
        if sequence.is_empty() {
            return Err("Empty SGF tree".into());
        }
        loop {
            self.whitespace();
            if self.chars.get(self.at) != Some(&'(') {
                break;
            }
            let child = self.tree(depth + 1)?;
            sequence.last_mut().unwrap().children.push(child);
        }
        self.expect(')')?;
        let mut end = sequence.pop().unwrap();
        while let Some(mut n) = sequence.pop() {
            n.children.push(end);
            end = n;
        }
        Ok(end)
    }
}
fn property<'a>(n: &'a SgfNode, key: &str) -> Option<&'a str> {
    n.props.get(key).and_then(|v| v.first()).map(String::as_str)
}
fn point(value: &str) -> Result<Option<usize>> {
    if value.is_empty() {
        return Ok(None);
    }
    let b = value.as_bytes();
    if b.len() != 2 || !(b'a'..=b's').contains(&b[0]) || !(b'a'..=b's').contains(&b[1]) {
        return Err("Only 19×19 SGF coordinates are supported".into());
    }
    Ok(Some((b[1] - b'a') as usize * SIZE + (b[0] - b'a') as usize))
}
pub fn import(text: &str) -> Result<Game> {
    if text.len() > 1_000_000 {
        return Err("SGF exceeds 1 MB".into());
    }
    let mut parser = Parser {
        chars: text.chars().collect(),
        at: 0,
        count: 0,
    };
    let root = parser.tree(0)?;
    parser.whitespace();
    if parser.at != parser.chars.len() {
        return Err("Import one SGF game at a time".into());
    }
    if property(&root, "SZ").unwrap_or("19") != "19" || property(&root, "GM").unwrap_or("1") != "1"
    {
        return Err("Only 19×19 Go records are supported".into());
    }
    if property(&root, "KM").unwrap_or("7.5").parse::<f64>().ok() != Some(7.5) {
        return Err("This trainer requires komi 7.5".into());
    }
    if property(&root, "RU")
        .is_some_and(|r| !r.eq_ignore_ascii_case("aga") && !r.eq_ignore_ascii_case("american"))
    {
        return Err("Only AGA records are supported".into());
    }
    let mut g = Game::new(Color::B, -5, "import".into());
    g.imported = true;
    let mut ids = BTreeMap::new();
    let base = g.root.clone();
    let mut main_leaf = base.clone();
    if let Some(old) = property(&root, "XID") {
        ids.insert(old.to_string(), base.clone());
    }
    fn visit(
        g: &mut Game,
        n: &SgfNode,
        parent: &str,
        ids: &mut BTreeMap<String, String>,
        main: &mut String,
        is_main: bool,
        depth: usize,
    ) -> Result<()> {
        if depth > 1500 {
            return Err("SGF line is too long".into());
        }
        if ["AB", "AW", "AE", "HA", "PL"]
            .iter()
            .any(|k| n.props.contains_key(*k))
        {
            return Err("Setup stones, handicap and player overrides are unsupported; import an even AGA game".into());
        }
        if n.props.contains_key("B") && n.props.contains_key("W") {
            return Err("SGF node contains two moves".into());
        }
        let mut current = parent.to_string();
        for (key, color) in [("B", Color::B), ("W", Color::W)] {
            if let Some(value) = property(n, key) {
                if g.position(parent)?.turn != color {
                    return Err("SGF has an invalid move order".into());
                }
                current = g.add(parent, point(value)?, true)?;
            }
        }
        if let Some(old) = property(n, "XID") {
            ids.insert(old.into(), current.clone());
        }
        if property(n, "XRESUME") == Some("1") {
            g.resume_heads.push(current.clone());
        }
        if is_main {
            *main = current.clone();
        }
        for (i, child) in n.children.iter().enumerate() {
            visit(g, child, &current, ids, main, is_main && i == 0, depth + 1)?;
        }
        Ok(())
    }
    visit(&mut g, &root, &base, &mut ids, &mut main_leaf, true, 0)?;
    g.active_head = property(&root, "XHEAD")
        .and_then(|s| ids.get(s))
        .cloned()
        .unwrap_or(main_leaf);
    g.result = Some(property(&root, "RE").unwrap_or("Unfinished record").into());
    g.status = "finished".into();
    Ok(g)
}
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(']', "\\]")
}
pub fn export(g: &Game) -> String {
    let mut text = format!(
        "(;GM[1]FF[4]CA[UTF-8]AP[WeiqiTrainer:0.1]SZ[19]RU[AGA]KM[7.5]XID[{}]XHEAD[{}]PB[{}]PW[{}]",
        g.root,
        g.active_head,
        if g.human == Color::B {
            "You".into()
        } else {
            format!("Human SL {}", rank_label(g.rank))
        },
        if g.human == Color::W {
            "You".into()
        } else {
            format!("Human SL {}", rank_label(g.rank))
        }
    );
    if let Some(result) = &g.result {
        text.push_str(&format!("RE[{}]", escape(result)));
    }
    if g.resume_heads.contains(&g.root) {
        text.push_str("XRESUME[1]");
    }
    let active = g.path(&g.active_head).unwrap_or_default();
    fn children(g: &Game, parent: &str, text: &mut String, active: &[&Node]) {
        let mut nodes: Vec<_> = g
            .nodes
            .values()
            .filter(|n| n.parent.as_deref() == Some(parent))
            .collect();
        nodes.sort_by_key(|n| {
            (
                !active.iter().any(|a| a.id == n.id),
                n.created_at.clone(),
                n.id.clone(),
            )
        });
        let branch = nodes.len() > 1;
        for n in nodes {
            if branch {
                text.push('(');
            }
            let coord = n
                .point
                .map(|p| {
                    format!(
                        "{}{}",
                        (b'a' + (p % SIZE) as u8) as char,
                        (b'a' + (p / SIZE) as u8) as char
                    )
                })
                .unwrap_or_default();
            text.push_str(&format!(";{}[{}]XID[{}]", n.color.unwrap(), coord, n.id));
            if g.resume_heads.contains(&n.id) {
                text.push_str("XRESUME[1]");
            }
            children(g, &n.id, text, active);
            if branch {
                text.push(')');
            }
        }
    }
    children(g, &g.root, &mut text, &active);
    text.push(')');
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_variations_active_line_and_escapes() {
        let mut g = Game::new(Color::B, -5, "auto".into());
        let r = g.root.clone();
        let a = g.add(&r, Some(60), true).unwrap();
        g.add(&a, Some(61), true).unwrap();
        let b = g.add(&r, None, true).unwrap();
        g.add(&b, Some(20), true).unwrap();
        g.active_head = a;
        g.result = Some("B+R".into());
        let text = export(&g);
        let imported = import(&text).unwrap();
        assert_eq!(imported.nodes.len(), 5);
        assert_eq!(
            imported.position(&imported.active_head).unwrap().board[60],
            1
        );
        assert_eq!(imported.result, g.result);
        assert!(import("(;SZ[9])").is_err());
        assert!(import("(;SZ[19]KM[6.5])").is_err());
        assert!(import("(;C[a\\]b\\\\c](;B[aa])(;B[bb]))").is_ok());
        assert!(import("(;AB[aa])").is_err());
    }
}
