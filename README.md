# arMP: ardririy's Markdown Parser
arMPはObsidian向けのマークダウンパーサです。基本的にはよくあるマークダウンの記法に従って構文解析を行いHTMLを生成しますが、Obsidianに独自の内部リンクを解決する機能を持っています。

## installation
armpはCrates.ioに登録**されていません**。このレポジトリをクローンして`cargo install --path .`などでインストールするか、ビルドを行ったバイナリに対してパスを通すなどしてください。現在のリリース(v0.0.1)は最新のコミットを反映していません。

## usage
実行前に、公開するマークダウンが置かれているディレクトリを環境変数`KNOWLEDGES`に登録してください。これは内部リンクの解決のために使用されますが、もし不要な場合は適当なパスを指定すれば良いです。
例:
```
export KNOWLEDGES="/home/username/repos/knowledges"
```

登録を行ったのち、以下のように実行するとHTMLが標準出力に吐き出されます。
```shell
$ armp <markdonw filepath>
```
もしファイルに保存したい場合は`1> <output file>`をつけてください。

