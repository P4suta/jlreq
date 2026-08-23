(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** The specification tables, built from the embedded TSV text.

    This module turns {!Jlreq_specdata} into the lookup structures the rest of the
    engine reads, and then -- before the engine has read a single byte of stdin --
    checks that what it built is the size and shape the specification says it is.
    {!self_check} is called from [main] and a failure exits 2.

    The census is not decoration. Every number below was measured against the files
    in [spec/] and is a claim about the specification, so a change in [spec/] that
    silently drops rows stops the engine at startup instead of producing a subtly
    wrong layout eighty-nine times. It is also the only defense this engine has
    against the embedding rule in [lib/specdata/dune] pasting the wrong file: a
    truncated [cat] would still compile.

    {1 Amounts}

    Every spacing amount is an exact multiple of 1/720 em, which is the unit the
    six matrices are transcribed in ([amounts-are-multiples-of-the-unit], ADR 0007).
    A quarter em is 180, a half em is 360, a full em is 720. Nothing here is a
    fraction at run time.

    {1 Locale}

    The captured matrices are read from the {b Japanese} transcriptions. The Rust
    engine reads the English ones. Two engines agreeing is only evidence when they
    were not fed the same keystrokes, and [test/] compares the two transcriptions
    directly so that the divergences the capture preambles record stay visible. *)

exception Invalid of string
(** Raised by the builders and by {!self_check}. *)

let fail format = Printf.ksprintf (fun message -> raise (Invalid message)) format

(* ----------------------------------------------------------------------------- *)
(* Amounts and class identifiers *)
(* ----------------------------------------------------------------------------- *)

(** The denominator every captured amount is exact in. *)
let unit_per_em = 720

(** The largest amount any matrix cell states. One em; nothing in Appendices B
    through E exceeds it. *)
let max_amount = unit_per_em

(** A character class as a number.

    [0] is the line edge -- [line-head] on the row axis of Tables 1 and 3 through
    5, [line-end] on their column axis. A character class is [1] through [30].

    §3.9.2 closes the set at thirty, but only twenty-eight of them have adjacency
    behavior: cl-17 (a ruby annotation) and cl-18 (a warichu) are structures
    rather than neighbors, so Appendix A enumerates them and no matrix axis
    carries them. The two predicates below keep that distinction, because reading
    [cl-17] out of Appendix A is correct and reading it off a matrix axis is a
    corrupt transcription. *)
type klass = int

(** Whether [value] is one of the thirty classes §3.9.2 names. *)
let is_class (value : klass) : bool = value >= 1 && value <= 30

(** Whether [value] is a class the six matrices give adjacency behavior to. *)
let has_adjacency (value : klass) : bool = is_class value && value <> 17 && value <> 18

(** Whether [value] may appear on a matrix axis: an adjacency class, or the line edge. *)
let is_axis_class (value : klass) : bool = value = 0 || has_adjacency value

(** [cl-07], [line-head] or [line-end] as a {!klass}. *)
let klass_of_label (label : string) : klass =
  if String.equal label "line-head" || String.equal label "line-end" then 0
  else if String.length label = 5 && String.sub label 0 3 = "cl-" then
    let digits = String.sub label 3 2 in
    match int_of_string_opt digits with
    | Some value when is_class value -> value
    | Some value -> fail "`%s` names class %d, and §3.9.2 closes the set at 30" label value
    | None -> fail "`%s` is not a class label" label
  else fail "`%s` is not a class label" label

(** The inverse of {!klass_of_label} on the row axis. *)
let row_label (value : klass) : string =
  if value = 0 then "line-head" else Printf.sprintf "cl-%02d" value

(** The inverse of {!klass_of_label} on the column axis. *)
let column_label (value : klass) : string =
  if value = 0 then "line-end" else Printf.sprintf "cl-%02d" value

(** One matrix coordinate as a single integer, so a pair can key a [Hashtbl]
    without allocating. Both components are below 64. *)
let coordinate (before : klass) (after : klass) : int = (before * 64) + after

(** A legend amount in 1/720 em.

    The legends write amounts as fractions of an em -- [1/4], [1/2], [3/4], [1/8] --
    and [0] for solid. A fraction that is not exact in 1/720 em is a violation of
    [amounts-are-multiples-of-the-unit] and is refused. *)
let amount_of_token (token : string) : int =
  let whole text =
    match int_of_string_opt text with
    | Some value -> value
    | None -> fail "`%s` is not an integer" text
  in
  let value =
    match String.index_opt token '/' with
    | None -> whole token * unit_per_em
    | Some slash ->
      let numerator = whole (String.sub token 0 slash) in
      let denominator =
        whole (String.sub token (slash + 1) (String.length token - slash - 1))
      in
      if denominator <= 0 then fail "`%s` divides by %d" token denominator;
      let scaled = numerator * unit_per_em in
      if scaled mod denominator <> 0 then
        fail "`%s` is not exactly representable in 1/%d em" token unit_per_em;
      scaled / denominator
  in
  if value < 0 || value > max_amount then
    fail "`%s` is %d/%d em, outside [0, 1] em" token value unit_per_em;
  value

(* ----------------------------------------------------------------------------- *)
(* Matrix cells *)
(* ----------------------------------------------------------------------------- *)

type side = Before | After

type term = {
  term_amount : int;  (** In 1/720 em. *)
  term_side : side;  (** Which neighbor's em the amount is taken from. *)
  term_hang : bool;  (** Ruby may extend over this space (§B.1). *)
}

(** A legend token.

    The vocabulary is the one the six legends publish, in the fraction notation
    rather than either language's words:

    {v
    any table     ×                   the adjacency is prohibited
                  blank               an empty cell
    Table 1       1/4 be + 1/4 af     amounts, each from one neighbor's em
                  1/2 be hang         ruby may extend over that space (§B.1)
                  ruby hang           ruby may extend over the character itself
    Table 2       not                 a break is prohibited at all four levels
                  not 3,4             prohibited at §C.3 levels 3 and 4 only
    Tables 3-6    1/2-0 stage 4       movable to a limit, at a priority stage
                  1/2=0 stage 2       two-valued: the amount or the limit (§3.1.9)
                  1/4 stage 3         rigid, optionally at a stage
                  residual            §3.8.4 step (d), Table 6 only
    v}

    A hyphen and an en dash are the same separator. A token outside this vocabulary
    is refused rather than ignored. *)
type cell =
  | Blank
  | Prohibited  (** [×] *)
  | No_break of int list
      (** [not], or [not 3,4]. The empty list means every §C.3 level. *)
  | Ruby_hang
  | Residual
  | Spacing of term list  (** Table 1. *)
  | Rigid of { amount : int; stage : int option }
  | Movable of {
      amount : int;
      limit : int;
      two_valued : bool;  (** [=] rather than [-]: the amount {i or} the limit. *)
      stage : int;
    }

(** How many priority stages a ladder may hold. §3.8.3's reduction ladder runs to
    six and §3.8.4's expansion ladder to four; the bound is deliberately loose and
    exists to catch a misread ordinal, not to encode the ladders. *)
let max_stage = 9

let starts_with ~prefix text =
  String.length text >= String.length prefix
  && String.equal (String.sub text 0 (String.length prefix)) prefix

let ends_with ~suffix text =
  let extra = String.length text - String.length suffix in
  extra >= 0 && String.equal (String.sub text extra (String.length suffix)) suffix

(** [text] split on [separator], keeping empty pieces. *)
let split_on (separator : string) (text : string) : string list =
  let width = String.length separator in
  if width = 0 then invalid_arg "Tables.split_on";
  let out = ref [] and start = ref 0 and index = ref 0 in
  let length = String.length text in
  while !index + width <= length do
    if String.equal (String.sub text !index width) separator then begin
      out := String.sub text !start (!index - !start) :: !out;
      index := !index + width;
      start := !index
    end
    else incr index
  done;
  out := String.sub text !start (length - !start) :: !out;
  List.rev !out

(** The words of [text], dropping runs of spaces. *)
let words (text : string) : string list =
  List.filter (fun piece -> piece <> "") (String.split_on_char ' ' text)

(** The offset and width of the first amount/limit separator in [head], if any.

    [-] and [=] are one byte; U+2013 EN DASH and U+2014 EM DASH are three, and the
    legends use them interchangeably with the hyphen. *)
let find_separator (head : string) : (int * int * bool) option =
  let length = String.length head in
  let rec search index =
    if index >= length then None
    else if head.[index] = '-' then Some (index, 1, false)
    else if head.[index] = '=' then Some (index, 1, true)
    else if
      index + 3 <= length
      && (String.equal (String.sub head index 3) "\xe2\x80\x93"
         || String.equal (String.sub head index 3) "\xe2\x80\x94")
    then Some (index, 3, false)
    else search (index + 1)
  in
  search 0

(** The [stage N] suffix and the head it qualifies. *)
let split_stage (token : string) : string * int option =
  let pieces = words token in
  let count = List.length pieces in
  if count < 2 then (token, None)
  else
    match (List.nth pieces (count - 2), List.nth pieces (count - 1)) with
    | "stage", ordinal -> (
      match int_of_string_opt ordinal with
      | Some stage when stage >= 1 && stage <= max_stage ->
        (String.concat " " (List.filteri (fun index _ -> index < count - 2) pieces), Some stage)
      | Some stage -> fail "`%s` states stage %d, outside [1, %d]" token stage max_stage
      | None -> fail "`%s` does not state an ordinal after `stage`" token)
    | _ -> (token, None)

(** One Table 1 term: an amount, the neighbor it is taken from, and whether ruby
    may hang over it. *)
let term_of_token (token : string) : term =
  match words token with
  | [ amount; side ] | [ amount; side; "hang" ] ->
    let term_side =
      match side with
      | "be" -> Before
      | "af" -> After
      | other -> fail "`%s` names the side `%s`, which is neither `be` nor `af`" token other
    in
    { term_amount = amount_of_token amount; term_side; term_hang = ends_with ~suffix:"hang" token }
  | _ -> fail "`%s` is not `<amount> be|af [hang]`" token

(** A legend token as a {!cell}. *)
let cell_of_token (token : string) : cell =
  if String.equal token "blank" then Blank
  else if String.equal token "\xc3\x97" (* U+00D7 MULTIPLICATION SIGN *) then Prohibited
  else if String.equal token "ruby hang" then Ruby_hang
  else if String.equal token "residual" then Residual
  else if String.equal token "not" then No_break []
  else if starts_with ~prefix:"not " token then begin
    let listed = String.sub token 4 (String.length token - 4) in
    let levels =
      List.map
        (fun piece ->
          match int_of_string_opt (String.trim piece) with
          | Some level when level >= 1 && level <= 4 -> level
          | _ -> fail "`%s` names a §C.3 level outside [1, 4]" token)
        (String.split_on_char ',' listed)
    in
    if levels = [] then fail "`%s` lists no level" token;
    No_break levels
  end
  else if List.exists (fun piece -> piece = "be" || piece = "af") (words token) then
    Spacing (List.map (fun part -> term_of_token (String.trim part)) (split_on "+" token))
  else
    let head, stage = split_stage token in
    let head = String.trim head in
    match find_separator head with
    | None -> Rigid { amount = amount_of_token head; stage }
    | Some (offset, width, two_valued) ->
      let amount = amount_of_token (String.trim (String.sub head 0 offset)) in
      let limit =
        amount_of_token
          (String.trim (String.sub head (offset + width) (String.length head - offset - width)))
      in
      let stage =
        match stage with
        | Some stage -> stage
        | None -> fail "`%s` states a limit without a stage" token
      in
      Movable { amount; limit; two_valued; stage }

(* ----------------------------------------------------------------------------- *)
(* The six matrices *)
(* ----------------------------------------------------------------------------- *)

type matrix = {
  number : int;
  row_axis : klass array;  (** The row axis, ascending; the line edge sorts first. *)
  column_axis : klass array;  (** The column axis, ascending. *)
  cells : (int, cell) Hashtbl.t;  (** Keyed by {!coordinate}. *)
  notes : (int, string) Hashtbl.t;  (** The qualifying appendix note, where a cell cites one. *)
}

let matrix_of_tsv (number : int) (text : string) : matrix =
  let file = Tsv.parse text in
  let table_column = Tsv.column file "table"
  and before_column = Tsv.column file "before"
  and after_column = Tsv.column file "after"
  and token_column = Tsv.column file "token"
  and note_column = Tsv.column file "note" in
  let cells = Hashtbl.create 1024 and notes = Hashtbl.create 64 in
  let row_axis = ref [] and column_axis = ref [] in
  let remember seen value = if not (List.mem value !seen) then seen := value :: !seen in
  List.iter
    (fun row ->
      let stated = Tsv.field row table_column in
      if not (String.equal stated (string_of_int number)) then
        fail "table %d holds a row labeled table `%s`" number stated;
      let axis what label =
        let value = klass_of_label label in
        if not (is_axis_class value) then
          fail "table %d has `%s` on its %s axis, which carries no adjacency" number label what;
        value
      in
      let before = axis "row" (Tsv.field row before_column) in
      let after = axis "column" (Tsv.field row after_column) in
      let key = coordinate before after in
      if Hashtbl.mem cells key then
        fail "table %d states (%s, %s) twice" number (row_label before) (column_label after);
      Hashtbl.add cells key (cell_of_token (Tsv.field row token_column));
      let note = Tsv.field row note_column in
      if note <> "" then Hashtbl.add notes key note;
      remember row_axis before;
      remember column_axis after)
    file.Tsv.rows;
  (* The axes are sorted rather than kept in discovery order. The two locale
     transcriptions walk the same matrix differently -- the Japanese files of
     Tables 1 and 5 run column-major and start Table 5 at `line-head` -- and the
     order rows happen to appear in is a property of the typing session, not of
     the specification. Sorting makes the axis the same datum on both sides, so
     `test_tables.ml` can compare them. *)
  let axis values = Array.of_list (List.sort compare values) in
  { number; row_axis = axis !row_axis; column_axis = axis !column_axis; cells; notes }

(** The cell at a coordinate, or [Blank] where the matrix has no such coordinate.

    Tables 2 and 6 carry no line-edge axis ([line-edge-axes-only-where-they-exist]),
    so asking them about class [0] is a question with no cell rather than an error. *)
let cell (table : matrix) (before : klass) (after : klass) : cell =
  match Hashtbl.find_opt table.cells (coordinate before after) with
  | Some found -> found
  | None -> Blank

(** Whether the matrix states a cell at this coordinate at all. *)
let states (table : matrix) (before : klass) (after : klass) : bool =
  Hashtbl.mem table.cells (coordinate before after)

(** The appendix note qualifying a cell, if it cites one. *)
let note (table : matrix) (before : klass) (after : klass) : string option =
  Hashtbl.find_opt table.notes (coordinate before after)

let table1 = matrix_of_tsv 1 Jlreq_specdata.Spec_data.table1
let table2 = matrix_of_tsv 2 Jlreq_specdata.Spec_data.table2
let table3 = matrix_of_tsv 3 Jlreq_specdata.Spec_data.table3
let table4 = matrix_of_tsv 4 Jlreq_specdata.Spec_data.table4
let table5 = matrix_of_tsv 5 Jlreq_specdata.Spec_data.table5
let table6 = matrix_of_tsv 6 Jlreq_specdata.Spec_data.table6
let matrices = [| table1; table2; table3; table4; table5; table6 |]

(* ----------------------------------------------------------------------------- *)
(* Appendix A *)
(* ----------------------------------------------------------------------------- *)

type listing = {
  listing_class : klass;
  listing_key : int array;  (** The code point sequence the row is keyed by. *)
  listing_key_text : string;  (** The key as Appendix A spells it, e.g. [304B 309A]. *)
  remark_en : string;
  remark_ja : string;
}

(** [304B 309A] as scalars. *)
let key_of_text (text : string) : int array =
  let pieces = words text in
  if pieces = [] then fail "`%s` is not a code point sequence" text;
  Array.of_list
    (List.map
       (fun piece ->
         match int_of_string_opt ("0x" ^ piece) with
         | Some value when value >= 0 && value <= 0x10FFFF && not (value >= 0xD800 && value <= 0xDFFF)
           -> value
         | _ -> fail "`%s` is not a Unicode scalar value" piece)
       pieces)

let appendix_a : listing list =
  let file = Tsv.parse Jlreq_specdata.Spec_data.appendix_a in
  let class_column = Tsv.column file "class"
  and key_column = Tsv.column file "key"
  and en_column = Tsv.column file "remark-en"
  and ja_column = Tsv.column file "remark-ja" in
  List.map
    (fun row ->
      let listing_key_text = Tsv.field row key_column in
      {
        listing_class = klass_of_label (Tsv.field row class_column);
        listing_key = key_of_text listing_key_text;
        listing_key_text;
        remark_en = Tsv.field row en_column;
        remark_ja = Tsv.field row ja_column;
      })
    file.Tsv.rows

(** Appendix A keyed by [(class, key)].

    §A.19 lists [216B] twice -- the row is duplicated in the published document --
    so the map is one entry shorter than the file is rows. The duplicate is
    identical in both appearances and is recorded here rather than resolved. *)
let appendix_a_listing : (string, listing) Hashtbl.t =
  let table = Hashtbl.create 2048 in
  List.iter
    (fun row ->
      let key = Printf.sprintf "%s\t%s" (row_label row.listing_class) row.listing_key_text in
      if not (Hashtbl.mem table key) then Hashtbl.add table key row)
    appendix_a;
  table

(** Every key Appendix A lists, without regard to which class listed it. *)
let appendix_a_keys : (string, unit) Hashtbl.t =
  let table = Hashtbl.create 2048 in
  List.iter
    (fun row -> if not (Hashtbl.mem table row.listing_key_text) then
       Hashtbl.add table row.listing_key_text ())
    appendix_a;
  table

(** The distinct Remarks cells, as [(English, Japanese)] pairs.

    Appendix A's Remarks column is a closed vocabulary: fourteen distinct pairs
    over 1687 rows, the empty pair among them. A cell outside the vocabulary is a
    revision of the specification, and this engine would rather stop than guess at
    it. *)
let appendix_a_remarks : (string * string) list =
  let seen = Hashtbl.create 32 in
  let out = ref [] in
  List.iter
    (fun row ->
      let pair = (row.remark_en, row.remark_ja) in
      if not (Hashtbl.mem seen pair) then begin
        Hashtbl.add seen pair ();
        out := pair :: !out
      end)
    appendix_a;
  List.rev !out

(* ----------------------------------------------------------------------------- *)
(* The derived Unicode tables *)
(* ----------------------------------------------------------------------------- *)

type fold = { fold_source : int; fold_target : int; fold_frame : string }

(** The Wide and Narrow compatibility decompositions, and nothing else. *)
let folding : fold list =
  let file = Tsv.parse Jlreq_specdata.Spec_data.folding in
  let source_column = Tsv.column file "source"
  and target_column = Tsv.column file "target"
  and frame_column = Tsv.column file "frame" in
  List.map
    (fun row ->
      let scalar text =
        match key_of_text text with
        | [| value |] -> value
        | _ -> fail "`%s` is not one scalar" text
      in
      let fold_frame = Tsv.field row frame_column in
      (match fold_frame with
      | "full-em" | "half-em" | "proportional" -> ()
      | other -> fail "`%s` is not a frame" other);
      {
        fold_source = scalar (Tsv.field row source_column);
        fold_target = scalar (Tsv.field row target_column);
        fold_frame;
      })
    file.Tsv.rows

let folding_map : (int, fold) Hashtbl.t =
  let table = Hashtbl.create 512 in
  List.iter (fun entry -> Hashtbl.replace table entry.fold_source entry) folding;
  table

type range = { first : int; last : int }

let range_list (text : string) : range list =
  let file = Tsv.parse text in
  let first_column = Tsv.column file "first" and last_column = Tsv.column file "last" in
  List.map
    (fun row ->
      let scalar column =
        match key_of_text (Tsv.field row column) with
        | [| value |] -> value
        | _ -> fail "a range bound is not one scalar"
      in
      let first = scalar first_column and last = scalar last_column in
      if first > last then fail "range %04X..%04X runs backwards" first last;
      { first; last })
    file.Tsv.rows

(** [Unified_Ideograph]: the members of cl-19 that §A.19 deliberately does not list. *)
let ideographs : range list = range_list Jlreq_specdata.Spec_data.ideographs

type script_range = { script : string; script_first : int; script_last : int }

(** [Script=Hiragana] and [Script=Katakana], which §C.2 note 3's small-kana fallback
    reads. *)
let scripts : script_range list =
  let file = Tsv.parse Jlreq_specdata.Spec_data.scripts in
  let script_column = Tsv.column file "script"
  and first_column = Tsv.column file "first"
  and last_column = Tsv.column file "last" in
  List.map
    (fun row ->
      let scalar column =
        match key_of_text (Tsv.field row column) with
        | [| value |] -> value
        | _ -> fail "a script range bound is not one scalar"
      in
      let script = Tsv.field row script_column in
      (match script with
      | "Hiragana" | "Katakana" -> ()
      | other -> fail "`%s` is not a script this engine reads" other);
      let script_first = scalar first_column and script_last = scalar last_column in
      if script_first > script_last then
        fail "script range %04X..%04X runs backwards" script_first script_last;
      { script; script_first; script_last })
    file.Tsv.rows

(* ----------------------------------------------------------------------------- *)
(* The class roster and the Style questions *)
(* ----------------------------------------------------------------------------- *)

type class_entry = {
  entry_class : klass;
  name_en : string;
  name_ja : string;
  enumeration : string;  (** The Appendix A section listing it, where one does. *)
}

let classes : class_entry list =
  let file = Tsv.parse Jlreq_specdata.Spec_data.classes in
  let class_column = Tsv.column file "class"
  and en_column = Tsv.column file "name_en"
  and ja_column = Tsv.column file "name_ja"
  and enumeration_column = Tsv.column file "enumeration" in
  List.map
    (fun row ->
      {
        entry_class = klass_of_label (Tsv.field row class_column);
        name_en = Tsv.field row en_column;
        name_ja = Tsv.field row ja_column;
        enumeration = Tsv.field row enumeration_column;
      })
    file.Tsv.rows

(** Every place JLReq permits more than one answer.

    Layer M0 keeps the rows as text: the Style resolution these encode is M1's
    work, and a half-built decoder here would be a guess with a type. *)
let questions : Tsv.t = Tsv.parse Jlreq_specdata.Spec_data.questions

(* ----------------------------------------------------------------------------- *)
(* The startup census *)
(* ----------------------------------------------------------------------------- *)

let expect what expected actual =
  if expected <> actual then
    fail "%s: the specification has %d, this build read %d" what expected actual

(** Check that what was built is the size and shape [spec/] states.

    Called from [main] before the first request is read. A failure is a build
    fault, not a request fault, so the engine exits 2 without having answered
    anything. *)
let self_check () : unit =
  (* Appendix A. *)
  expect "Appendix A rows" 1687 (List.length appendix_a);
  expect "Appendix A listings" 1686 (Hashtbl.length appendix_a_listing);
  expect "Appendix A distinct keys" 1133 (Hashtbl.length appendix_a_keys);
  expect "Appendix A distinct Remarks pairs" 14 (List.length appendix_a_remarks);
  List.iter
    (fun row ->
      if not (is_class row.listing_class) then
        fail "Appendix A lists class %d, which is not a character class" row.listing_class;
      if Array.length row.listing_key = 0 then fail "an Appendix A row has an empty key")
    appendix_a;

  (* The derived Unicode tables. *)
  expect "folding entries" 226 (List.length folding);
  expect "Unified_Ideograph ranges" 16 (List.length ideographs);
  expect "Hiragana and Katakana ranges" 22 (List.length scripts);

  (* The class roster and the Style questions. *)
  expect "character classes" 30 (List.length classes);
  expect "Style questions" 22 (Tsv.row_count questions);

  (* The six matrices. *)
  Array.iter
    (fun table ->
      let axis = if table.number = 2 || table.number = 6 then 28 else 29 in
      let expected = axis * axis in
      expect
        (Printf.sprintf "Table %d cells" table.number)
        expected (Hashtbl.length table.cells);
      expect
        (Printf.sprintf "Table %d row axis" table.number)
        axis (Array.length table.row_axis);
      expect
        (Printf.sprintf "Table %d column axis" table.number)
        axis (Array.length table.column_axis);
      Array.iter
        (fun value ->
          if not (is_axis_class value) then
            fail "Table %d has a row axis entry of %d" table.number value)
        table.row_axis;
      Array.iter
        (fun value ->
          if not (is_axis_class value) then
            fail "Table %d has a column axis entry of %d" table.number value)
        table.column_axis;
      (* Tables 2 and 6 carry no line-edge axis. *)
      let has_edge = Array.exists (fun value -> value = 0) in
      let edge_expected = table.number <> 2 && table.number <> 6 in
      if has_edge table.row_axis <> edge_expected || has_edge table.column_axis <> edge_expected then
        fail "Table %d disagrees with `line-edge-axes-only-where-they-exist`" table.number;
      (* Every axis pair is stated exactly once, so the matrix is complete. *)
      Array.iter
        (fun before ->
          Array.iter
            (fun after ->
              if not (states table before after) then
                fail "Table %d has no cell at (%s, %s)" table.number (row_label before)
                  (column_label after))
            table.column_axis)
        table.row_axis;
      (* Every amount is in [0, 1] em; every stage ordinal is in its ladder. *)
      let check_amount value =
        if value < 0 || value > max_amount then
          fail "Table %d states %d/%d em" table.number value unit_per_em
      in
      Hashtbl.iter
        (fun _ found ->
          match found with
          | Blank | Prohibited | Ruby_hang | Residual | No_break _ -> ()
          | Spacing terms -> List.iter (fun term -> check_amount term.term_amount) terms
          | Rigid { amount; stage } ->
            check_amount amount;
            Option.iter
              (fun stage ->
                if stage < 1 || stage > max_stage then
                  fail "Table %d states stage %d" table.number stage)
              stage
          | Movable { amount; limit; stage; _ } ->
            check_amount amount;
            check_amount limit;
            if stage < 1 || stage > max_stage then
              fail "Table %d states stage %d" table.number stage)
        table.cells)
    matrices;

  (* `residual` is Table 6's alone, and `not` is Table 2's alone. *)
  Array.iter
    (fun table ->
      Hashtbl.iter
        (fun _ found ->
          match found with
          | Residual when table.number <> 6 ->
            fail "Table %d states `residual`, which is §3.8.4 step (d)'s and Table 6's"
              table.number
          | No_break _ when table.number <> 2 ->
            fail "Table %d states `not`, which is Table 2's" table.number
          | Spacing _ when table.number <> 1 ->
            fail "Table %d states a Table 1 spacing token" table.number
          | _ -> ())
        table.cells)
    matrices
