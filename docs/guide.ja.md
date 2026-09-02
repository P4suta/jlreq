<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# `jlreq` 0.1.0 日本語利用ガイド

`jlreq` は、UTF-8本文とメモリ上のTTF/OTF/TTCを受け取り、描画順のグリフ、物理座標、
元本文のバイト範囲、使用フォント、診断をまとめた `TextLayout` を返します。字形の
ラスタライズ、PDF/GPUへの出力、描画自体はレンダラーの責任です。

このガイドのコード例はすべて完全なプログラムで、第一引数にフォントファイルの
パスを取ります。リポジトリのゲートが各例をfixtureフォントで実際にコンパイル・
実行するため、ここに載っているコードは常に現行APIで動きます。

## インストール

```sh
cargo add jlreq
```

```toml
[dependencies]
jlreq = "0.1"
```

`jlreq` の最低サポートRustは1.88です（`jlreq-core` は1.85）。クレートにフォントは
同梱されません。レイアウトはフォントの「バイト列」を受け取るので、
[Noto Sans JP](https://fonts.google.com/noto/specimen/Noto+Sans+JP) などを用意して
ください。OSフォント探索を使う場合だけ
`cargo add jlreq --features system-fonts` を指定します。

## 最短の利用手順

```rust
use jlreq::{FontLibrary, LayoutOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("フォントファイルを指定してください（例: NotoSansJP-Regular.otf）")?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;
    let layout = jlreq::layout("日本語組版", &fonts, LayoutOptions::try_new(240.0, 16.0)?)?;

    for glyph in layout.glyphs() {
        if let Some(font) = layout.font(glyph.font_id()) {
            // レンダラーが1グリフを描くのに必要な情報一式:
            let _draw = (
                font.bytes(),
                font.face_index(),
                glyph.glyph_id(),
                glyph.draw_origin(),
                glyph.font_size_26_6(),
                glyph.variations(),
                font.synthesis(),
                glyph.transform(),
            );
        }
    }
    Ok(())
}
```

完成したレイアウトは、参照する `FontResource` をすべて所有します。ただし、未使用の
登録フォントは保持しないため、IDは `0, 1, 2, ...` と連続するとは限りません。
`layout.font(glyph.font_id())` で検索し、`layout.fonts()[id]` のような添字アクセスは
行わないでください。別の `FontLibrary` で発行されたIDは、たとえ添字が範囲内でも
`None` になります（取り違えは無音で誤フォントを返す代わりに検出されます）。

## 描画契約

レンダラーは、各グリフについて次の情報をそのまま利用できます。

- `draw_origin`: シェーパーのオフセットを反映済みの物理描画原点
- `font_size` / `font_size_26_6`: シェーピングに用いた実効文字サイズ
- `variations`: グローバル値、システムフォント既定値、span値をタグ単位で統合した軸
- `FontResource::synthesis`: 可変軸では表せない合成太字と傾斜
- `FontResource::metrics`: em比のascent/descent/x-height/cap height/下線位置と太さ。
  下線・取り消し線・ベースライン合わせに使い、実効サイズを掛けてレイアウト単位へ
  換算します
- `transform`: 縦書き回転または縦中横の局所変換

同じrunのグリフは実効軸の配列を共有します。軸値は公開境界で26.6固定小数へ量子化
され、同じ固定小数値は `Eq` と `Hash` でも同一です。OpenTypeタグは4バイトの
`0x20..=0x7e` で、空白は末尾のパディングにだけ使えます。

## セル境界とインク境界

`GlyphPlacement::cell_bounds`、`TextLine::bounds`、`TextLayout::bounds` は、組版セルの
物理境界です。空白のadvance、ルビなどの注釈セル、空行のblock extentを含みます。
これは字形アウトラインのインク境界ではありません。クリッピング、衝突判定、装飾線
をインクに合わせる場合は、選択したface、サイズ、軸、合成状態からレンダラー側で
アウトライン境界を取得してください。

## 組版ポリシー（Style）

JLReq 2020 が選択肢として示すすべての項目 — 禁則の厳しさ、ぶら下げ、ルビの
はみ出し、詰め伸ばしの優先順など — は、型付きの `Style` 1値にまとまっています。
`Style::jlreq_2020()`（既定）のほか、`book_2020()`、`magazine_2020()`、
`newspaper_2020()`、`jis_reading_2020()` の公開プロファイルがあり、
`StyleBuilder` で個別の選択だけを上書きできます。

```rust no_run
use jlreq::{FontLibrary, LayoutOptions, Style};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("フォントファイルを指定してください")?;
    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;

    let book = jlreq::layout(
        "行末の約物処理はプロファイルで変わります。",
        &fonts,
        LayoutOptions::try_new(200.0, 16.0)?.with_style(Style::book_2020()),
    )?;
    let _ = book.lines().len();
    Ok(())
}
```

## span、縦書き、9つの行内構造

`DocumentBuilder` はUTF-8バイト範囲に `SpanStyle` を設定し、明示改行（必須・任意・
禁止）、9つのJLReq構造を型付きで受け付けます。ルビ・圏点・合印・添字の注釈文字列は
自動でシェーピングされます。振分けは範囲内のクラスタを自動で均等割りするので、
分割位置を自分で決めたいときだけ `mandatory_break` を範囲内に置きます。

| 構造 | ビルダーメソッド |
| --- | --- |
| モノ／グループ／熟語ルビ | `mono_ruby`, `group_ruby`, `jukugo_ruby`, 明示runの `ruby` |
| 縦中横 | `tate_chu_yoko` |
| 圏点 | `emphasis_dots` |
| 割注 | `warichu` |
| 振分け | `furawake` |
| 字取り | `jidori` |
| 合印 | `reference_mark` |
| 添字 | `script`（上付き・下付きは配置に反映されます） |
| 数式 | `formula` |

```rust no_run
use jlreq::{DocumentBuilder, FontLibrary, LayoutOptions, SpanStyle, WritingMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("フォントファイルを指定してください")?;

    let mut document = DocumentBuilder::new("漢字12\n次段落");
    document.span(0..6, SpanStyle::new().with_font_size(24.0)?)?;
    document.group_ruby(0..6, "かんじ")?;
    document.tate_chu_yoko(6..8)?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;
    let _layout = jlreq::layout_document(
        &document.build()?,
        &fonts,
        LayoutOptions::try_new(240.0, 16.0)?
            .with_writing_mode(WritingMode::VerticalRl)
            .with_line_gap(2.0)?,
    )?;
    Ok(())
}
```

`SpanStyle` は書体（family）、サイズ、言語、OpenType機能・可変軸に加えて、意味役割
（`TextRole`: 小数点、位取り、文中・文末の区切り約物など。`Plain` は推論自体を
止めます）と仮想ボディ（`MetricsFrame`: 全角・プロポーショナル・半角。既定の
ヒューリスティクスが判定しない文字種の逃げ道です）を指定できます。

## 段落スタイル（字下げ・版面・そろえ・widow）

`ParagraphStyle` は、範囲が完全に含む各段落へ、版面（行長）、そろえ、組版ポリシー、
字下げ、段落末の孤立行（widow）方針、タブストップを個別に上書きします。文書全体の
既定値は `LayoutOptions` 側の `with_first_line_indent` / `with_widow` /
`with_tab_stops` です。

```rust no_run
use jlreq::{Alignment, DocumentBuilder, FontLibrary, LayoutOptions, ParagraphStyle, Widow};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("フォントファイルを指定してください")?;

    let text = "見出し\n本文の一段落目です。\n引用の段落です。";
    let heading = 0..9;
    let body = 10..40;
    let quote = 41..text.len();

    let mut document = DocumentBuilder::new(text);
    document.paragraph_style(
        heading,
        ParagraphStyle::new().with_alignment(Alignment::Center),
    )?;
    document.paragraph_style(
        body,
        ParagraphStyle::new()
            .with_first_line_indent(16.0)?
            .with_widow(Widow::MinimumClusters(2)),
    )?;
    document.paragraph_style(quote, ParagraphStyle::new().with_line_extent(240.0)?)?;

    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;
    let layout = jlreq::layout_document(
        &document.build()?,
        &fonts,
        LayoutOptions::try_new(320.0, 16.0)?,
    )?;
    for line in layout.lines() {
        let _ = (line.paragraph_index(), line.is_first_in_paragraph());
    }
    Ok(())
}
```

## タブ組（小数点そろえを含む）

`with_tab_width` の等間隔ラダーに加えて、`TabStop` で明示位置と4種のそろえ
（開始・中央・終了・指定文字＝小数点そろえ）を指定できます。単位は他の長さと同じ
量子化済み `f32` です。

```rust no_run
use jlreq::{FontLibrary, LayoutOptions, TabAlignment, TabStop};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("フォントファイルを指定してください")?;
    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;

    let stops = [
        TabStop::try_new(96.0, TabAlignment::Start)?,
        TabStop::try_new(200.0, TabAlignment::Character('.'))?,
    ];
    let layout = jlreq::layout(
        "A\t3.14",
        &fonts,
        LayoutOptions::try_new(320.0, 16.0)?.with_tab_stops(stops),
    )?;
    let _ = layout.glyphs().count();
    Ok(())
}
```

## フォント登録とフォールバック

`register_font` はフォント自身の `name` テーブルから書体名を導出するので、そのまま
`SpanStyle::with_family` の対象になります。明示のメタデータが必要なときは
`register_face` を使ってください。primaryは `.notdef` の供給元になり、
`set_fallback_order` で優先順を固定できます。拡張書記素（異体字シーケンスを含む）を
丸ごとカバーする最初のフォントが選ばれ、どれも覆えない場合は範囲を保持したまま
primaryの `.notdef` が出力され、`font.missing-glyph` が診断として報告されます。
どのフォントも名乗らない書体を要求したspanには `font.unknown-family` が報告されます。

## 編集UI（キャレット・選択・単語）

`hit_test` は `(byte_offset, affinity)` を返します。caretを戻すときは両方を
`caret_rect` に渡してください。`Affinity` を省くと、折返し位置、改行、bidi境界の
どちら側かを一意に決められません。レイアウトはエディタ操作一式も備えます。

- `next_visual_caret` / `prev_visual_caret`: 見た目の並び順で1つ進む・戻る
  （bidiでも視覚順、行末では次の行へ続きます）
- `caret_previous_line` / `caret_next_line`: インライン位置を保ったまま隣の行へ
  （縦書きでは隣の列に相当します）
- `next_grapheme_boundary` / `prev_grapheme_boundary`: 結合文字や絵文字を壊さない
  カーソル単位
- `word_range_at` / `sentence_range_at`: 辞書ベースの分かち書きで、日本語の
  ダブルクリック選択がそのまま動きます
- `GlyphPlacement::construct`: グリフが属する構造の序数。`Document::construct` と
  突き合わせれば「ルビごと選択」が1回の照会で書けます
- `selection_rects` は視覚順で連続する選択runごとの矩形（bidiの未選択領域を覆わ
  ない厳密形）、`selection_rects_filled` は行末まで塗るエディタ向けの形です

```rust no_run
use jlreq::{FontLibrary, LayoutOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_path = std::env::args()
        .nth(1)
        .ok_or("フォントファイルを指定してください")?;
    let mut fonts = FontLibrary::new();
    fonts.register_font(std::fs::read(font_path)?)?;
    let layout = jlreq::layout(
        "これは日本語の文章です。",
        &fonts,
        LayoutOptions::try_new(160.0, 16.0)?,
    )?;

    let hit = layout.hit_test_xy(20.0, 8.0)?;
    let caret = layout.caret_rect(hit.byte_offset(), hit.affinity());
    let word = layout.word_range_at(hit.byte_offset());
    let next = layout.next_visual_caret(hit.byte_offset(), hit.affinity());
    let below = layout.caret_next_line(hit.byte_offset(), hit.affinity());
    let filled = layout.selection_rects_filled(0..layout.source().len());
    let _ = (caret, word, next, below, filled);
    Ok(())
}
```

## エラーと診断

不正な入力や資源上限は部分結果なしの `LayoutError` になり、`code()`（安定した機械可読
コード）、`message()`（安定した一文の説明）、`range()`（責任範囲）を持ちます。完全な
レイアウトと両立する事象 — グリフ欠落、行あふれ、widow、未知の書体名 — は
`TextLayout::diagnostics` に載ります。コードの一覧は
[`error-codes.md`](error-codes.md) に固定されています。

## 決定論の境界

明示的なフォントbytes、face index、本文、オプションが同じなら、対応OS間で26.6結果は
bit単位で同一です。`system-fonts` による探索はOSのフォント集合とFontiqueの結果に
依存するため、この保証の外です。ただし登録時に選ばれたbytes、face index、既定軸、
合成状態は `FontResource` にコピーされ、その完成レイアウトだけで同じ外観を再現
できます。キャッシュの有無、`layout` と再利用可能な `LayoutEngine` の違いは結果に
影響しません。

`jlreq-core` は引き続き依存なしの `no_std + alloc` 固定小数組版層です。既に
シェーピング済みのclusterを持つ実装だけが直接利用し、通常のアプリケーションは
高水準の `jlreq` APIを利用してください。

エラーコードは [`error-codes.md`](error-codes.md)、公開名は
[`public-api.toml`](public-api.toml)、設計上の境界は
[`design/api-spine.md`](design/api-spine.md) に固定されています。実行可能な
サンプルは [`crates/jlreq/examples/`](../crates/jlreq/examples/) にあります。
