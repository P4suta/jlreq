(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** {!Jlreq.Tables}: the census, the token grammar, and the third transcription.

    The last section is the one that earns a second engine. Appendices B through E
    exist only as PDF, so every cell of the six matrices was typed in twice, once
    from the English rendering and once from the Japanese. The Rust engine reads
    the English files. This engine reads the Japanese ones. This test builds the
    English ones too and compares them cell for cell, which makes the agreement of
    the two transcriptions a checked fact rather than an assumption -- and turns
    any disagreement into a named coordinate here instead of an unexplained DIFF
    somewhere in the suite. *)

open Jlreq.Tables

let quarter_em = 180
let half_em = 360
let three_quarter_em = 540
let eighth_em = 90

let census () =
  Check.returns "the startup self-check passes" self_check;

  (* Appendix A. *)
  Check.equal_int "Appendix A rows" ~expected:1687 ~actual:(List.length appendix_a);
  Check.equal_int "Appendix A listings" ~expected:1686
    ~actual:(Hashtbl.length appendix_a_listing);
  Check.equal_int "Appendix A distinct keys" ~expected:1133
    ~actual:(Hashtbl.length appendix_a_keys);
  Check.equal_int "Appendix A distinct Remarks pairs" ~expected:14
    ~actual:(List.length appendix_a_remarks);
  (* The one recorded duplicate: §A.19 lists U+216B twice. *)
  let duplicates =
    let seen = Hashtbl.create 2048 and out = ref [] in
    List.iter
      (fun row ->
        let key = Printf.sprintf "%s %s" (row_label row.listing_class) row.listing_key_text in
        if Hashtbl.mem seen key then out := key :: !out else Hashtbl.add seen key ())
      appendix_a;
    !out
  in
  Check.ok "the only duplicated listing is cl-19 216B" (duplicates = [ "cl-19 216B" ]);
  Check.ok "the empty Remarks pair is one of the fourteen"
    (List.mem ("", "") appendix_a_remarks);
  (* A multi-scalar key, which is why keys are sequences and not scalars. *)
  Check.ok "a combining-mark key is two scalars"
    (match Hashtbl.find_opt appendix_a_listing "cl-27\t0254 0300" with
    | Some row -> Array.length row.listing_key = 2 && row.listing_key.(1) = 0x0300
    | None -> false);

  (* The derived Unicode tables. *)
  Check.equal_int "folding entries" ~expected:226 ~actual:(List.length folding);
  Check.equal_int "Unified_Ideograph ranges" ~expected:16 ~actual:(List.length ideographs);
  Check.equal_int "Hiragana and Katakana ranges" ~expected:22 ~actual:(List.length scripts);
  Check.ok "the ideographic space folds onto U+0020"
    (match Hashtbl.find_opt folding_map 0x3000 with
    | Some entry -> entry.fold_target = 0x0020 && entry.fold_frame = "full-em"
    | None -> false);
  Check.ok "U+4E00..U+9FFF is a Unified_Ideograph range"
    (List.exists (fun range -> range.first = 0x4E00 && range.last = 0x9FFF) ideographs);
  Check.ok "both scripts are present"
    (List.exists (fun range -> range.script = "Hiragana") scripts
    && List.exists (fun range -> range.script = "Katakana") scripts);

  (* The class roster and the Style questions. *)
  Check.equal_int "character classes" ~expected:30 ~actual:(List.length classes);
  Check.equal_int "Style questions" ~expected:22 ~actual:(Jlreq.Tsv.row_count questions);
  Check.ok "cl-17 and cl-18 are listed classes"
    (List.exists (fun entry -> entry.entry_class = 17) classes
    && List.exists (fun entry -> entry.entry_class = 18) classes);
  Check.equal_bool "cl-17 has no adjacency" ~expected:false ~actual:(has_adjacency 17);
  Check.equal_bool "cl-18 has no adjacency" ~expected:false ~actual:(has_adjacency 18);
  Check.equal_bool "cl-19 has adjacency" ~expected:true ~actual:(has_adjacency 19);
  Check.equal_bool "the line edge is an axis but not a class" ~expected:true
    ~actual:(is_axis_class 0 && not (is_class 0));

  (* The six matrices. *)
  List.iter
    (fun (table, cells, axis) ->
      Check.equal_int
        (Printf.sprintf "Table %d cells" table.number)
        ~expected:cells ~actual:(Hashtbl.length table.cells);
      Check.equal_int
        (Printf.sprintf "Table %d axis" table.number)
        ~expected:axis ~actual:(Array.length table.row_axis))
    [
      (table1, 841, 29); (table2, 784, 28); (table3, 841, 29);
      (table4, 841, 29); (table5, 841, 29); (table6, 784, 28);
    ]

let labels_and_amounts () =
  Check.equal_int "line-head is the line edge" ~expected:0 ~actual:(klass_of_label "line-head");
  Check.equal_int "line-end is the line edge" ~expected:0 ~actual:(klass_of_label "line-end");
  Check.equal_int "cl-01" ~expected:1 ~actual:(klass_of_label "cl-01");
  Check.equal_int "cl-30" ~expected:30 ~actual:(klass_of_label "cl-30");
  Check.equal_string "the row axis spells the edge line-head" ~expected:"line-head"
    ~actual:(row_label 0);
  Check.equal_string "the column axis spells it line-end" ~expected:"line-end"
    ~actual:(column_label 0);
  Check.equal_string "a class label is zero-padded" ~expected:"cl-07" ~actual:(row_label 7);
  Check.raises "cl-31" (fun () -> klass_of_label "cl-31");
  Check.raises "cl-00" (fun () -> klass_of_label "cl-00");
  Check.raises "an unpadded label" (fun () -> klass_of_label "cl-1");
  Check.raises "prose where a label belongs" (fun () -> klass_of_label "line head");

  Check.equal_int "a full em" ~expected:720 ~actual:(amount_of_token "1");
  Check.equal_int "three quarters" ~expected:three_quarter_em ~actual:(amount_of_token "3/4");
  Check.equal_int "a half" ~expected:half_em ~actual:(amount_of_token "1/2");
  Check.equal_int "a quarter" ~expected:quarter_em ~actual:(amount_of_token "1/4");
  Check.equal_int "an eighth" ~expected:eighth_em ~actual:(amount_of_token "1/8");
  Check.equal_int "a third is exact in 1/720 em" ~expected:240 ~actual:(amount_of_token "1/3");
  Check.equal_int "solid" ~expected:0 ~actual:(amount_of_token "0");
  Check.raises "an amount above one em" (fun () -> amount_of_token "2");
  Check.raises "an amount not exact in 1/720 em" (fun () -> amount_of_token "1/7");
  Check.raises "a division by zero" (fun () -> amount_of_token "1/0");
  Check.raises "prose where an amount belongs" (fun () -> amount_of_token "half")

let token_grammar () =
  let token text = cell_of_token text in
  Check.ok "blank" (token "blank" = Blank);
  Check.ok "the multiplication sign is prohibition" (token "\xc3\x97" = Prohibited);
  Check.ok "ruby hang" (token "ruby hang" = Ruby_hang);
  Check.ok "residual" (token "residual" = Residual);
  Check.ok "not" (token "not" = No_break []);
  Check.ok "not 3,4" (token "not 3,4" = No_break [ 3; 4 ]);
  Check.ok "not 1" (token "not 1" = No_break [ 1 ]);
  Check.raises "a level outside the four" (fun () -> token "not 5");

  Check.ok "one Table 1 term"
    (token "1/4 af"
    = Spacing [ { term_amount = quarter_em; term_side = After; term_hang = false } ]);
  Check.ok "a term the ruby may hang over"
    (token "1/2 be hang"
    = Spacing [ { term_amount = half_em; term_side = Before; term_hang = true } ]);
  Check.ok "two terms"
    (token "1/2 be + 1/4 af"
    = Spacing
        [
          { term_amount = half_em; term_side = Before; term_hang = false };
          { term_amount = quarter_em; term_side = After; term_hang = false };
        ]);
  Check.raises "a term with no side" (fun () -> token "1/4 xx");

  Check.ok "a rigid amount" (token "1/2" = Rigid { amount = half_em; stage = None });
  Check.ok "a rigid amount at a stage"
    (token "1/4 stage 3" = Rigid { amount = quarter_em; stage = Some 3 });
  Check.ok "a reduction to a floor"
    (token "1/2-0 stage 5"
    = Movable { amount = half_em; limit = 0; two_valued = false; stage = 5 });
  Check.ok "a reduction to a non-zero floor"
    (token "1/4-1/8 stage 6"
    = Movable { amount = quarter_em; limit = eighth_em; two_valued = false; stage = 6 });
  Check.ok "a two-valued cell"
    (token "1/2=0 stage 2"
    = Movable { amount = half_em; limit = 0; two_valued = true; stage = 2 });
  Check.ok "an expansion to a ceiling"
    (token "0-1/4 stage 3"
    = Movable { amount = 0; limit = quarter_em; two_valued = false; stage = 3 });
  Check.ok "an en dash reads as a hyphen"
    (token "1/4\xe2\x80\x931/8 stage 6"
    = Movable { amount = quarter_em; limit = eighth_em; two_valued = false; stage = 6 });
  Check.raises "a limit with no stage" (fun () -> token "1/2-0");
  Check.raises "a stage with no ordinal" (fun () -> token "1/2-0 stage");
  Check.raises "a stage outside the ladders" (fun () -> token "1/2-0 stage 12");
  Check.raises "a token outside the vocabulary" (fun () -> token "sometimes");

  (* Cells the transcription actually holds, at coordinates the capture preambles
     and the appendix notes name. *)
  Check.ok "Table 1 (cl-01, cl-29) is prohibited" (cell table1 1 29 = Prohibited);
  Check.ok "Table 1 (cl-02, line-end) is a half em before"
    (cell table1 2 0 = Spacing [ { term_amount = half_em; term_side = Before; term_hang = false } ]);
  Check.ok "Table 1 (cl-05, cl-05) is a quarter each side"
    (cell table1 5 5
    = Spacing
        [
          { term_amount = quarter_em; term_side = Before; term_hang = false };
          { term_amount = quarter_em; term_side = After; term_hang = false };
        ]);
  Check.ok "Table 3 (cl-02, line-end) is two-valued at stage 2"
    (cell table3 2 0 = Movable { amount = half_em; limit = 0; two_valued = true; stage = 2 });
  Check.ok "Table 6 (cl-02, cl-01) is residual" (cell table6 2 1 = Residual);
  Check.ok "Table 2 (cl-02, cl-01) forbids no break" (cell table2 2 1 = Blank);
  Check.ok "Tables 2 and 6 have no line edge"
    (not (states table2 0 1) && not (states table6 0 1));
  Check.ok "a note is carried through" (note table1 1 22 = Some "B.2#1");
  Check.ok "an unqualified cell cites no note" (note table1 1 29 = None)

(** Build the English transcriptions and compare them, cell for cell, with the
    Japanese ones the engine runs on. *)
let transcriptions_agree () =
  let pairs =
    [
      (table1, En_data.table1); (table2, En_data.table2); (table3, En_data.table3);
      (table4, En_data.table4); (table5, En_data.table5); (table6, En_data.table6);
    ]
  in
  List.iter
    (fun (japanese, english_text) ->
      let english = matrix_of_tsv japanese.number english_text in
      Check.equal_int
        (Printf.sprintf "Table %d has the same cell count in both locales" japanese.number)
        ~expected:(Hashtbl.length japanese.cells) ~actual:(Hashtbl.length english.cells);
      Check.ok
        (Printf.sprintf "Table %d has the same row axis in both locales" japanese.number)
        (japanese.row_axis = english.row_axis);
      Check.ok
        (Printf.sprintf "Table %d has the same column axis in both locales" japanese.number)
        (japanese.column_axis = english.column_axis);
      let disagreements = ref [] in
      Array.iter
        (fun before ->
          Array.iter
            (fun after ->
              if cell japanese before after <> cell english before after then
                disagreements :=
                  Printf.sprintf "(%s, %s) token" (row_label before) (column_label after)
                  :: !disagreements
              else if note japanese before after <> note english before after then
                disagreements :=
                  Printf.sprintf "(%s, %s) note" (row_label before) (column_label after)
                  :: !disagreements)
            japanese.column_axis)
        japanese.row_axis;
      Check.equal_int
        (Printf.sprintf "Table %d: transcriptions disagree at no coordinate" japanese.number)
        ~expected:0
        ~actual:(List.length !disagreements);
      List.iter
        (fun where ->
          Printf.printf "     table %d disagrees at %s\n" japanese.number where)
        (List.rev !disagreements))
    pairs

let run () =
  census ();
  labels_and_amounts ();
  token_grammar ();
  transcriptions_agree ()
