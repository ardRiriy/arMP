use std::{fs::File, io::{BufRead, BufReader}, ops::{Bound, RangeBounds}};

use itertools::Itertools;

use crate::{token::{BlockToken, BlockType, InlineToken, InlineType}, util::get_path};

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
            self.text[self.index+1..end_of_decorator]
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

    fn process_footnote(&mut self, id: String) {
        let token = InlineToken::new(
            InlineType::FootNote,
            Some(id),
            None
        );
        self.tokens.push(token);
        self.next();
    }

    fn process_latex(&mut self) {
        self.process_tempary_str();
        // 後ろの$を探す
        for i in self.index+1..self.text.len() {
            if self.text[i] == '$' {
                if self.index + 1 != i {
                    let tex = self.text[self.index+1..i].iter().join("");
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
                    for i in self.index+2..self.text.len() {
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
                    for i in self.index+2..self.text.len() {
                        if self.text[i] == ']' {
                            if prev {
                                let mut flag = false; // 対応するurlが存在したか？
                                if let Some(path) = get_path(link.iter().join("")) {
                                    let file = File::open(path);
                                    if let Ok(file) = file {
                                        let reader = BufReader::new(file);
                                        let first_line = reader.lines().next();
                                        if let Some(Ok(line)) = first_line {
                                            if line.starts_with("<!-- url: ") && line.trim().ends_with("-->") {
                                                let trimed = line.trim_start_matches("<!-- url:")
                                                    .trim_end_matches("-->")
                                                    .trim()
                                                    .to_string();
                                                if !trimed.is_empty() {
                                                    let token = InlineToken::new(
                                                        InlineType::Url,
                                                        Some(link.iter().join("")),
                                                    Some(vec![InlineToken::new(InlineType::Text, Some(format!("article/{trimed}")), None)])
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
                _ => { /* 外部URLだと思って次へ飛ばす */}
            }
        }

        for i in self.index+1..self.text.len() {
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
                    self.consume_bracket();
                }
                '$' => {
                    // 数式
                    self.process_latex();
                }
                '!' => {
                    // 画像
                    if self.index+2 < self.text.len() && self.text[self.index+1] == '[' && self.text[self.index+2] == '[' {
                        let mut path = vec![];
                        for i in  self.index+3..self.text.len()-1 {
                            if self.text[i] == ']' && self.text[i+1] == ']' {
                                self.process_picture(i+1, path.iter().join(""));
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
            token.proceed_block_content(s);
        }
        self.tokens.push(token);
    }

    fn process_footnote(&mut self) {
        let v = self.content[self.index].split(':').collect_vec();
        let id = v[0][2..v[0].len()-1].to_string();
        let text = v[1..].iter().join(":");

        let mut token = BlockToken::new(BlockType::FootNote);
        // 1つ目がid, 2つ目がcontentということにしておく
        token.process_block_content_as_plain_text(id);
        token.proceed_block_content(text);
        self.tokens.push(token);
        self.next();
    }

    fn process_latex(&mut self, end: usize) {
        let latex = self.content[self.index+1..end].iter().join("");
        let mut token = BlockToken::new(BlockType::Latex);
        token.process_block_content_as_plain_text(latex);
        self.tokens.push(token);
        self.index = end;
        self.next();
    }

    fn consume(&mut self) {
        'outer: while self.index < self.content.len() {

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
                        self.process_codeblock(self.index..=i);
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
                for i in self.index+1..self.content.len() {
                    if self.content[i].trim().ends_with("$$") {
                        self.process_latex(i);
                    }
                }
                continue;
            } else if self.content[self.index].trim().starts_with("<!--") && self.content[self.index].trim().ends_with("-->") {
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
