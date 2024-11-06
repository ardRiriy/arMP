use itertools::Itertools;

use crate::lexer::InlineLexer;

#[derive(Clone, Copy, Debug)]
pub enum InlineType {
    Text,
    Bold,
    Code,
    LineBreak,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockType {
    h1,
    h2,
    h3,
    Plain,
    Empty, // 段落替え
    Hr, // 区切り線 
}

#[derive(Clone, Debug)]
pub struct InlineToken {
    inline_type: InlineType,
    text: Option<String>,
    children: Vec<InlineToken>,
}

impl InlineToken {
    pub fn new(
        inline_type: InlineType, 
        text: Option<String>, 
        children: Option<Vec<InlineToken>>
    ) -> Self 
    {
        let children = children.unwrap_or_default();

        InlineToken { inline_type, text, children }
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
            },
            InlineType::LineBreak => "<br>".to_string(),
            InlineType::Code => {
                assert!(self.text.is_some());
                format!("<code>{}</code>", self.text.clone().unwrap())
            }
        }
    }
}


#[derive(Clone, Debug)]
pub struct BlockToken {
    block_type: BlockType,
    inline_tokens: Vec<InlineToken>,
}

impl BlockToken {
    pub fn new(block_type: BlockType) -> Self {
        Self { block_type, inline_tokens: Vec::new() }
    }

    pub fn is_same_type(&self, other: BlockType) -> bool {
        self.block_type == other
    }

    pub fn proceed_block_contest(&mut self, content: String) {
        if !self.inline_tokens.is_empty() {
            self.inline_tokens.push(InlineToken::new(InlineType::LineBreak, None, None));
        }
        self.inline_tokens = [self.inline_tokens.clone(), 
            InlineLexer::new(content.chars().collect()).tokenize()].iter()
            .flatten()
            .cloned()
            .collect();
    }

    pub fn to_html(&self) -> String {
        let content = self.inline_tokens
            .iter()
            .map(|it| it.to_html())
            .join("\n");
        match self.block_type {
            BlockType::h1 => format!("<h1>{content}</h1>"),
            BlockType::h2 => format!("<h2>{content}</h2>"),
            BlockType::h3 => format!("<h3>{content}</h3>"),
            BlockType::Plain => format!("<p>{content}</p>"),
            BlockType::Empty => "<br>".to_string(),
            BlockType::Hr => "<hr>".to_string(),
        }
    }
}
