use itertools::Itertools;

#[derive(Clone, Copy, Debug)]
pub enum InlineType {
    Text,
    Bold,
}

#[derive(Clone, Debug)]
pub struct Token {
    inline_type: InlineType,
    text: Option<String>,
    children: Vec<Token>,
}

impl Token {
    pub fn new(
        inline_type: InlineType, 
        text: Option<String>, 
        children: Option<Vec<Token>>
    ) -> Self 
    {
        let children = match children {
            Some(v) => v,
            None => Vec::new()
        };

        Token { inline_type, text, children }
    }

    pub fn to_html(&self) -> String {
        match self.inline_type {
            InlineType::Text => {
                assert!(self.text.is_some());
                self.text.clone().unwrap()
            },
            InlineType::Bold => {
                let children_html = self.children
                    .iter()
                    .map(|elm| elm.to_html())
                    .join("");
                format!("<strong>{}</strong>", children_html)
            }
        }
    }
}