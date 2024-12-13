use std::ops::{Bound, RangeBounds};

use itertools::Itertools;

use crate::token::{BlockToken, BlockType};

pub struct BlockLexer {
    tokens: Vec<BlockToken>,
    index: usize,
    content: Vec<String>,
}

impl BlockLexer {
    pub fn new(content: Vec<String>) -> Self {
        Self {
            content,
            index: 0,
            tokens: Vec::new(),
        }
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
            self.tokens[n - 1].proceed_block_content(self.content[self.index].clone());
        } else {
            let mut token = BlockToken::new(BlockType::Plain);
            token.proceed_block_content(self.content[self.index].clone());
            self.tokens.push(token);
        }

        self.next();
    }

    fn process_h1(&mut self) {
        let mut token = BlockToken::new(BlockType::h1);
        token.proceed_block_content(self.content[self.index][2..].to_string());
        self.tokens.push(token);
        self.next();
    }

    fn process_h2(&mut self) {
        let mut token = BlockToken::new(BlockType::h2);
        token.proceed_block_content(self.content[self.index][3..].to_string());
        self.tokens.push(token);
        self.next();
    }

    fn process_h3(&mut self) {
        let mut token = BlockToken::new(BlockType::h3);
        token.proceed_block_content(self.content[self.index][4..].to_string());
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

    fn process_codeblock<R: RangeBounds<usize>>(&mut self, range: R, language: String) {
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

        let code = self.content[start + 1..end - 1].iter().join("\n");
        let mut token = BlockToken::new(BlockType::CodeBlock);
        token.process_block_content_as_plain_text(code);
        token.process_block_content_as_plain_text(language);
        
        self.tokens.push(token);
        self.index = end;
    }

    fn process_quote(&mut self) {
        let mut prev = false; // 直前が>で始まっていたか？
        let mut quote_content = vec![];
        for i in self.index..self.content.len() {
            if self.content[i].is_empty() {
                // 問答無用で終了
                self.index = i + 1;
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
            token.proceed_block_content(s);
        }
        self.tokens.push(token);
    }

    fn process_footnote(&mut self) {
        let v = self.content[self.index].split(':').collect_vec();
        let id = v[0][2..v[0].len() - 1].to_string();
        let text = v[1..].iter().join(":");

        let mut token = BlockToken::new(BlockType::FootNote);
        // 1つ目がid, 2つ目がcontentということにしておく
        token.process_block_content_as_plain_text(id);
        token.proceed_block_content(text);
        self.tokens.push(token);
        self.next();
    }

    fn process_latex(&mut self, end: usize) {
        let latex = self.content[self.index + 1..end].iter().join("");
        let mut token = BlockToken::new(BlockType::Latex);
        token.process_block_content_as_plain_text(latex);
        self.tokens.push(token);
        self.index = end;
        self.next();
    }

    fn consume(&mut self) {
        'outer: while self.index < self.content.len() {
            if self.content[self.index].starts_with("# ") {
                // h1
                self.process_h1();
                continue;
            } else if self.content[self.index].starts_with("## ") {
                // h2
                self.process_h2();
                continue;
            } else if self.content[self.index].starts_with("### ") {
                // h3
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
                let language = self.content[self.index].split_off(3);
                for i in self.index+1..self.content.len() {
                    if self.content[i].trim_start().starts_with("```") {
                        self.process_codeblock(self.index..=i, language);
                        continue 'outer;
                    }
                }
            } else if self.content[self.index].starts_with(">") {
                // 引用
                self.process_quote();
                continue;
            } else if self.content[self.index].starts_with("[^") {
                let v = self.content[self.index].split("]:").collect_vec();
                if v.len() >= 2 && v[0].len() >= 3 {
                    // 始まりが[^(一文字以上) で、途中に]:があるのでfootnoteと判断
                    self.process_footnote();
                    continue;
                }
            } else if self.content[self.index].starts_with("$$") {
                for i in self.index + 1..self.content.len() {
                    if self.content[i].trim().ends_with("$$") {
                        self.process_latex(i);
                    }
                }
                continue;
            } else if self.content[self.index].trim().starts_with("<!--")
                && self.content[self.index].trim().ends_with("-->")
            {
                // 行単位のコメントアウトはスキップ
                self.next();
                continue;
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