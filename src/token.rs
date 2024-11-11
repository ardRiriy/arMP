use itertools::Itertools;

use crate::lexer::InlineLexer;

#[derive(Clone, Copy, Debug)]
pub enum InlineType {
    Text,
    Bold,
    Code,
    LineBreak,
    Url,
    FootNote,
    Latex,
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
    CodeBlock,
    Quote, // 引用
    FootNote,
    Latex,
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
            },
            InlineType::Url => {
                assert!(!self.children.is_empty());
                assert!(self.children[0].text.is_some());
                assert!(self.text.is_some());
                let content = self.text.clone().unwrap();
                let url = self.children[0].text.clone().unwrap();
                format!("<a href=\"{url}\">{content}</a>")
            },
            InlineType::FootNote => {
                assert!(self.text.is_some());
                let id = self.text.as_ref().unwrap();
                format!("<span id=\"{id}\"></span>")
            },
            InlineType::Latex => {
                assert!(self.text.is_some());
                format!("\\({}\\)", self.text.as_ref().unwrap())
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

    pub fn proceed_block_content(&mut self, content: String) {
        if !self.inline_tokens.is_empty() {
            self.inline_tokens.push(InlineToken::new(InlineType::LineBreak, None, None));
        }
        self.inline_tokens = [self.inline_tokens.clone(),
            InlineLexer::new(content.chars().collect()).tokenize()].iter()
            .flatten()
            .cloned()
            .collect();
    }

    pub fn process_block_content_as_plain_text(&mut self, content: String) {
        self.inline_tokens.push(InlineToken::new(InlineType::Text, Some(content), None));
    }

    pub fn to_html(&self) -> String {
        let content = self.inline_tokens
            .iter()
            .map(|it| it.to_html())
            .join("\n");
        match self.block_type {
            BlockType::h1 => format!("<h2>{content}</h2>"),
            BlockType::h2 => format!("<h3>{content}</h3>"),
            BlockType::h3 => format!("<h4>{content}</h4>"),
            BlockType::Plain => format!("<p>{content}</p>"),
            BlockType::Empty => "<br>".to_string(),
            BlockType::Hr => "<hr>".to_string(),
            BlockType::CodeBlock => format!("<pre><code class=\"codeblock\">{content}</code></pre>"),
            BlockType::Quote => format!("<blockquote>{content}</blockquote>"),
            BlockType::FootNote => {
                assert!(self.inline_tokens.len() >= 3);
                let id = self.inline_tokens[0].text.clone().unwrap();
                let text = self.inline_tokens[2..].iter().map(|tk| tk.to_html()).join("");
                format!("<foot-note for=\"{id}\">{text}</foot-note>")
            },
            BlockType::Latex => format!("\\[{content}\\]"),
        }
    }
}
