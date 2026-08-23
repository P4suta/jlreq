(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** {!Jlreq.Style}: the twenty-two questions, the five profiles, and the
    combinations the specification excludes.

    Nothing here retypes the table. The checks are about the shape of what was
    built -- that every profile answers every question with something the question
    permits, that the exclusion relation is enforced in both directions -- plus a
    handful of spot values that would catch a column read off by one. *)

open Jlreq

let profiles = [ "jlreq-2020"; "jis-reading-2020"; "book-2020"; "magazine-2020"; "newspaper-2020" ]

let run () =
  Check.returns "the startup self-check passes" Style.self_check;
  Check.equal_int "the questions JLReq leaves open" ~expected:22 ~actual:Style.count;

  (* Every profile answers every question, and answers it legally. *)
  List.iter
    (fun profile ->
      let style = Style.of_profile profile in
      Array.iter
        (fun (question : Style.question) ->
          let answer = Style.answer style question.Style.name in
          Check.ok
            (Printf.sprintf "%s answers %s with something it permits" profile
               question.Style.name)
            (List.mem answer question.Style.permits))
        Style.questions;
      Check.returns (profile ^ " is itself a legal combination") (fun () -> Style.check style))
    profiles;
  Check.raises "a profile that does not exist" (fun () -> Style.of_profile "kumihan-2020");

  (* The default is JLReq's own reading. *)
  Check.equal_string "the default profile is jlreq-2020"
    ~expected:(Style.kinsoku_level (Style.of_profile "jlreq-2020"))
    ~actual:(Style.kinsoku_level (Style.default ()));

  (* Spot values, one per profile, on the settings that differ between them. *)
  let level profile = Style.kinsoku_level (Style.of_profile profile) in
  Check.equal_string "jlreq-2020 is strict" ~expected:"strict" ~actual:(level "jlreq-2020");
  Check.equal_string "magazine-2020 is loose" ~expected:"loose" ~actual:(level "magazine-2020");
  Check.equal_string "newspaper-2020 is very loose" ~expected:"very-loose"
    ~actual:(level "newspaper-2020");
  Check.equal_string "book-2020 reduces by Table 5" ~expected:"table-5"
    ~actual:(Style.reduction_table (Style.of_profile "book-2020"));
  Check.equal_string "jis-reading-2020 reduces by Table 4" ~expected:"table-4"
    ~actual:(Style.reduction_table (Style.of_profile "jis-reading-2020"));
  Check.equal_string "book-2020 hangs punctuation" ~expected:"hanging"
    ~actual:(Style.hanging_punctuation (Style.of_profile "book-2020"));
  Check.equal_string "jlreq-2020 does not hang punctuation" ~expected:"none"
    ~actual:(Style.hanging_punctuation (Style.of_profile "jlreq-2020"));
  Check.equal_string "book-2020 opens a bracket by pattern 3" ~expected:"pattern-3"
    ~actual:(Style.line_head_opening_bracket (Style.of_profile "book-2020"));
  Check.equal_string "jis-reading-2020 sets a line-end bracket solid" ~expected:"solid"
    ~actual:(Style.line_end_punctuation (Style.of_profile "jis-reading-2020"));

  (* The §C.3 level as Table 2 indexes it. *)
  let bit level =
    Style.kinsoku_level_bit (Style.build ~profile:"jlreq-2020"
        ([ ("kinsoku.level", level) ]
        @
        if String.equal level "very-strict" then
          [
            ("kinsoku.grouped_numeral_before_western", "unbreakable");
            ("kinsoku.relaxation_mechanism", "matrix");
          ]
        else []))
  in
  Check.equal_int "very loose is level 1" ~expected:0b0001 ~actual:(bit "very-loose");
  Check.equal_int "loose is level 2" ~expected:0b0010 ~actual:(bit "loose");
  Check.equal_int "strict is level 3" ~expected:0b0100 ~actual:(bit "strict");
  Check.equal_int "very strict is level 4" ~expected:0b1000 ~actual:(bit "very-strict");

  (* Setting one answer leaves the rest of the profile alone. *)
  let overridden = Style.build ~profile:"book-2020" [ ("adjustment.remainder", "trailing") ] in
  Check.equal_string "an override takes" ~expected:"trailing"
    ~actual:(Style.remainder overridden);
  Check.equal_string "and leaves the profile alone" ~expected:"table-5"
    ~actual:(Style.reduction_table overridden);

  Check.raises "a setting that does not exist" (fun () ->
      Style.build [ ("kinsoku.strictness", "strict") ]);
  Check.raises "an answer the setting does not permit" (fun () ->
      Style.build [ ("kinsoku.level", "very-very-strict") ]);

  (* §C.3's very strict convention is stated in terms that two of the default
     answers contradict, so it cannot be selected on its own. *)
  Check.raises "very strict kinsoku with the default answers" (fun () ->
      Style.build [ ("kinsoku.level", "very-strict") ]);
  Check.raises "very strict kinsoku with only one of the two settled" (fun () ->
      Style.build
        [
          ("kinsoku.level", "very-strict");
          ("kinsoku.grouped_numeral_before_western", "unbreakable");
        ]);
  Check.returns "very strict kinsoku with both settled" (fun () ->
      Style.build
        [
          ("kinsoku.level", "very-strict");
          ("kinsoku.grouped_numeral_before_western", "unbreakable");
          ("kinsoku.relaxation_mechanism", "matrix");
        ]);
  (* The relation is symmetric, and is recorded on only one of the two answers. *)
  Check.raises "the exclusion holds when it is stated last" (fun () ->
      Style.build
        [
          ("kinsoku.relaxation_mechanism", "reclassify");
          ("kinsoku.grouped_numeral_before_western", "unbreakable");
          ("kinsoku.level", "very-strict");
        ]);

  (* The reduction matrix a style selects is the one §D names. *)
  Check.equal_int "table-3 is Table 3" ~expected:3
    ~actual:(Style.reduction_matrix (Style.of_profile "jlreq-2020")).Tables.number;
  Check.equal_int "table-5 is Table 5" ~expected:5
    ~actual:(Style.reduction_matrix (Style.of_profile "book-2020")).Tables.number
