use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use itertools::Itertools;

use crate::{
    token::{InlineToken, InlineType},
    util::get_path,
};

#[derive(Debug)]
pub struct InlineLexer {
    text: Vec<char>,
    temprary: Vec<char>,      // consumeしたtextをおいておく
    tokens: Vec<InlineToken>, // Token列
    index: usize,
}

impl InlineLexer {
    pub fn new(text: Vec<char>) -> Self {
        InlineLexer {
            text,
            temprary: Vec::new(),
            tokens: Vec::new(),
            index: 0,
        }
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
    fn process_decorator(
        &mut self,
        inline_type: InlineType,
        l: usize,
        r: usize,
        end_of_decorator: usize,
    ) {
        let inline_text = self.text[l..r].iter().copied().collect_vec();
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
            self.text[self.index + 1..end_of_decorator]
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
            let display_text = self.text[self.index + 1..end_of_decorator]
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
                for (i, &c) in self.text[self.index + 1..].iter().enumerate() {
                    if c == ')' {
                        end_of_decorator = self.index + 1 + i;
                        if i != 0 {
                            url = self.text[self.index + 1..self.index + 1 + i]
                                .iter()
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
                Some(vec![InlineToken::new(InlineType::Text, Some(url), None)]),
            );
            self.tokens.push(token);
            self.index = end_of_decorator;
            self.next();
        }
    }

    fn process_footnote(&mut self, id: String) {
        let token = InlineToken::new(InlineType::FootNote, Some(id), None);
        self.tokens.push(token);
        self.next();
    }

    fn process_latex(&mut self) {
        self.process_tempary_str();
        // 後ろの$を探す
        for i in self.index + 1..self.text.len() {
            if self.text[i] == '$' {
                if self.index + 1 != i {
                    let tex = self.text[self.index + 1..i].iter().join("");
                    let token = InlineToken::new(InlineType::Latex, Some(tex), None);
                    self.tokens.push(token);
                    self.index = i;
                }
                self.next();
                return;
            }
        }
    }

    fn process_picture(&mut self, end_of_decorator: usize, path: String) {
        self.process_tempary_str();
        let token = InlineToken::new(InlineType::Picture, Some(path), None);
        self.tokens.push(token);
        self.index = end_of_decorator;
        self.next();
    }

    fn consume_bracket(&mut self) {
        self.process_tempary_str();
        // TODO: obsidianの[[]]とURLの[]()と脚注の[^*]で読み替えないといけない
        if self.index + 1 < self.text.len() {
            match self.text[self.index + 1] {
                '^' => {
                    // この場合は脚注
                    let mut text = vec![];
                    for i in self.index + 2..self.text.len() {
                        if self.text[i] == ']' {
                            assert!(!text.is_empty()); // 対応が面倒くさいのでパースエラーということにしておく
                            self.index = i;
                            self.process_footnote(text.iter().join(""));
                            return;
                        }
                        text.push(self.text[i]);
                    }
                }
                '[' => {
                    // 後ろに"]]"のテキストがあれば内部リンクで対応
                    let mut link = vec![];
                    let mut prev = false; // 直前が]だったか？
                    for i in self.index + 2..self.text.len() {
                        if self.text[i] == ']' {
                            if prev {
                                let mut flag = false; // 対応するurlが存在したか？
                                if let Some(path) = get_path(link.iter().join("")) {
                                    let file = File::open(path);
                                    if let Ok(file) = file {
                                        let reader = BufReader::new(file);
                                        let first_line = reader.lines().next();
                                        if let Some(Ok(line)) = first_line {
                                            if line.starts_with("<!-- url: ")
                                                && line.trim().ends_with("-->")
                                            {
                                                let trimed = line
                                                    .trim_start_matches("<!-- url:")
                                                    .trim_end_matches("-->")
                                                    .trim()
                                                    .to_string();
                                                if !trimed.is_empty() {
                                                    let token = InlineToken::new(
                                                        InlineType::Url,
                                                        Some(link.iter().join("")),
                                                        Some(vec![InlineToken::new(
                                                            InlineType::Text,
                                                            Some(format!("article/{trimed}")),
                                                            None,
                                                        )]),
                                                    );
                                                    flag = true;
                                                    self.tokens.push(token);
                                                }
                                            }
                                        }
                                    }
                                }
                                if !flag {
                                    // 処理されなかった場合はlink部分をplainなtextにする
                                    // とはいいつつ、tempraryに突っ込んでおけば後でよしなにしてくれる
                                    self.temprary.extend(link);
                                }
                                self.index = i;
                                self.next();
                                return;
                            }
                            prev = true;
                        } else {
                            link.push(self.text[i]);
                        }
                    }
                }
                _ => { /* 外部URLだと思って次へ飛ばす */ }
            }
        }

        for i in self.index + 1..self.text.len() {
            if self.text[i] == ']' {
                self.process_external_url(i);
                break;
            }
        }
    }

    fn consume_inline_text(&mut self) {
        'outer: while self.index < self.text.len() {
            match self.text[self.index] {
                '*' => {
                    if self.index + 1 < self.text.len() && self.text[self.index + 1] == '*' {
                        // index + 2から最後まで、**となっている箇所があるかを判定
                        for i in self.index + 2..self.text.len() - 1 {
                            if self.text[i] == '*' && self.text[i + 1] == '*' {
                                // temporaryをここで処理をしてしまう
                                self.process_tempary_str();

                                // [self.index+2, i)の区間を取り出して、その区間をLexerに掛ける
                                self.process_decorator(InlineType::Bold, self.index + 2, i, i + 1);
                                continue 'outer;
                            }
                        }
                    }
                    self.consume_str();
                }
                '`' => {
                    // backquote: inline code
                    for i in self.index + 1..self.text.len() {
                        if self.text[i] == '`' {
                            self.process_tempary_str();
                            self.process_inline_code(i);
                            continue 'outer;
                        }
                    }
                }
                '\\' => {
                    // backslash: 次の文字を強制的にconsumeする。文末にある場合は無視。
                    if self.index + 1 < self.text.len() {
                        self.next();
                        self.consume_str();
                    }
                }
                '[' => {
                    self.consume_bracket();
                }
                '$' => {
                    // 数式
                    self.process_latex();
                }
                '!' => {
                    // 画像
                    if self.index + 2 < self.text.len()
                        && self.text[self.index + 1] == '['
                        && self.text[self.index + 2] == '['
                    {
                        let mut path = vec![];
                        for i in self.index + 3..self.text.len() - 1 {
                            if self.text[i] == ']' && self.text[i + 1] == ']' {
                                self.process_picture(i + 1, path.iter().join(""));
                                continue 'outer;
                            } else {
                                path.push(self.text[i]);
                            }
                        }
                        self.consume_str();
                    } else {
                        self.consume_str();
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
