(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** {!Jlreq.Spec}: classification, the Remarks vocabulary, and the matrix lookups.

    The classification checks use the code points the development census picks as
    each class's representative -- the least ambiguous key Appendix A lists for it
    -- so a change that moves one of them shows up here as a named class rather
    than as two thousand differing census answers. *)

open Jlreq

let classify ?(frame = Model.Full_em) ?role ?(writing_mode = Model.Horizontal_tb)
    ?(unlisted_is_ideographic = false) ?(highest_ambiguous_class = false)
    ?(grouped_numeral_requires_role = false) (piece : string) : int =
  Spec.class_of ~piece ~frame ~role ~writing_mode ~unlisted_is_ideographic
    ~highest_ambiguous_class ~grouped_numeral_requires_role

let scalar code = Utf8.of_scalar code

let run () =
  (* An amount of an em, rounded away from zero. *)
  Check.equal_int "a quarter em of 1000" ~expected:250 ~actual:(Spec.scale_spec_units 1000 180);
  Check.equal_int "a quarter em of 500" ~expected:125 ~actual:(Spec.scale_spec_units 500 180);
  Check.equal_int "an eighth em of 1000" ~expected:125 ~actual:(Spec.scale_spec_units 1000 90);
  Check.equal_int "a half em of 1000" ~expected:500 ~actual:(Spec.scale_spec_units 1000 360);
  Check.equal_int "a whole em of 1000" ~expected:1000 ~actual:(Spec.scale_spec_units 1000 720);
  Check.equal_int "a quarter em of 1001 rounds up" ~expected:251
    ~actual:(Spec.scale_spec_units 1001 180);
  Check.equal_int "no amount is no space" ~expected:0 ~actual:(Spec.scale_spec_units 1000 0);

  (* The representative of each class the census can address classifies to it.
     cl-24 and cl-26 are not here: Appendix A lists the same key (U+0020) for both
     and the ordinary class wins, which is the census's own finding. *)
  let representative label code expected =
    Check.equal_int (Printf.sprintf "%s classifies U+%04X" label code) ~expected
      ~actual:(classify (scalar code))
  in
  representative "cl-01" 0x3008 1;
  representative "cl-02" 0x3009 2;
  representative "cl-03" 0x301C 3;
  representative "cl-04" 0x203C 4;
  representative "cl-05" 0x30FB 5;
  representative "cl-06" 0x3002 6;
  representative "cl-07" 0x3001 7;
  representative "cl-08" 0x3033 8;
  representative "cl-09" 0x30FD 9;
  representative "cl-10" 0x30FC 10;
  representative "cl-11" 0x3041 11;
  representative "cl-12" 0x2116 12;
  representative "cl-13" 0x2103 13;
  representative "cl-14" 0x3000 14;
  representative "cl-15" 0x3042 15;
  representative "cl-16" 0x30A2 16;
  representative "cl-19" 0x203B 19;
  representative "cl-27" 0x0022 27;

  (* The three §3.9.2 questions, each with both answers. *)
  Check.equal_int "an unlisted code point is Western when it is proportional" ~expected:27
    ~actual:(classify ~frame:Model.Proportional "\xf0\x9f\xa6\x80");
  Check.equal_int "an unlisted code point is ideographic when it is not" ~expected:19
    ~actual:(classify ~frame:Model.Full_em "\xf0\x9f\xa6\x80");
  Check.equal_int "an unlisted code point can be ideographic anyway" ~expected:19
    ~actual:(classify ~frame:Model.Proportional ~unlisted_is_ideographic:true "\xf0\x9f\xa6\x80");
  Check.equal_int "an ambiguous key takes the lowest class by default" ~expected:17
    ~actual:(classify (scalar 0x2194));
  Check.equal_int "and the highest when the style says so" ~expected:19
    ~actual:(classify ~highest_ambiguous_class:true (scalar 0x2194));
  Check.equal_int "a half-width digit is a grouped numeral's by width" ~expected:19
    ~actual:(classify ~frame:Model.Half_em "1");
  Check.equal_int "and Western when a grouped numeral needs the role" ~expected:27
    ~actual:(classify ~frame:Model.Half_em ~grouped_numeral_requires_role:true "1");
  Check.equal_int "the role names the class the caller means" ~expected:24
    ~actual:(classify ~frame:Model.Half_em ~role:Model.Grouped_numeral "1");

  (* The frame decides between the full-width form and the Western one. *)
  Check.equal_int "a proportional letter is Western" ~expected:27
    ~actual:(classify ~frame:Model.Proportional "A");
  Check.equal_int "a full-em letter is the full-width form" ~expected:19
    ~actual:(classify ~frame:Model.Full_em "A");

  (* A role the key is not listed under does not invent a listing. *)
  Check.equal_int "a middle dot called a decimal point is still a middle dot" ~expected:5
    ~actual:(classify ~role:Model.Decimal_point (scalar 0x30FB));
  Check.equal_int "a proportional letter called a unit symbol is one" ~expected:25
    ~actual:(classify ~frame:Model.Proportional ~role:Model.Unit_symbol "m");

  (* Usage: §A.6 lists U+002E for horizontal composition only. *)
  Check.equal_int "a full stop in horizontal composition" ~expected:6
    ~actual:(classify ~frame:Model.Half_em ".");
  Check.ok "and not in vertical composition"
    (classify ~frame:Model.Half_em ~writing_mode:Model.Vertical_rl "." <> 6);

  (* Membership follows the Wide and Narrow folds, but never before a literal
     listing: U+3000 is the ideographic space and not a Western word space. *)
  Check.ok "the ideographic space is cl-14" (Spec.single_has_class 0x3000 14);
  Check.ok "and is not cl-26" (not (Spec.single_has_class 0x3000 26));
  Check.ok "a full-width parenthesis is an opening bracket"
    (Spec.single_has_class 0xFF08 Spec.opening_bracket);
  Check.ok "an ideograph outside Appendix A is still cl-19"
    (Spec.single_has_class 0x20000 Spec.ideograph);
  Check.ok "hiragana is hiragana" (Spec.is_hiragana 0x3042);
  Check.ok "katakana is katakana" (Spec.is_katakana 0x30A2);
  Check.ok "an Appendix A pair" (Spec.is_pair 0x02E5 0x02E9);
  Check.ok "and a sequence that is not one" (not (Spec.is_pair 0x0041 0x0042));

  (* Table 1, in the units the caller's em is stated in. *)
  let em = { Model.inline = 1000; Model.block = 1000 } in
  let space ?(before_solid = false) ?(after_solid = false) ~before ~after () =
    Spec.table_one_space ~before ~after ~before_size:em ~after_size:em ~before_solid ~after_solid
  in
  Check.equal_int "an ideograph before a Western character" ~expected:250
    ~actual:(space ~before:19 ~after:27 ());
  Check.equal_int "a Western character before an ideograph" ~expected:250
    ~actual:(space ~before:27 ~after:19 ());
  Check.equal_int "two ideographs are solid" ~expected:0 ~actual:(space ~before:19 ~after:19 ());
  Check.equal_int "a closing bracket at the line end" ~expected:500
    ~actual:(space ~before:2 ~after:0 ());
  Check.equal_int "a middle dot at the line end" ~expected:250
    ~actual:(space ~before:5 ~after:0 ());
  Check.equal_int "an ideograph at the line end" ~expected:0
    ~actual:(space ~before:19 ~after:0 ());
  Check.equal_int "a solid neighbor drops the term it is measured from" ~expected:0
    ~actual:(space ~before:19 ~after:5 ~after_solid:true ());
  Check.equal_int "and leaves the other one alone" ~expected:250
    ~actual:(space ~before:19 ~after:5 ~before_solid:true ());
  Check.ok "two ideographs state no space" (Spec.table_one_is_blank ~before:19 ~after:19);
  Check.ok "an ideograph before a Western character does"
    (not (Spec.table_one_is_blank ~before:19 ~after:27));

  (* The term is measured from one neighbor's em, not from the paragraph's. *)
  let half = { Model.inline = 500; Model.block = 500 } in
  Check.equal_int "a trailing term takes the following em" ~expected:125
    ~actual:
      (Spec.table_one_space ~before:27 ~after:19 ~before_size:em ~after_size:half
         ~before_solid:false ~after_solid:false);
  Check.equal_int "a leading term takes the preceding em" ~expected:125
    ~actual:
      (Spec.table_one_space ~before:19 ~after:27 ~before_size:half ~after_size:em
         ~before_solid:false ~after_solid:false);

  (* Table 2. *)
  (match Spec.table_two_cell 19 2 with
  | Some cell ->
    Check.equal_int "no break before a closing bracket, at any level" ~expected:0b1111
      ~actual:cell.Spec.break_levels;
    Check.ok "and the adjacency itself is possible" (not cell.Spec.break_prohibited)
  | None -> Check.ok "Table 2 states (cl-19, cl-02)" false);
  (match Spec.table_two_cell 19 19 with
  | Some cell ->
    Check.equal_int "two ideographs break freely" ~expected:0 ~actual:cell.Spec.break_levels
  | None -> Check.ok "Table 2 states (cl-19, cl-19)" false);
  (match Spec.table_two_cell 1 29 with
  | Some cell -> Check.ok "an impossible adjacency" cell.Spec.break_prohibited
  | None -> Check.ok "Table 2 states (cl-01, cl-29)" false);
  Check.ok "Table 2 has no cell for cl-17" (Spec.table_two_cell 17 19 = None);

  (* Tables 3 through 6 as ladders. *)
  (match Spec.ranged_cell Tables.table3 27 19 with
  | Some cell ->
    Check.equal_int "a Western-to-ideograph space reduces to an eighth em" ~expected:90
      ~actual:(match cell.Spec.ranged_limit with Some limit -> limit | None -> -1);
    Check.equal_int "at stage six" ~expected:6 ~actual:cell.Spec.ranged_stage;
    Check.ok "continuously" (not cell.Spec.ranged_two_valued)
  | None -> Check.ok "Table 3 states (cl-27, cl-19)" false);
  (match Spec.ranged_cell Tables.table3 2 0 with
  | Some cell ->
    Check.equal_int "a line-end closing bracket reduces to nothing" ~expected:0
      ~actual:(match cell.Spec.ranged_limit with Some limit -> limit | None -> -1);
    Check.equal_int "at stage two" ~expected:2 ~actual:cell.Spec.ranged_stage;
    Check.ok "and all at once" cell.Spec.ranged_two_valued
  | None -> Check.ok "Table 3 states (cl-02, line-end)" false);

  (* The Remarks vocabulary is closed. *)
  Check.returns "an empty cell" (fun () -> Spec.remark_of_text "");
  Check.returns "an advance" (fun () -> Spec.remark_of_text "\xe5\xad\x97\xe5\xb9\x85\xe3\x81\xaf\xe5\x8d\x8a\xe8\xa7\x92");
  Check.raises "a phrase nothing in Appendix A writes" (fun () ->
      Spec.remark_of_text "used in diagonal composition")
