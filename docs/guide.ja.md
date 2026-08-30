<!--
SPDX-FileCopyrightText: 2026 jlreq contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# `jlreq` 0.1.0 日本語利用ガイド

`jlreq` は、UTF-8本文とメモリ上のTTF/OTF/TTCを受け取り、描画順のグリフ、物理座標、
元本文のバイト範囲、使用フォント、診断をまとめた `TextLayout` を返します。字形の
ラスタライズ、PDF/GPUへの出力、描画自体はレンダラーの責任です。

## 最短の利用手順

```rust,no_run
# fn example(font_bytes: Vec<u8>) -> Result<(), jlreq::LayoutError> {
use jlreq::{FontLibrary, LayoutOptions};

let mut fonts = FontLibrary::new();
fonts.register_font(font_bytes)?;
let layout = jlreq::layout(
    "日本語組版",
    &fonts,
    LayoutOptions::try_new(240.0, 16.0)?,
)?;

for glyph in layout.glyphs() {
    if let Some(font) = layout.font(glyph.font_id()) {
        let draw_data = (
            font.bytes(),
            font.face_index(),
            glyph.glyph_id(),
            glyph.draw_origin(),
            glyph.font_size_26_6(),
            glyph.variations(),
            font.synthesis(),
            glyph.transform(),
        );
        let _ = draw_data;
    }
}
# Ok(())
# }
```

完成したレイアウトは、参照する `FontResource` をすべて所有します。ただし、未使用の
登録フォントは保持しないため、IDは `0, 1, 2, ...` と連続するとは限りません。
`layout.font(glyph.font_id())` で検索し、`layout.fonts()[id]` のような添字アクセスは
行わないでください。

## 描画契約

レンダラーは、各グリフについて次の情報をそのまま利用できます。

- `draw_origin`: シェーパーのオフセットを反映済みの物理描画原点
- `font_size` / `font_size_26_6`: シェーピングに用いた実効文字サイズ
- `variations`: グローバル値、システムフォント既定値、span値をタグ単位で統合した軸
- `FontResource::synthesis`: 可変軸では表せない合成太字と傾斜
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

## span、縦書き、注釈

`DocumentBuilder` はUTF-8バイト範囲に `SpanStyle` を設定し、明示改行、ルビ、縦中横、
圏点、割注、振分け、字取り、合印、添字、数式を型付きで受け付けます。大きなspanや
注釈がある行の後続段落は、実際の行セルと `line_gap` から配置されるため、横書き・
縦書きのどちらでも重なりません。

```rust,no_run
# fn vertical(font_bytes: Vec<u8>) -> Result<(), jlreq::LayoutError> {
use jlreq::{DocumentBuilder, FontLibrary, LayoutOptions, SpanStyle, WritingMode};

let mut document = DocumentBuilder::new("漢字12\n次段落");
document.span(0..6, SpanStyle::new().font_size(24.0)?)?;
document.group_ruby(0..6, "かんじ")?;
document.tate_chu_yoko(6..8)?;

let mut fonts = FontLibrary::new();
fonts.register_font(font_bytes)?;
let _layout = jlreq::layout_document(
    &document.build()?,
    &fonts,
    LayoutOptions::try_new(240.0, 16.0)?
        .writing_mode(WritingMode::VerticalRl)
        .line_gap(2.0)?,
)?;
# Ok(())
# }
```

## 編集UI

`hit_test` は `(byte_offset, affinity)` を返します。caretを戻すときは両方を
`caret_rect` に渡してください。`Affinity` を省くと、折返し位置、改行、bidi境界の
どちら側かを一意に決められません。空行へのヒットは、その空行自身のUTF-8位置へ
写像されます。`selection_rects` は行全体を一つに塗らず、視覚順で連続する選択run
ごとの矩形を返すため、bidi本文の未選択領域を覆いません。

```rust,no_run
# fn editing(layout: &jlreq::TextLayout) -> Result<(), jlreq::LayoutError> {
let hit = layout.hit_test_xy(20.0, 30.0)?;
let caret = layout.caret_rect(hit.byte_offset(), hit.affinity());
let selected = layout.selection_rects(0..layout.source().len());
let _ = (caret, selected);
# Ok(())
# }
```

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
[`design/api-spine.md`](design/api-spine.md) に固定されています。
