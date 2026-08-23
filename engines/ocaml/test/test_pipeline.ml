(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** {!Jlreq.Pipeline}: the cost function's boundaries, the validation boundary,
    line adjustment, and one paragraph composed end to end.

    The cost constants are checked here because they are the part of the engine
    with no external source: they are not in JLReq, not in [spec/], and not in any
    design note. A conformance case that breaks in the wrong place would say only
    that the paragraph came out wrong; these say which constant moved.

    §3.8.4's expansion ladder is checked here for a related reason. The eighty-nine
    built-in cases reach it six times and every one of those six lines has exactly
    one boundary, which is the one shape in which the ladder's stages and their
    ceilings are indistinguishable from handing the whole shortfall to the only
    place that will take it. Every check below is a line with two or three. *)

open Jlreq

let em = { Model.inline = 1000; Model.block = 1000 }

let cluster ?size ?frame ?role first last advance =
  {
    Model.first;
    Model.last;
    Model.advance;
    Model.size_override = size;
    Model.frame_override = frame;
    Model.role;
  }

let text ?(size = em) ?(frame = Model.Full_em) source clusters =
  { Model.source; Model.size; Model.frame; Model.clusters = Array.of_list clusters }

(* ----------------------------------------------------------------------------- *)
(* Line adjustment *)
(* ----------------------------------------------------------------------------- *)

type piece = {
  at : string;  (** The cluster's own source text. *)
  advance : int;
  own_frame : Model.frame option;
  own_role : Model.role option;
  own_size : Model.size option;
}

let p ?frame ?role ?size ?(advance = 1000) at =
  { at; advance; own_frame = frame; own_role = role; own_size = size }

let kanji = "\xe2\x80\xbb" (* U+203B REFERENCE MARK, cl-19 *)
let ideographic_space = "\xe3\x80\x80" (* U+3000, cl-14 *)
let katakana = "\xe3\x82\xa2" (* U+30A2 KATAKANA LETTER A, cl-16 *)
let unit_sign = "\xe2\x84\xa7" (* U+2127 INVERTED OHM SIGN, cl-25 or cl-27 *)
let celsius = "\xe2\x84\x83" (* U+2103 DEGREE CELSIUS, cl-13 *)
let kunoji_upper = "\xe3\x80\xb3" (* U+3033, cl-08 *)
let kunoji_lower = "\xe3\x80\xb5" (* U+3035, cl-08, the same mark's lower half *)
let em_dash = "\xe2\x80\x94" (* U+2014, cl-08 *)
let ellipsis = "\xe2\x80\xa6" (* U+2026, cl-08, a different kind *)

(** Where each line's clusters land, for a paragraph broken at one stated point.

    [break_after] is a cluster count: the request states a mandatory break there, so
    what precedes it is a line that is not the paragraph's last and is therefore the
    one §3.8.4 may open up. *)
let placed ?(paragraph_frame = Model.Full_em) ?(alignment = Paragraph.Justify)
    ?(style = Style.default ()) ~(extent : int) ~(break_after : int) (pieces : piece list) :
    int list list =
  let source = String.concat "" (List.map (fun piece -> piece.at) pieces) in
  let clusters, offsets, _ =
    List.fold_left
      (fun (clusters, offsets, start) piece ->
        let stop = start + String.length piece.at in
        ( cluster ?size:piece.own_size ?frame:piece.own_frame ?role:piece.own_role start stop
            piece.advance
          :: clusters,
          stop :: offsets,
          stop ))
      ([], [], 0) pieces
  in
  let breaks =
    match List.nth_opt (List.rev offsets) (break_after - 1) with
    | Some offset -> [ { Paragraph.offset; Paragraph.kind = Paragraph.Mandatory } ]
    | None -> []
  in
  let paragraph =
    Paragraph.build
      ~text:(text ~frame:paragraph_frame source (List.rev clusters))
      ~line_extent:extent ~breaks ~alignment ()
  in
  List.map
    (fun (line : Layout.line) ->
      List.map (fun (placement : Layout.placement) -> placement.Layout.inline)
        line.Layout.clusters)
    (Pipeline.compose paragraph style).Layout.lines

let first_line (lines : int list list) : string =
  match lines with
  | line :: _ -> String.concat " " (List.map string_of_int line)
  | [] -> "no line"

let ceiling (answer : string) : Style.t =
  Style.build [ ("adjustment.japanese_latin_expansion_ceiling", answer) ]

(* ----------------------------------------------------------------------------- *)
(* Vertical composition and tate-chu-yoko *)
(* ----------------------------------------------------------------------------- *)

let opening_bracket = "\xe3\x80\x88" (* U+3008, cl-01 *)
let closing_bracket = "\xe3\x80\x89" (* U+3009, cl-02 *)
let middle_dot = "\xe3\x83\xbb" (* U+30FB, cl-05 *)
let full_stop = "\xe3\x80\x82" (* U+3002, cl-06 *)
let ideographic_comma = "\xe3\x80\x81" (* U+3001, cl-07 *)
let hyphen = "\xe2\x80\x90" (* U+2010, cl-03 or cl-27 *)

(** The paragraph built from [pieces], with each named piece range covered by a
    tate-chu-yoko construct.

    [runs] and [breaks] are stated in {i piece} indices rather than byte offsets,
    because every check below is about which cluster went where and none of them is
    about UTF-8. *)
let compose_pieces ?(mode = Model.Vertical_rl) ?(style = Style.default ())
    ?(alignment = Paragraph.Start) ?(runs : (int * int) list = [])
    ?(breaks : (int * Paragraph.break_kind) list = []) ~(extent : int) (pieces : piece list) :
    Layout.t =
  let source = String.concat "" (List.map (fun piece -> piece.at) pieces) in
  let bounds = Array.make (List.length pieces + 1) 0 in
  let clusters, _ =
    List.fold_left
      (fun (clusters, start) piece ->
        let index = List.length clusters in
        let stop = start + String.length piece.at in
        bounds.(index) <- start;
        bounds.(index + 1) <- stop;
        ( cluster ?size:piece.own_size ?frame:piece.own_frame ?role:piece.own_role start stop
            piece.advance
          :: clusters,
          stop ))
      ([], 0) pieces
  in
  let constructs =
    List.map
      (fun (first, last) ->
        { Construct.range = (bounds.(first), bounds.(last)); Construct.kind = Construct.Tate_chu_yoko })
      runs
  in
  let breaks =
    List.map
      (fun (index, kind) -> { Paragraph.offset = bounds.(index); Paragraph.kind = kind })
      breaks
  in
  let paragraph =
    Paragraph.build ~text:(text source (List.rev clusters)) ~line_extent:extent ~breaks
      ~constructs ~alignment ~writing_mode:mode ()
  in
  Pipeline.compose paragraph style

(** One placement as [inline:block:advance:mode/transform] -- where it went, what it
    contributed, and the two fields §3.2's orientation decides. *)
let show_placement (placement : Layout.placement) : string =
  Printf.sprintf "%d:%d:%d:%s/%s" placement.Layout.inline placement.Layout.block
    placement.Layout.advance
    (Model.writing_mode_name placement.Layout.writing_mode)
    (Layout.transform_name placement.Layout.transform)

(** One line as [(inline extent/block extent) placement...]. *)
let show_line (line : Layout.line) : string =
  Printf.sprintf "(%d/%d) %s" line.Layout.inline_extent line.Layout.block_extent
    (String.concat " " (List.map show_placement line.Layout.clusters))

let lines_of (layout : Layout.t) : string =
  String.concat " | " (List.map show_line layout.Layout.lines)

let vertical ?mode ?style ?alignment ?runs ?breaks ~extent pieces : string =
  lines_of (compose_pieces ?mode ?style ?alignment ?runs ?breaks ~extent pieces)

let compose_json (envelope : string) : string =
  match Jlreq_proto.Protocol.request_of_line envelope with
  | Some request -> Jlreq_proto.Json.to_string (Jlreq_proto.Protocol.answer request)
  | None -> "the line was blank"

let envelope (id : string) (request : string) : string =
  Printf.sprintf "{\"protocol\":%S,\"spec\":%S,\"id\":%S,\"request\":%s}"
    Jlreq_proto.Protocol.protocol Jlreq_proto.Protocol.spec id request

let run () =
  (* The badness of one line, by leftover measure. *)
  let badness ?(is_last = false) ?(preference = "least-adjustment") delta =
    Pipeline.line_badness (Int64.of_int delta) is_last preference
  in
  Check.equal_int64 "a line that fits exactly costs nothing" ~expected:0L ~actual:(badness 0);
  Check.equal_int64 "a line short by one" ~expected:1L ~actual:(badness 1);
  Check.equal_int64 "a line short by a thousand" ~expected:1_000_000L ~actual:(badness 1000);
  Check.equal_int64 "even texture doubles it" ~expected:2_000_000L
    ~actual:(badness ~preference:"even-texture" 1000);
  Check.equal_int64 "the last line is charged a hundredth" ~expected:10_000L
    ~actual:(badness ~is_last:true 1000);
  Check.equal_int64 "and a last line short by nine is charged nothing" ~expected:0L
    ~actual:(badness ~is_last:true 9);
  Check.equal_int64 "an overrun of one" ~expected:10_001_000L ~actual:(badness (-1));
  Check.equal_int64 "an overrun of a thousand" ~expected:1_010_000_000L
    ~actual:(badness (-1000));
  Check.ok "an overrun costs a thousand times the shortfall it mirrors"
    (Int64.equal (badness (-1000))
       (Num.sadd (Num.smul (badness 1000) 1_000L) 10_000_000L));
  Check.ok "and the surcharge puts the smallest overrun above a wide shortfall"
    (Int64.compare (badness (-1)) (badness 3000) > 0);
  (* The magnitude is capped, so two impossibly wide lines still compare equal
     rather than overflowing into each other. *)
  Check.equal_int64 "the magnitude caps at a million"
    ~expected:(badness 1_000_000) ~actual:(badness 2_000_000);
  Check.equal_int64 "an overrun caps the same way"
    ~expected:(badness (-1_000_000)) ~actual:(badness (-9_000_000));
  Check.equal_int64 "the cap squared" ~expected:1_000_000_000_000L ~actual:(badness 1_000_000);
  (* The unreachable cost leaves room for several of itself to be added up. *)
  Check.ok "the infinite cost is a quarter of the range"
    (Int64.equal Num.infinite_cost (Int64.div Int64.max_int 4L));
  Check.ok "and four of them do not saturate"
    (Int64.compare (Num.sadd (Num.sadd Num.infinite_cost Num.infinite_cost)
                      (Num.sadd Num.infinite_cost Num.infinite_cost))
       Int64.max_int
    < 0);

  (* The validation boundary. *)
  let two = text "\xe6\x97\xa5\xe6\x9c\xac" [ cluster 0 3 1000; cluster 3 6 1000 ] in
  Check.returns "a well formed paragraph" (fun () ->
      Paragraph.build ~text:two ~line_extent:1000 ());
  Check.raises "clusters that leave a byte uncovered" (fun () ->
      Paragraph.build ~text:(text "\xe6\x97\xa5\xe6\x9c\xac" [ cluster 0 3 1000 ])
        ~line_extent:1000 ());
  Check.raises "a cluster boundary inside a scalar" (fun () ->
      Paragraph.build
        ~text:(text "\xe6\x97\xa5\xe6\x9c\xac" [ cluster 0 2 1000; cluster 2 6 1000 ])
        ~line_extent:1000 ());
  Check.raises "a negative advance" (fun () ->
      Paragraph.build ~text:(text "A" [ cluster 0 1 (-1) ]) ~line_extent:1000 ());
  Check.raises "a size of nothing" (fun () ->
      Paragraph.build ~text:(text ~size:{ Model.inline = 0; Model.block = 1000 } "A"
                               [ cluster 0 1 1000 ])
        ~line_extent:1000 ());
  Check.raises "a measure of nothing" (fun () -> Paragraph.build ~text:two ~line_extent:0 ());
  Check.raises "a break inside a scalar" (fun () ->
      Paragraph.build ~text:two ~line_extent:1000
        ~breaks:[ { Paragraph.offset = 1; Paragraph.kind = Paragraph.Allowed } ]
        ());
  Check.raises "two breaks at one offset" (fun () ->
      Paragraph.build ~text:two ~line_extent:1000
        ~breaks:
          [
            { Paragraph.offset = 3; Paragraph.kind = Paragraph.Allowed };
            { Paragraph.offset = 3; Paragraph.kind = Paragraph.Mandatory };
          ]
        ());
  Check.raises "a full-em cluster holding two ideographs" (fun () ->
      Paragraph.build ~text:(text "\xe6\x97\xa5\xe6\x9c\xac" [ cluster 0 6 1000 ])
        ~line_extent:1000 ());
  Check.returns "a proportional cluster holding a ligature" (fun () ->
      Paragraph.build
        ~text:(text ~frame:Model.Proportional "fi" [ cluster 0 2 800 ])
        ~line_extent:1000 ());

  (* The end of the source is a break whether or not the caller says so. *)
  let paragraph =
    Paragraph.build ~text:two ~line_extent:1000
      ~breaks:[ { Paragraph.offset = 3; Paragraph.kind = Paragraph.Allowed } ]
      ()
  in
  Check.equal_int "the terminal break is added" ~expected:2
    ~actual:(Array.length paragraph.Paragraph.breaks);
  Check.ok "and it is mandatory"
    (Paragraph.is_mandatory paragraph.Paragraph.breaks.(1));

  (* Composed: two full-em clusters, a one-em measure, one break between them. *)
  let layout = Pipeline.compose paragraph (Style.default ()) in
  Check.equal_int "two lines" ~expected:2 ~actual:(List.length layout.Layout.lines);
  Check.equal_int "no diagnostics" ~expected:0 ~actual:(List.length layout.Layout.diagnostics);
  (match layout.Layout.lines with
  | [ first; second ] ->
    Check.equal_int "the first line starts at the origin" ~expected:0
      ~actual:first.Layout.inline_origin;
    Check.equal_int "and is one em wide" ~expected:1000 ~actual:first.Layout.inline_extent;
    Check.equal_int "the second line is one em down" ~expected:1000
      ~actual:second.Layout.block_origin;
    Check.equal_int "and holds one placement" ~expected:1
      ~actual:(List.length second.Layout.clusters)
  | _ -> Check.ok "two lines" false);

  (* The same paragraph over the wire, byte for byte. *)
  Check.equal_string "quick-start/two-lines"
    ~expected:
      "{\"protocol\":\"jlreq.conformance/1\",\"spec\":\"jlreq-2020-08-11+unicode-17.0.0\",\"id\":\"quick-start/two-lines\",\"response\":{\"lines\":[{\"range\":[0,3],\"inline_origin\":0,\"block_origin\":0,\"inline_extent\":1000,\"block_extent\":1000,\"clusters\":[{\"origin\":{\"cluster\":0},\"range\":[0,3],\"inline\":0,\"block\":0,\"advance\":1000,\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"writing_mode\":\"horizontal-tb\",\"transform\":\"identity\"}],\"attachments\":[]},{\"range\":[3,6],\"inline_origin\":0,\"block_origin\":1000,\"inline_extent\":1000,\"block_extent\":1000,\"clusters\":[{\"origin\":{\"cluster\":1},\"range\":[3,6],\"inline\":0,\"block\":1000,\"advance\":1000,\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"writing_mode\":\"horizontal-tb\",\"transform\":\"identity\"}],\"attachments\":[]}],\"diagnostics\":[]}}"
    ~actual:
      (compose_json
         (envelope "quick-start/two-lines"
            "{\"source\":\"\xe6\x97\xa5\xe6\x9c\xac\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,3],\"advance\":1000},{\"range\":[3,6],\"advance\":1000}],\"line_extent\":1000,\"breaks\":[{\"offset\":3,\"kind\":\"allowed\"}],\"writing_mode\":\"horizontal-tb\",\"style\":\"jlreq-2020\"}"));

  (* End alignment moves the origin and leaves the extent alone (§3.5.3). *)
  Check.equal_string "3.5.3/end-alignment-shifts-origin-without-changing-extent"
    ~expected:
      "{\"protocol\":\"jlreq.conformance/1\",\"spec\":\"jlreq-2020-08-11+unicode-17.0.0\",\"id\":\"end\",\"response\":{\"lines\":[{\"range\":[0,6],\"inline_origin\":3000,\"block_origin\":0,\"inline_extent\":2000,\"block_extent\":1000,\"clusters\":[{\"origin\":{\"cluster\":0},\"range\":[0,3],\"inline\":3000,\"block\":0,\"advance\":1000,\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"writing_mode\":\"horizontal-tb\",\"transform\":\"identity\"},{\"origin\":{\"cluster\":1},\"range\":[3,6],\"inline\":4000,\"block\":0,\"advance\":1000,\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"writing_mode\":\"horizontal-tb\",\"transform\":\"identity\"}],\"attachments\":[]}],\"diagnostics\":[]}}"
    ~actual:
      (compose_json
         (envelope "end"
            "{\"source\":\"\xe6\x97\xa5\xe6\x9c\xac\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,3],\"advance\":1000},{\"range\":[3,6],\"advance\":1000}],\"line_extent\":5000,\"alignment\":\"end\",\"style\":\"jlreq-2020\"}"));

  (* --------------------------------------------------------------------------- *)
  (* §3.8.4: opening a justified line *)
  (* --------------------------------------------------------------------------- *)
  (* One cluster past the stated break, so that the line before it is not the
     paragraph's last and §3.8.4 applies to it. *)
  let tail = p "x" in
  let three_kanji = [ p kanji; p kanji; p kanji; tail ] in

  (* Table 6 states (cl-19, cl-19) as `0-1/4 stage 3`, so half a quarter em of room
     is taken at the third stage and stops there. *)
  Check.equal_string "two third-stage boundaries share what fits under their ceilings"
    ~expected:"0 1250 2500"
    ~actual:(first_line (placed ~extent:3500 ~break_after:3 three_kanji));
  Check.equal_string "and a wider line takes each of them exactly to its quarter em"
    ~expected:"0 1300 2600"
    ~actual:(first_line (placed ~extent:3600 ~break_after:3 three_kanji));
  (* §E.1: past those ceilings the fourth step opens the same boundaries further,
     rather than the line staying short. *)
  Check.equal_string "and past them the fourth step re-levels the same boundaries"
    ~expected:"0 1600 3200"
    ~actual:(first_line (placed ~extent:4200 ~break_after:3 three_kanji));
  (* A residual cell has no ceiling of its own and no stage before the fourth. *)
  Check.equal_string "two residual boundaries split the shortfall evenly"
    ~expected:"0 1250 2500"
    ~actual:
      (first_line
         (placed ~extent:3500 ~break_after:3
            [ p kanji; p ideographic_space; p kanji; tail ]));
  Check.equal_string "a third-stage boundary is filled before a residual one"
    ~expected:"0 1375 2500"
    ~actual:
      (first_line
         (placed ~extent:3500 ~break_after:3
            [ p kanji; p kanji; p ideographic_space; tail ]));

  (* The rounding remainder goes where `adjustment.remainder` says (§3.8.3). *)
  Check.equal_string "an odd shortfall leaves its last unit at the leading boundary"
    ~expected:"0 1251 2501"
    ~actual:(first_line (placed ~extent:3501 ~break_after:3 three_kanji));
  Check.equal_string "or at the trailing one" ~expected:"0 1250 2501"
    ~actual:
      (first_line
         (placed ~extent:3501 ~break_after:3
            ~style:(Style.build [ ("adjustment.remainder", "trailing") ])
            three_kanji));

  (* §3.8.4 step (b): the quarter em between cl-19 and cl-27 opens to a half em, to
     a third of an em, or -- read as a fixed space -- no further than it already is,
     which leaves the third stage's own boundary to take the whole shortfall. *)
  let kanji_western =
    [ p kanji; p ~frame:Model.Proportional "A"; p kanji; p kanji; tail ]
  in
  Check.equal_string "a half em is the default ceiling" ~expected:"0 1450 2900 3900"
    ~actual:(first_line (placed ~extent:4900 ~break_after:4 kanji_western));
  Check.equal_string "a third of an em is the alternative" ~expected:"0 1334 2668 3900"
    ~actual:
      (first_line
         (placed ~extent:4900 ~break_after:4 ~style:(ceiling "third-em") kanji_western));
  Check.equal_string "and a fixed quarter em opens nothing there at all"
    ~expected:"0 1250 2500 3900"
    ~actual:(first_line (placed ~extent:4900 ~break_after:4 ~style:(ceiling "rigid") kanji_western));
  (* The question is asked at cl-19 against cl-27 and nowhere else: the same
     coordinate with katakana on the Japanese side keeps the table's own half em. *)
  Check.equal_string "the ceiling question governs cl-19 against cl-27"
    ~expected:"0 1250 2500 3750"
    ~actual:
      (first_line
         (placed ~extent:4750 ~break_after:4 ~style:(ceiling "rigid")
            [ p kanji; p unit_sign; p kanji; p kanji; tail ]));
  Check.equal_string "and not cl-16 against cl-27" ~expected:"0 1375 2750 3750"
    ~actual:
      (first_line
         (placed ~extent:4750 ~break_after:4 ~style:(ceiling "rigid")
            [ p katakana; p unit_sign; p katakana; p kanji; tail ]));

  (* §E.2 note 4: two inseparable characters open only when they are of different
     kinds, and the vertical kana repeat mark's two halves are one kind. *)
  let inseparable left right =
    first_line
      (placed ~extent:2250 ~break_after:2 [ p left; p right; tail ])
  in
  Check.equal_string "two occurrences of one inseparable character stay solid" ~expected:"0 1000"
    ~actual:(inseparable kunoji_upper kunoji_upper);
  Check.equal_string "and so do the two halves of one repeat mark" ~expected:"0 1000"
    ~actual:(inseparable kunoji_upper kunoji_lower);
  Check.equal_string "two different kinds open a quarter em" ~expected:"0 1250"
    ~actual:(inseparable em_dash ellipsis);

  (* §E.2 note 10: a Western character keeps its postfixed abbreviation when it is a
     quantity symbol or a European numeral. *)
  let postfixed before =
    first_line (placed ~extent:2250 ~break_after:2 [ before; p celsius; tail ])
  in
  Check.equal_string "an ordinary Western character opens a quarter em" ~expected:"0 1250"
    ~actual:(postfixed (p ~frame:Model.Proportional "Z"));
  Check.equal_string "a European numeral does not" ~expected:"0 1000"
    ~actual:(postfixed (p ~frame:Model.Proportional "1"));
  Check.equal_string "and neither does a declared quantity symbol" ~expected:"0 1000"
    ~actual:(postfixed (p ~frame:Model.Proportional ~role:Model.Quantity_symbol "Z"));

  (* §3.8.4 step (a): the Western word space opens to a half em before anything
     else, and §3.2.2's collapsed space at a line edge is not there to open. *)
  let words =
    [
      p ~advance:500 "A"; p ~advance:333 " "; p ~advance:500 "B"; p ~advance:500 "C";
    ]
  in
  Check.equal_string "a small shortfall is taken by the word space alone" ~expected:"0 500 933"
    ~actual:
      (first_line (placed ~paragraph_frame:Model.Proportional ~extent:1433 ~break_after:3 words));
  Check.equal_string "a larger one carries on into the fourth step" ~expected:"0 750 1500"
    ~actual:
      (first_line (placed ~paragraph_frame:Model.Proportional ~extent:2000 ~break_after:3 words));
  Check.equal_string "a word space at either line edge opens nothing" ~expected:"0 0 500"
    ~actual:
      (first_line
         (placed ~paragraph_frame:Model.Proportional ~extent:2000 ~break_after:3
            [
              p ~advance:333 " "; p ~advance:500 "A"; p ~advance:333 " "; p ~advance:500 "B";
            ]));

  (* Only a justified line takes up the measure, and never the last one. *)
  Check.equal_string "a line set flush at the start is left short" ~expected:"0 1000 2000"
    ~actual:
      (first_line (placed ~alignment:Paragraph.Start ~extent:4200 ~break_after:3 three_kanji));
  Check.equal_string "and the last line of a paragraph is left short too"
    ~expected:"0 1000 2000 3000"
    ~actual:(first_line (placed ~extent:5200 ~break_after:4 three_kanji));

  (* Vertical composition: §3.2's three orientations. A tate-chu-yoko member is set
     horizontally inside the vertical line, a proportional cluster is rotated a
     quarter turn (§3.2.6), and a Western character in a fixed frame is
     quasi-Japanese and stands up (§3.2.4). *)
  let digit ?(advance = 500) ?size at = p ~frame:Model.Proportional ~advance ?size at in
  Check.equal_string "vertical composition orients each cluster for itself"
    ~expected:"(2250/1000) 0:0:750:vertical-rl/rotate-clockwise 750:0:500:vertical-rl/identity 1250:0:1000:vertical-rl/identity"
    ~actual:
      (vertical ~extent:4000
         [ digit "1"; p ~frame:Model.Half_em ~advance:500 "A"; p kanji ]);
  Check.equal_string "and horizontal composition sets everything upright"
    ~expected:"(2250/1000) 0:0:750:horizontal-tb/identity 750:0:500:horizontal-tb/identity 1250:0:1000:horizontal-tb/identity"
    ~actual:
      (vertical ~mode:Model.Horizontal_tb ~extent:4000
         [ digit "1"; p ~frame:Model.Half_em ~advance:500 "A"; p kanji ]);

  (* §3.2.5: the string is set solid from left to right and the whole of it is
     centered on the line, so a member sits half the run's width back from where an
     ordinary cluster would and every member after it follows by its own advance. *)
  Check.equal_string "a tate-chu-yoko run is centered across the line" ~expected:"(1000/1000) 0:-500:500:horizontal-tb/tate-chu-yoko 0:0:500:horizontal-tb/tate-chu-yoko"
    ~actual:(vertical ~extent:4000 ~runs:[ (0, 2) ] [ digit "1"; digit "2" ]);
  Check.equal_string "and half of an odd width is taken toward the line's own origin"
    ~expected:"(1000/1233) 0:-616:300:horizontal-tb/tate-chu-yoko 0:-316:433:horizontal-tb/tate-chu-yoko 0:117:500:horizontal-tb/tate-chu-yoko"
    ~actual:
      (vertical ~extent:4000 ~runs:[ (0, 3) ]
         [ digit ~advance:300 "1"; digit ~advance:433 "2"; digit ~advance:500 "3" ]);
  (* The run is as wide across the line as its members are long and as long as its
     tallest member is wide: the two ems change places with the member. *)
  Check.equal_string "a member's block em is what the run takes up along the line"
    ~expected:"(1400/1000) 0:-500:500:horizontal-tb/tate-chu-yoko 0:0:500:horizontal-tb/tate-chu-yoko"
    ~actual:
      (vertical ~extent:4000 ~runs:[ (0, 2) ]
         [
           digit "1";
           digit ~size:{ Model.inline = 1000; Model.block = 1400 } "2";
         ]);

  (* §3.2.5's four amounts: a half em after a comma, a closing bracket or a full
     stop, a half em before an opening bracket, and solid against everything else --
     including the six cl-30 coordinates Table 1 gives a quarter em to. *)
  Check.equal_string "a run takes a half em after a comma and before an opening bracket"
    ~expected:"(4000/1000) 0:0:1500:vertical-rl/identity 1500:-500:500:horizontal-tb/tate-chu-yoko 1500:0:500:horizontal-tb/tate-chu-yoko 3000:0:1000:vertical-rl/identity"
    ~actual:
      (vertical ~extent:6000 ~runs:[ (1, 3) ]
         [ p ideographic_comma; digit "1"; digit "2"; p opening_bracket ]);
  Check.equal_string "and is solid against an ideograph and against a middle dot"
    ~expected:"(3250/1000) 0:0:1000:vertical-rl/identity 1000:-500:500:horizontal-tb/tate-chu-yoko 1000:0:500:horizontal-tb/tate-chu-yoko 2000:0:1250:vertical-rl/identity"
    ~actual:
      (vertical ~extent:6000 ~runs:[ (1, 3) ]
         [ p kanji; digit "1"; digit "2"; p middle_dot ]);
  Check.equal_string "a half em after a closing bracket, and after a full stop"
    ~expected:"(5000/1000) 0:0:1500:vertical-rl/identity 1500:-250:500:horizontal-tb/tate-chu-yoko 2500:0:1500:vertical-rl/identity 4000:-250:500:horizontal-tb/tate-chu-yoko"
    ~actual:
      (vertical ~extent:8000 ~runs:[ (1, 2); (3, 4) ]
         [ p closing_bracket; digit "1"; p full_stop; digit "2" ]);
  (* The run is one thing on the line, so the space §3.2.5 puts after it is stated
     against the next character of the paragraph and survives the line ending. *)
  Check.equal_string "the half em before an opening bracket survives a line end"
    ~expected:"(1500/1000) 0:-500:500:horizontal-tb/tate-chu-yoko 0:0:500:horizontal-tb/tate-chu-yoko | (1000/1000) 0:-1000:1000:vertical-rl/identity"
    ~actual:
      (vertical ~extent:2000 ~runs:[ (0, 2) ]
         ~breaks:[ (2, Paragraph.Mandatory) ]
         [ digit "1"; digit "2"; p opening_bracket ]);

  (* §E.2 note 12: the (cl-30, cl-30) cell belongs to two characters of different
     runs. Three runs give two such boundaries and three internal ones, and only the
     two open. *)
  Check.equal_string "expansion opens between two runs and never inside one"
    ~expected:"(3500/1000) 0:-500:500:horizontal-tb/tate-chu-yoko 0:0:500:horizontal-tb/tate-chu-yoko 1250:-500:500:horizontal-tb/tate-chu-yoko 1250:0:500:horizontal-tb/tate-chu-yoko 2500:-500:500:horizontal-tb/tate-chu-yoko 2500:0:500:horizontal-tb/tate-chu-yoko | (1000/1000) 0:-1000:1000:vertical-rl/identity"
    ~actual:
      (vertical ~extent:3500 ~alignment:Paragraph.Justify
         ~runs:[ (0, 2); (2, 4); (4, 6) ] ~breaks:[ (6, Paragraph.Mandatory) ]
         [ digit "1"; digit "2"; digit "3"; digit "4"; digit "5"; digit "6"; p kanji ]);

  (* §C.2 note 13, which this engine answers by refusing the request rather than by
     declining the opportunity. *)
  Check.raises "a break inside a tate-chu-yoko run" (fun () ->
      compose_pieces ~extent:4000 ~runs:[ (0, 2) ] ~breaks:[ (1, Paragraph.Allowed) ]
        [ digit "1"; digit "2" ]);
  Check.raises "and a mandatory one, which is no more divisible" (fun () ->
      compose_pieces ~extent:4000 ~runs:[ (0, 2) ] ~breaks:[ (1, Paragraph.Mandatory) ]
        [ digit "1"; digit "2" ]);
  Check.raises "and one in horizontal composition, where the run sets nothing"
    (fun () ->
      compose_pieces ~mode:Model.Horizontal_tb ~extent:4000 ~runs:[ (0, 2) ]
        ~breaks:[ (1, Paragraph.Allowed) ] [ digit "1"; digit "2" ]);
  Check.returns "a break at the run's own edge" (fun () ->
      compose_pieces ~extent:4000 ~runs:[ (0, 2) ] ~breaks:[ (2, Paragraph.Allowed) ]
        [ digit "1"; digit "2"; p kanji ]);
  Check.returns "and one between two runs set side by side" (fun () ->
      compose_pieces ~extent:4000 ~runs:[ (0, 2); (2, 4) ] ~breaks:[ (2, Paragraph.Allowed) ]
        [ digit "1"; digit "2"; digit "3"; digit "4" ]);

  (* §A.24 and §A.25 list U+0020 at a quarter em, which is a width the protocol has
     no frame for, so neither listing is reachable and the space stays the Western
     word space however the caller labels it. §A.03 lists U+2010 the same way, so a
     proportional hyphen is the Western character §A.27 lists. *)
  Check.equal_string "a space the caller calls a grouped numeral is still a word space"
    ~expected:"(3000/1000) 0:0:1000:vertical-rl/identity 1000:0:1000:vertical-rl/identity 2000:0:1000:vertical-rl/identity"
    ~actual:
      (vertical ~extent:6000
         [ p kanji; p ~role:Model.Grouped_numeral " "; p kanji ]);
  Check.equal_string "and a proportional hyphen is a Western character" ~expected:"(3000/1000) 0:0:1250:vertical-rl/identity 1250:0:750:vertical-rl/rotate-clockwise 2000:0:1000:vertical-rl/identity"
    ~actual:(vertical ~extent:6000 [ p kanji; digit hyphen; p kanji ]);

  (* A request the protocol does not carry is an error, not a default. *)
  Check.raises "an unknown request field" (fun () ->
      compose_json
        (envelope "x"
           "{\"source\":\"A\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,1],\"advance\":1000}],\"line_extent\":1000,\"leading\":1000}"));
  Check.raises "an unknown cluster role" (fun () ->
      compose_json
        (envelope "x"
           "{\"source\":\"A\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,1],\"advance\":1000,\"role\":\"headword\"}],\"line_extent\":1000}"));
  Check.raises "an unknown style setting" (fun () ->
      compose_json
        (envelope "x"
           "{\"source\":\"A\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,1],\"advance\":1000}],\"line_extent\":1000,\"style\":{\"profile\":\"jlreq-2020\",\"spacing.leading\":\"loose\"}}"));
  Check.returns "and a request with every optional field left out" (fun () ->
      compose_json
        (envelope "x"
           "{\"source\":\"A\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,1],\"advance\":1000}],\"line_extent\":1000}"))
