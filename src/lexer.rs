use itertools::Itertools;

use crate::token::{BlockToken, BlockType, InlineToken, InlineType};

#[derive(Debug)]
pub struct InlineLexer {
    text: Vec<char>,
    temprary: Vec<char>, // consumeしたtextをおいておく
    tokens: Vec<InlineToken>, // Token列
    index: usize,
}

impl InlineLexer {
    pub fn new(text: Vec<char>) -> Self {
        InlineLexer { text, temprary: Vec::new(), tokens: Vec::new(), index: 0 }
    }

    fn next(&mut self) {
        self.index += 1;
    }

    fn consume_str(&mut self) {
        self.temprary.push(self.text[self.index]);
        self.next();
    }

    fn process_tempary_str(&mut self) {
        if self.temprary.is_empty() {
            // 何もする必要がない
            return;
        }

        let text = self.temprary.iter().join("");
        
        let token = InlineToken::new(InlineType::Text, Some(text), None);
        self.tokens.push(token);
        self.temprary.clear();
    }

    // Bold(e.g. **hoge**)等のdecoratorが複数ある場合にindexがずれないようにend_of_decoratorを指定する
    fn process_decorator(&mut self, inline_type: InlineType ,l: usize, r: usize, end_of_decorator: usize) {
        let inline_text = self.text[l..r]
            .iter()
            .copied()
            .collect_vec();
        let children = InlineLexer::new(inline_text).tokenize();
        let token = InlineToken::new(inline_type, None, Some(children));
        self.tokens.push(token);
        self.index = end_of_decorator;
        self.next();
    }

    fn consume_inline_text(&mut self) {
        'outer: while self.index < self.text.len() {
            match self.text[self.index] {
                '*' => {
                    if self.index + 1 < self.text.len() && self.text[self.index + 1] == '*' {
                        // index + 2から最後まで、**となっている箇所があるかを判定
                        for i in self.index+2..self.text.len()-1 {
                            if self.text[i] == '*' && self.text[i+1] == '*' {
                                // temporaryをここで処理をしてしまう
                                self.process_tempary_str();

                                // [self.index+2, i)の区間を取り出して、その区間をLexerに掛ける
                                self.process_decorator(InlineType::Bold, self.index+2, i, i+1);
                                continue 'outer;
                            }
                        }
                    }
                    self.consume_str();
                }
                _ => {
                    self.consume_str();
                }
            }
        }
    }

    pub fn tokenize(&mut self) -> Vec<InlineToken> {
        self.consume_inline_text();
        // 最後に残ったtempraryをtextと処理して終了
        self.process_tempary_str();
        self.tokens.clone()
    }
}

pub struct BlockLexer {
    tokens: Vec<BlockToken>,
    index: usize,
    content: Vec<String>, 
}

impl BlockLexer {
    pub fn new(content: Vec<String>) -> Self {
        Self { content, index: 0, tokens: Vec::new() }
    }

    fn is_same_type(&self, other: BlockType) -> bool {
        if let Some(token) = self.tokens.last() {
            token.is_same_type(other)
        } else {
            false
        }
    }

    fn next(&mut self) {
        self.index += 1;
    }

    fn process_plain(&mut self) {
        if self.is_same_type(BlockType::Plain) {
            // 直前と同じトークンの場合は同じタイプに入れておく
            let n = self.tokens.len();
            self.tokens[n - 1].proceed_block_contest(self.content[self.index].clone());
        } else {
            let mut token = BlockToken::new(BlockType::Plain);
            token.proceed_block_contest(self.content[self.index].clone());
            self.tokens.push(token);
        }
        
        self.next();
    }

    fn process_h1(&mut self) {
        let mut token = BlockToken::new(crate::token::BlockType::h1);
        token.proceed_block_contest(self.content[self.index][2..].to_string());
        self.tokens.push(token);
        self.next();
    }

    fn process_h2(&mut self) {
        let mut token = BlockToken::new(crate::token::BlockType::h1);
        token.proceed_block_contest(self.content[self.index][3..].to_string());
        self.tokens.push(token);
        self.next();
    }

    fn process_h3(&mut self) {
        let mut token = BlockToken::new(crate::token::BlockType::h1);
        token.proceed_block_contest(self.content[self.index][4..].to_string());
        self.tokens.push(token);
        self.next();
    }


    fn process_empty(&mut self) {
        let mut token = BlockToken::new(crate::token::BlockType::Empty);
        token.proceed_block_contest("".to_string());
        self.tokens.push(token);
        // 2つ分の空行を消費したので2回next
        self.next();
        self.next();
    }

    fn consume(&mut self) {
        while self.index < self.content.len() {
            if self.content[self.index].starts_with("# ") { // h1
                self.process_h1();
                continue;
            } else if self.content[self.index].starts_with("## ") { // h2
                self.process_h2();
                continue;
            } else if self.content[self.index].starts_with("### ") { // h3
                self.process_h3();
                continue;
            } else if self.content[self.index].is_empty() {
                // 空行(段落分け or 無視)
                if self.index + 1 < self.content.len() && self.content[self.index + 1].is_empty() {
                    self.process_empty();
                    continue;
                } else {
                    // 完全な空行は無視してnext
                    self.next();
                    continue;
                }
            }

            // 何もないならplainとして処理
            self.process_plain();            
        }
    }

    pub fn tokenize(&mut self) -> Vec<BlockToken> {
        self.consume();
        self.tokens.clone()
    }
}
