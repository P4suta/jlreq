(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** The twenty-two places JLReq permits more than one answer.

    Nothing in this module is written down here. [spec/derived/questions.tsv] is the
    whole of it: one row per question, the answers it permits, the answer each of
    the five profiles selects, and the pairs of answers that cannot both be chosen.
    A [Style] is that table with one answer picked per row.

    That is the point of building it this way. The list of questions is a reading of
    the specification, and a second engine that retyped the list would be agreeing
    with the first engine's reading rather than with the document. Here a question
    that the derivation stops finding disappears from both engines at once.

    {1 Contradictions}

    Some answers exclude others: §C.3's very strict convention is stated in terms
    that the default answers to two other questions contradict. The [excludes]
    column records each such pair once, on whichever answer states it, and the
    relation is symmetric. {!build} refuses a combination that trips one -- the
    protocol calls a contradictory style an input error, not a layout with a
    diagnostic. *)

exception Invalid of string
(** Raised on an unknown question, an answer the question does not permit, or a
    combination the specification excludes. *)

let fail format = Printf.ksprintf (fun message -> raise (Invalid message)) format

type exclusion = {
  owner_answer : string;  (** The answer of {i this} question that excludes. *)
  other_question : string;
  other_answer : string;
}

type question = {
  name : string;  (** The dotted setting name, e.g. [kinsoku.level]. *)
  permits : string list;  (** Every answer, in the order §-order lists them. *)
  profile_answers : (string * string) list;  (** Profile name to answer. *)
  excludes : exclusion list;
}

(** The five profiles, and the column of [questions.tsv] each one reads.

    The profile names are the protocol's; the column names are the derivation's.
    [jlreq-2020] is the default and is JLReq's own preference wherever it states
    one. *)
let profile_columns =
  [
    ("jlreq-2020", "jlreq");
    ("jis-reading-2020", "jis_reading");
    ("book-2020", "book");
    ("magazine-2020", "magazine");
    ("newspaper-2020", "newspaper");
  ]

let words (text : string) : string list =
  List.filter (fun piece -> piece <> "") (String.split_on_char ' ' text)

(** [text] split on [separator], dropping empty pieces. *)
let split_on (separator : char) (text : string) : string list =
  List.filter (fun piece -> piece <> "") (String.split_on_char separator text)

let exclusions_of_text (question : string) (text : string) : exclusion list =
  List.map
    (fun entry ->
      match String.split_on_char '|' entry with
      | [ owner_answer; other_question; other_answer; _rule ] ->
        { owner_answer; other_question; other_answer }
      | _ -> fail "`%s` in %s's exclusions is not `answer|question|answer|rule`" entry question)
    (split_on ';' text)

let questions : question array =
  let file = Tables.questions in
  let column name = Tsv.column file name in
  let question_column = column "question"
  and permits_column = column "permits"
  and excludes_column = column "excludes" in
  let profile_column = List.map (fun (name, header) -> (name, column header)) profile_columns in
  Array.of_list
    (List.map
       (fun row ->
         let name = Tsv.field row question_column in
         let permits = words (Tsv.field row permits_column) in
         if permits = [] then fail "%s permits no answer" name;
         let profile_answers =
           List.map
             (fun (profile, index) ->
               let answer = Tsv.field row index in
               if not (List.mem answer permits) then
                 fail "profile %s answers %s with `%s`, which it does not permit" profile name
                   answer;
               (profile, answer))
             profile_column
         in
         let excludes = exclusions_of_text name (Tsv.field row excludes_column) in
         List.iter
           (fun exclusion ->
             if not (List.mem exclusion.owner_answer permits) then
               fail "%s excludes from `%s`, which it does not permit" name exclusion.owner_answer)
           excludes;
         { name; permits; profile_answers; excludes })
       file.Tsv.rows)

let count = Array.length questions

let index_of (name : string) : int =
  let rec search index =
    if index >= count then fail "`%s` is not a setting this specification has" name
    else if String.equal questions.(index).name name then index
    else search (index + 1)
  in
  search 0

type t = { answers : string array }
(** One answer per question, in [questions.tsv] order. *)

(** The answers a named profile selects. *)
let of_profile (profile : string) : t =
  {
    answers =
      Array.map
        (fun question ->
          match List.assoc_opt profile question.profile_answers with
          | Some answer -> answer
          | None -> fail "`%s` is not a style profile" profile)
        questions;
  }

(** [Style::default()]: the profile the protocol names [jlreq-2020].

    This is the answer set JLReq itself prefers wherever it states a preference,
    and this project's published reading where it states none. *)
let default () : t = of_profile "jlreq-2020"

(** The answer in force for one question. *)
let answer (style : t) (name : string) : string = style.answers.(index_of name)

(** [with_answer style name value] replaces one answer.

    The answer must be one the question permits; the protocol's schema enumerates
    the same set, so a value that gets this far and is still unknown means the two
    have drifted apart. *)
let with_answer (style : t) (name : string) (value : string) : t =
  let index = index_of name in
  if not (List.mem value questions.(index).permits) then
    fail "`%s` is not an answer %s permits" value name;
  let answers = Array.copy style.answers in
  answers.(index) <- value;
  { answers }

(** Refuse a combination the specification excludes.

    The relation is symmetric and recorded once, so both directions are checked
    from the one entry. *)
let check (style : t) : unit =
  Array.iteri
    (fun index question ->
      List.iter
        (fun exclusion ->
          if
            String.equal style.answers.(index) exclusion.owner_answer
            && String.equal (answer style exclusion.other_question) exclusion.other_answer
          then
            fail "%s `%s` excludes %s `%s`" question.name exclusion.owner_answer
              exclusion.other_question exclusion.other_answer)
        question.excludes)
    questions

(** A style built from a profile and a list of overrides, checked as a whole.

    Checking at the end rather than at each setter is what lets a caller state two
    settings that exclude each other's {i defaults} but not each other. *)
let build ?(profile = "jlreq-2020") (overrides : (string * string) list) : t =
  let style = List.fold_left (fun style (name, value) -> with_answer style name value)
      (of_profile profile) overrides
  in
  check style;
  style

(* ----------------------------------------------------------------------------- *)
(* The answers the pipeline asks for *)
(* ----------------------------------------------------------------------------- *)

(* Named accessors rather than bare strings at the call sites: a misspelled
   question raises here, once, instead of silently comparing false forever. *)

let kinsoku_level style = answer style "kinsoku.level"
let reduction_table style = answer style "adjustment.reduction_table"
let line_end_punctuation style = answer style "spacing.line_end_punctuation"
let line_end_full_stop_comma style = answer style "spacing.line_end_full_stop_comma"
let line_head_opening_bracket style = answer style "spacing.line_head_opening_bracket"
let ruby_overhang_kana style = answer style "ruby.overhang_kana"
let ruby_overhang_indent style = answer style "ruby.overhang_indent"
let ruby_alignment style = answer style "ruby.alignment"
let ruby_group_distribution style = answer style "ruby.group_distribution"
let ruby_jukugo_layout style = answer style "ruby.jukugo_layout"
let iteration_mark_at_line_head style = answer style "kinsoku.iteration_mark_at_line_head"
let hanging_punctuation style = answer style "adjustment.hanging_punctuation"
let grouped_numeral_before_western style = answer style "kinsoku.grouped_numeral_before_western"
let sentence_medial_dividing_mark style = answer style "spacing.sentence_medial_dividing_mark"

let japanese_latin_expansion_ceiling style =
  answer style "adjustment.japanese_latin_expansion_ceiling"

let expansion_order style = answer style "adjustment.expansion_order"
let adjustment_preference style = answer style "adjustment.preference"
let remainder style = answer style "adjustment.remainder"
let unlisted_code_point style = answer style "classification.unlisted_code_point"
let ambiguous_context style = answer style "classification.ambiguous_context"

let grouped_numeral_qualification style =
  answer style "classification.grouped_numeral_qualification"

let relaxation_mechanism style = answer style "kinsoku.relaxation_mechanism"

(** The §C.3 convention level as the bit Table 2's cells are indexed by. *)
let kinsoku_level_bit (style : t) : int =
  match kinsoku_level style with
  | "very-loose" -> 0b0001
  | "loose" -> 0b0010
  | "strict" -> 0b0100
  | "very-strict" -> 0b1000
  | other -> fail "`%s` is not a §C.3 convention level" other

(** The reduction matrix §D's answer selects. *)
let reduction_matrix (style : t) : Tables.matrix =
  match reduction_table style with
  | "table-3" -> Tables.table3
  | "table-4" -> Tables.table4
  | "table-5" -> Tables.table5
  | other -> fail "`%s` is not a reduction table" other

(** Check that the built table is the shape [questions.tsv] states.

    Called from [main] with the rest of the startup census. *)
let self_check () : unit =
  if count <> 22 then
    fail "the specification permits 22 choices, this build read %d" count;
  List.iter (fun (profile, _) -> ignore (of_profile profile)) profile_columns;
  Array.iter
    (fun question ->
      List.iter
        (fun exclusion ->
          let other = questions.(index_of exclusion.other_question) in
          if not (List.mem exclusion.other_answer other.permits) then
            fail "%s excludes %s `%s`, which %s does not permit" question.name
              exclusion.other_question exclusion.other_answer exclusion.other_question)
        question.excludes)
    questions;
  (* Every profile must itself be a legal combination. *)
  List.iter (fun (profile, _) -> check (of_profile profile)) profile_columns
