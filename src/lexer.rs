use std::ops::{Bound, RangeBounds};

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

    fn process_inline_code(&mut self, end_of_decorator: usize) {
        // inline codeの中身はすべてplain textとして処理したいので別扱い
        let inline_text = if self.index + 1 == end_of_decorator {
            "".to_string()
        } else {
            self.text[self.index+1..end_of_decorator-1]
                .iter()
                .copied()
                .collect()
        };

        let token = InlineToken::new(InlineType::Code, Some(inline_text), None);
        self.tokens.push(token);
        self.index = end_of_decorator;
        self.next();
    }
    
    fn process_external_url(&mut self, end_of_decorator: usize) {
        if self.index + 1 == end_of_decorator {
            // "[]" という形で中身に何もない場合はtemporaryに突っ込んで終了しておく 空文字列のURLは意味がないので
            self.temprary.push('[');
            self.temprary.push(']');
            self.index = end_of_decorator;
            self.next();
        } else {
            let display_text = self.text[self.index+1..end_of_decorator]
                .iter()
                .copied()
                .join("");
            // 後続にURLが続くことを期待して処理を続ける
            // なお、続かない場合はURLを空にして処理をする
            self.index = end_of_decorator;
            self.next();
            let mut end_of_decorator = self.index;

            let mut url = "".to_string();
            if self.index + 1 < self.text.len() && self.text[self.index] == '(' {
                for (i, &c) in self.text[self.index+1..].iter().enumerate() {
                    if c == ')' {
                        end_of_decorator = self.index+1+i;
                        if i != 0 {
                            url = self.text[self.index+1..self.index+1+i].iter()
                                .copied()
                                .join("");
                            break;
                        }
                    }
                }
            }
            let token = InlineToken::new(
                InlineType::Url,
                Some(display_text),
                Some(
                    vec![
                        InlineToken::new(
                            InlineType::Text,
                            Some(url),
                            None
                        )
                    ]
                )
             );
            self.tokens.push(token);
            self.index = end_of_decorator;
            self.next();
        }
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
                '`' => { // backquote: inline code
                    for i in self.index+1..self.text.len() {
                        if self.text[i] == '`' {
                            self.process_tempary_str();
                            self.process_inline_code(i);
                            continue 'outer;
                        }
                    }
                }
                '\\' => { // backslash: 次の文字を強制的にconsumeする。文末にある場合は無視。
                    if self.index + 1 < self.text.len() {
                        self.next();
                        self.consume_str();
                    }

                }
                '[' => {
                    // TODO: obsidianの[[]]とURLの[]()で読み替えないといけない
                    // 一旦外部URLのみをパースする
                    for i in self.index+1..self.text.len() {
                        if self.text[i] == ']' {
                            self.process_tempary_str();
                            self.process_external_url(i);
                            continue 'outer;
                        }
                    }
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
        let mut token = BlockToken::new(BlockType::h1);
        token.proceed_block_contest(self.content[self.index][2..].to_string());
        self.tokens.push(token);
        self.next();
    }

    fn process_h2(&mut self) {
        let mut token = BlockToken::new(BlockType::h2);
        token.proceed_block_contest(self.content[self.index][3..].to_string());
        self.tokens.push(token);
        self.next();
    }

    fn process_h3(&mut self) {
        let mut token = BlockToken::new(BlockType::h3);
        token.proceed_block_contest(self.content[self.index][4..].to_string());
        self.tokens.push(token);
        self.next();
    }

    fn process_empty(&mut self) {
        let token = BlockToken::new(BlockType::Empty);
        self.tokens.push(token);
        // 2つ分の空行を消費したので2回next
        self.next();
        self.next();
    }

    fn process_hr(&mut self) {
        let token = BlockToken::new(BlockType::Hr);
        self.tokens.push(token);
        self.next();
    }

    fn process_codeblock<R: RangeBounds<usize>>(&mut self, range: R) {
        let start = match range.start_bound() {
            Bound::Unbounded => 0,
            Bound::Excluded(&s) => s + 1,
            Bound::Included(&s) => s,
        };
        let end = match range.end_bound() {
            Bound::Unbounded => self.content.len(),
            Bound::Excluded(&t) => t.min(self.content.len()),
            Bound::Included(&t) => (t + 1).min(self.content.len()),
        };

        let code = self.content[start+1..end-1].iter().join("\n");
        let mut token = BlockToken::new(BlockType::CodeBlock);
        token.process_block_content_as_plain_text(code);
        self.tokens.push(token);
        self.index = end;
    }

    fn process_quote(&mut self) {
        let mut prev = false; // 直前が>で始まっていたか？
        let mut quote_content = vec![];
        for i in self.index..self.content.len() {
            if self.content[i].is_empty() {
                // 問答無用で終了
                self.index = i+1;
                break;
            } else if self.content[i].starts_with(">") {
                quote_content.push(self.content[i][1..].trim().to_string());
                prev = true;
            } else if prev {
                quote_content.push(self.content[i].to_string());
                prev = false;
            } else {
                self.index = i;
                break;
            }
        }
        let mut token = BlockToken::new(BlockType::Quote);
        for s in quote_content {
            token.proceed_block_contest(s);
        }
        self.tokens.push(token);
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
            } else if self.content[self.index].starts_with("---") {
                // 多分実用上困らない...はず
                self.process_hr();
                continue;
            } else if self.content[self.index].starts_with("```") {
                for i in self.index+1..self.content.len() {
                    if self.content[i].trim_start().starts_with("```") {
                        // TODO: 言語対応
                        self.process_codeblock(self.index..=i);
                    }
                }
            } else if self.content[self.index].starts_with(">") {
                // 引用
                self.process_quote();
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
