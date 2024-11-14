# This is test Content
koreha line 1
**koreha line 2**(strong line)

koreha line 3(not new paragraph)


koreha line 4(new paragraph)

## This is h2
this is not strong: \**not strong**
backslask in plaintext is available!: \\

### This is h3
inline code test here: `v.iter().join(" ");`testetste
this is no content code: ``aaa
---
```rust
fn main(){
    println!("Hello World!");
}
```
this is plain text

> This is in the quote.
This is also in the quote.
> **This is also.**

This is not in the quote.
Follow me!: [my twitter account](https://x.com/ardririy)
this is url but url is empty : [not url!!]
this is url but url is empty[^test]: [empty url]() hahaha
this is not url: [](https://x.com/ardririy)

([hoge](https://x.com/ardririy))aiueoaiueo

数式: $E = mc^2$

以下、[ABC378E - Mod Sigma Problem](https://atcoder.jp/contests/abc378/tasks/abc378_e)の式変形。
$$
\begin{align}
\sum_{i=1}^N \sum_{j=i}^N (S_j - S_{i-1})\mod  M &= \sum_{i=1}^N \sum_{j=i}^N S_j - S_{i-1} + \begin{cases} 0 &\text{if } S_j \geq S_{i-1} \\ M & \text{otherwise} \end{cases} \\
&= \sum_{i=1}^N (\sum_{j=1}^N S_j - \sum_{j=1}^N S_{i-1} + kM )
\end{align}
$$

<!-- これはコメントなので表示しないでね -->

[[Python3.13がリリースされてるので手元でビルドとかしてみる]] ← 内部リンク

[^test]: 遷移先のないURLは自身へ戻ってきます。
