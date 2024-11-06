use itertools::Itertools;

use crate::token::{InlineType, Token};

#[derive(Debug)]
pub struct InlineLexer {
    text: Vec<char>,
    temprary: Vec<char>, // consumeしたtextをおいておく
    tokens: Vec<Token>, // Token列
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
        
        let token = Token::new(InlineType::Text, Some(text), None);
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
        let token = Token::new(inline_type, None, Some(children));
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

    pub fn tokenize(&mut self) -> Vec<Token> {
        self.consume_inline_text();
        // 最後に残ったtempraryをtextと処理して終了
        self.process_tempary_str();
        self.tokens.clone()
    }
}