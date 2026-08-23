(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** Classification and the matrix lookups the pipeline asks for.

    {!Tables} turns the files under [spec/] into cells and listings. This module is
    the layer above: it answers "which of §3.9.2's thirty classes is this
    occurrence of this text?" and "what does Table 1 say about that adjacency?".

    {1 The Remarks column is a vocabulary, not prose}

    Appendix A's Remarks cells qualify a listing two ways, and both narrow
    classification:

    - {b usage}: [横組で使用] lists the key for horizontal composition only,
      [縦組で使用] for vertical only;
    - {b advance}: [字幅は半角] says the listing is about the half-width form,
      [プロポーショナル] about the proportional one.

    Fourteen distinct cells cover all 1687 rows. They are parsed here rather than
    pattern-matched whole, and a phrase outside the vocabulary stops the engine:
    an unread qualification is a listing applied where the specification does not
    apply it, which is a wrong answer rather than a missing feature.

    [四分角] (a quarter em) and [三分角] (a third of an em) name advances the
    protocol has no [Frame] for -- it spells [full-em], [half-em] and
    [proportional] and nothing else -- so they qualify nothing here. A cell naming
    only those is read as stating no frame at all, which is how an unqualified cell
    reads, and every key carrying one is listed under some other class whose own
    Remarks decide the outcome.

    {1 Narrowing}

    A key is often listed by several classes: [1] is an ideographic character
    (§A.19), a grouped numeral (§A.24), part of a unit symbol (§A.25) and a Western
    character (§A.27). {!class_of} starts from every class that lists the key and
    narrows by usage, then by advance, then by the role the caller declared. Each
    stage keeps its result only when the result is non-empty, so a narrowing that
    would erase every candidate is not applied at all: JLReq lists a key under a
    class deliberately, and a qualification that excludes all of them is being read
    too strictly.

    The narrowing beyond that -- which classes count as structural, when a stated
    advance overrides an unstated one, how a key listed as both ideographic and
    Western resolves -- is not in JLReq or in [spec/]. It is this project's, and it
    is recorded in the comments at each step. *)

(** The denominator the captured matrices state amounts in. *)
let units_per_em = Tables.unit_per_em

exception Invalid of string

let fail format = Printf.ksprintf (fun message -> raise (Invalid message)) format

(* ----------------------------------------------------------------------------- *)
(* Class sets *)
(* ----------------------------------------------------------------------------- *)

(** A set of classes as a bit per class. Class [n] is bit [n - 1]; OCaml's native
    [int] is 63 bits, so all thirty fit with room to spare. *)
type class_set = int

let class_bit (klass : int) : class_set = 1 lsl (klass - 1)
let mem_class (set : class_set) (klass : int) : bool = set land class_bit klass <> 0

(** The lowest class in the set, or [None] when it is empty. *)
let first_class (set : class_set) : int option =
  let rec search klass =
    if klass > 30 then None else if mem_class set klass then Some klass else search (klass + 1)
  in
  search 1

(** The highest class in the set, or [None] when it is empty. *)
let last_class (set : class_set) : int option =
  let rec search klass =
    if klass < 1 then None else if mem_class set klass then Some klass else search (klass - 1)
  in
  search 30

(** [keep original narrowed] is [narrowed] unless it is empty.

    Every narrowing stage goes through here. A stage that excludes every candidate
    has misread its qualification, and the specification's own listing is the
    better answer than nothing. *)
let keep (original : class_set) (narrowed : class_set) : class_set =
  if narrowed = 0 then original else narrowed

(** The classes that exist only inside a structure: a reference mark's characters,
    an ornamented or ruby complex, a grouped numeral, a unit symbol, a warichu
    bracket, a tate-chu-yoko member.

    None of these is a class a bare code point falls into on its own -- each one
    describes a character's place in something the caller built -- so a key listed
    under one of them and under an ordinary class is the ordinary class unless the
    caller said otherwise. §3.9.2 names the classes; treating them as a group that
    loses to any ordinary class is this project's reading and is not stated in
    JLReq. *)
let construct_classes : class_set =
  List.fold_left
    (fun set klass -> set lor class_bit klass)
    0
    [ 20; 21; 22; 23; 24; 25; 28; 29; 30 ]

let opening_bracket = 1
let closing_bracket = 2
let middle_dot = 5
let full_stop = 6
let comma = 7
let inseparable = 8
let math_symbol = 17
let math_operator = 18
let ideograph = 19

(* ----------------------------------------------------------------------------- *)
(* The Remarks vocabulary *)
(* ----------------------------------------------------------------------------- *)

type usage =
  | Unqualified
  | Horizontal_only
  | Vertical_only

(** The frames a Remarks cell names, as a bit set. Zero means the cell states no
    advance, which permits every frame. *)
type frames = int

let frame_full_em : frames = 1
let frame_half_em : frames = 2
let frame_proportional : frames = 4
let frames_unstated : frames = 0

let frame_bit (frame : Model.frame) : frames =
  match frame with
  | Model.Full_em -> frame_full_em
  | Model.Half_em -> frame_half_em
  | Model.Proportional -> frame_proportional

type remark = {
  remark_usage : usage;
  remark_frames : frames;
}

let starts_at (text : string) (offset : int) (word : string) : bool =
  let width = String.length word in
  offset + width <= String.length text && String.equal (String.sub text offset width) word

(** One advance word.

    [全角] and [半角] are the two the protocol has frames for. [四分角] and
    [三分角] are advances it does not name, and contribute nothing rather than
    being guessed onto the nearest frame. *)
let advance_word = function
  | "全角" -> Some frame_full_em
  | "半角" -> Some frame_half_em
  | "プロポーショナル" -> Some frame_proportional
  | "四分角" | "三分角" -> Some frames_unstated
  | _ -> None

let advance_words = [ "全角"; "半角"; "プロポーショナル"; "四分角"; "三分角" ]

(** The phrases that qualify nothing: the label above a width, and the two asides
    Appendix A puts in the same cell as a qualification. *)
let inert_phrases =
  [
    "この文字の後ろにU+3035が配置される";
    "処理系によっては，U+2015 (HORIZONTAL BAR)にも，同様の振る舞いを実装しているものもある";
    "位取りの空白";
    "位取りのコンマ";
    "小数点";
  ]

(** One Remarks cell as a qualification.

    The cell is a sequence of phrases separated by a newline or an ideographic
    comma. Every phrase must be one this vocabulary knows; anything else raises. *)
let remark_of_text (text : string) : remark =
  let length = String.length text in
  let usage = ref Unqualified and collected = ref frames_unstated in
  let offset = ref 0 in
  let separator () =
    if starts_at text !offset "\n" then Some 1
    else if starts_at text !offset "，" then Some (String.length "，")
    else if starts_at text !offset " " then Some 1
    else None
  in
  (* An advance list: `半角又はプロポーショナル`, with or without a leading `字幅は`. *)
  let read_widths () =
    let rec step first =
      match List.find_opt (starts_at text !offset) advance_words with
      | None -> if first then false else fail "`%s` names no advance after `又は`" text
      | Some word ->
        offset := !offset + String.length word;
        (match advance_word word with
        | Some bits -> collected := !collected lor bits
        | None -> fail "`%s` names the advance `%s`" text word);
        if starts_at text !offset "又は" then begin
          offset := !offset + String.length "又は";
          step false
        end
        else true
    in
    step true
  in
  while !offset < length do
    match separator () with
    | Some width -> offset := !offset + width
    | None ->
      if starts_at text !offset "横組で使用" then begin
        usage := Horizontal_only;
        offset := !offset + String.length "横組で使用"
      end
      else if starts_at text !offset "縦組で使用" then begin
        usage := Vertical_only;
        offset := !offset + String.length "縦組で使用"
      end
      else if starts_at text !offset "字幅は" then begin
        offset := !offset + String.length "字幅は";
        if not (read_widths ()) then fail "`%s` states `字幅は` with no advance" text
      end
      else
        match List.find_opt (starts_at text !offset) inert_phrases with
        | Some phrase -> offset := !offset + String.length phrase
        | None ->
          if not (read_widths ()) then
            fail
              "the Remarks cell `%s` holds a phrase this engine does not read, at byte %d"
              text !offset
  done;
  { remark_usage = !usage; remark_frames = !collected }

(* ----------------------------------------------------------------------------- *)
(* Appendix A, keyed for lookup *)
(* ----------------------------------------------------------------------------- *)

type listing = {
  listing_class : int;
  listing_remark : remark;
}

(** A one- or two-scalar key as a single integer. Scalars are below 2^21. *)
let key_of (first : int) (second : int) : int = (first lsl 21) lor second

let listings_table : (int, listing list) Hashtbl.t =
  let table = Hashtbl.create 2048 in
  List.iter
    (fun (row : Tables.listing) ->
      let key =
        match row.Tables.listing_key with
        | [| single |] -> key_of single 0
        | [| a; b |] -> key_of a b
        | _ -> fail "an Appendix A key holds more than two code points"
      in
      let entry = { listing_class = row.Tables.listing_class;
                    listing_remark = remark_of_text row.Tables.remark_ja } in
      let existing = try Hashtbl.find table key with Not_found -> [] in
      if not (List.exists (fun other -> other.listing_class = entry.listing_class) existing) then
        Hashtbl.replace table key (existing @ [ entry ]))
    Tables.appendix_a;
  table

let listings (first : int) (second : int) : listing list =
  match Hashtbl.find_opt listings_table (key_of first second) with
  | Some found -> found
  | None -> []

let folding_target (scalar : int) : int option =
  match Hashtbl.find_opt Tables.folding_map scalar with
  | Some fold -> Some fold.Tables.fold_target
  | None -> None

let in_ranges (ranges : Tables.range list) (scalar : int) : bool =
  List.exists (fun range -> scalar >= range.Tables.first && scalar <= range.Tables.last) ranges

let is_ideograph (scalar : int) : bool = in_ranges Tables.ideographs scalar

let script_of (scalar : int) : string option =
  let rec search = function
    | [] -> None
    | range :: rest ->
      if scalar >= range.Tables.script_first && scalar <= range.Tables.script_last then
        Some range.Tables.script
      else search rest
  in
  search Tables.scripts

let is_hiragana (scalar : int) : bool = script_of scalar = Some "Hiragana"
let is_katakana (scalar : int) : bool = script_of scalar = Some "Katakana"

(** The listings a key is classified from: the ones Appendix A states literally,
    or -- for a single scalar Appendix A does not list -- the ones stated for its
    Wide or Narrow fold.

    Literal membership wins. U+3000 IDEOGRAPHIC SPACE is listed as cl-14 in its
    own right and folds to U+0020, which is cl-26; reading the fold first would
    make the ideographic space a Western word space. *)
let listings_for_candidate (first : int) (second : int) : listing list =
  let literal = listings first second in
  if literal <> [] || second <> 0 then literal
  else
    match folding_target first with
    | Some folded -> listings folded 0
    | None -> []

let candidates (first : int) (second : int) : class_set =
  let selected = listings_for_candidate first second in
  let classes =
    List.fold_left (fun set entry -> set lor class_bit entry.listing_class) 0 selected
  in
  (* §A.19 does not enumerate the unified ideographs; the Unicode property is the
     enumeration, and a key with it is cl-19 whether or not a row says so. *)
  if second = 0 && is_ideograph first && not (mem_class classes ideograph) then
    classes lor class_bit ideograph
  else classes

(* ----------------------------------------------------------------------------- *)
(* The three narrowing stages *)
(* ----------------------------------------------------------------------------- *)

let narrow_by_usage (classes : class_set) (first : int) (second : int)
    (mode : Model.writing_mode) : class_set =
  let permitted =
    List.fold_left
      (fun set entry ->
        let ok =
          match entry.listing_remark.remark_usage with
          | Unqualified -> true
          | Horizontal_only -> mode = Model.Horizontal_tb
          | Vertical_only -> mode = Model.Vertical_rl
        in
        if ok then set lor class_bit entry.listing_class else set)
      0
      (listings_for_candidate first second)
  in
  keep classes (classes land permitted)

(** Whether a class states its advance by the shape of the class itself rather
    than in a Remarks cell.

    Opening and closing brackets, middle dots, full stops and commas are the five
    classes JLReq sets in a half-em body with a half em of space beside it
    (§3.1.1). A caller who declares one of them full-em or half-em has said which
    reading applies without Appendix A having to. This is not stated in JLReq; it
    is what makes a half-width comma a comma rather than a grouped numeral's
    separator. *)
let stated_by_advance (klass : int) (frame : Model.frame) : bool =
  List.mem klass [ opening_bracket; closing_bracket; middle_dot; full_stop; comma ]
  && (frame = Model.Full_em || frame = Model.Half_em)

let narrow_by_frame (classes : class_set) (first : int) (second : int) (frame : Model.frame)
    : class_set =
  let wanted = frame_bit frame in
  let entries = listings_for_candidate first second in
  let permitted =
    List.fold_left
      (fun set entry ->
        let stated = entry.listing_remark.remark_frames in
        if stated = frames_unstated || stated land wanted <> 0 then
          set lor class_bit entry.listing_class
        else set)
      0 entries
  in
  let narrowed = ref (keep classes (classes land permitted)) in
  (* A class that states this advance outright beats one that merely does not
     forbid it -- but never beats a structural class, which the caller's role, not
     the glyph's width, decides. *)
  let explicit =
    List.fold_left
      (fun set entry ->
        let stated = entry.listing_remark.remark_frames in
        if
          entry.listing_class < 20
          && (stated land wanted <> 0 || stated_by_advance entry.listing_class frame)
        then set lor class_bit entry.listing_class
        else set)
      0 entries
  in
  if explicit <> 0 then
    narrowed := keep !narrowed (!narrowed land (explicit lor construct_classes));
  (* The five half-em classes are not what a proportional advance is describing. *)
  if frame = Model.Proportional then begin
    let without =
      !narrowed
      land lnot
             (class_bit opening_bracket lor class_bit closing_bracket lor class_bit middle_dot
            lor class_bit full_stop lor class_bit comma)
    in
    narrowed := keep !narrowed without
  end;
  (* Every Latin letter and digit is listed both as an ideographic character (the
     full-width form, §A.19) and as a Western character (§A.27). The frame says
     which one the caller shaped: proportional is Western, full-em is the
     full-width form, and a half-em advance is Western unless the key is also a
     grouped numeral, where the half-width digit is the grouped numeral's. *)
  if mem_class !narrowed ideograph && mem_class !narrowed 27 then
    narrowed :=
      (match frame with
      | Model.Proportional -> !narrowed land lnot (class_bit ideograph)
      | Model.Full_em -> !narrowed land lnot (class_bit 27)
      | Model.Half_em when mem_class !narrowed 24 -> !narrowed land lnot (class_bit ideograph)
      | Model.Half_em -> !narrowed);
  !narrowed

(** Whether Appendix A lists the single scalar under a class, following the Wide
    and Narrow folds when it is not listed literally. *)
let single_has_class (scalar : int) (klass : int) : bool =
  if klass = ideograph && is_ideograph scalar then true
  else
    let literal = listings scalar 0 in
    if literal <> [] then List.exists (fun entry -> entry.listing_class = klass) literal
    else
      match folding_target scalar with
      | Some folded -> List.exists (fun entry -> entry.listing_class = klass) (listings folded 0)
      | None -> false

let narrow_by_role (classes : class_set) (role : Model.role option) (scalar : int)
    (grouped_numeral_requires_role : bool) : class_set =
  let selected =
    match role with
    | Some (Model.Decimal_point | Model.Digit_group_separator | Model.Grouped_numeral) ->
      Some (class_bit 24)
    | Some (Model.Sentence_medial | Model.Sentence_terminator) -> Some (class_bit 4)
    | Some Model.Unit_symbol -> Some (class_bit 25)
    | Some Model.Warichu_bracket when single_has_class scalar opening_bracket ->
      Some (class_bit 28)
    | Some Model.Warichu_bracket when single_has_class scalar closing_bracket ->
      Some (class_bit 29)
    | _ -> None
  in
  match selected with
  | Some selected -> keep classes (classes land selected)
  | None ->
    (* `by-role` says a bare sequence of European numerals is Western text until
       the caller calls it a grouped numeral (docs/decisions/). *)
    if grouped_numeral_requires_role && mem_class classes 24 then class_bit 27 else classes

(** The class of one occurrence.

    [piece] is the source text of a cluster, [frame] and [role] are the caller's
    declarations for it, and the three flags are the Style answers §3.9.2 leaves
    open. *)
let class_of ~(piece : string) ~(frame : Model.frame) ~(role : Model.role option)
    ~(writing_mode : Model.writing_mode) ~(unlisted_is_ideographic : bool)
    ~(highest_ambiguous_class : bool) ~(grouped_numeral_requires_role : bool) : int =
  let scalars = Utf8.scalars piece in
  match scalars with
  | [] -> ideograph
  | _ :: _ :: _ :: _ -> if frame = Model.Proportional then 27 else ideograph
  | scalars ->
    let first = List.nth scalars 0 in
    let second = if List.length scalars = 2 then List.nth scalars 1 else 0 in
    let found = candidates first second in
    if found = 0 then
      (* §3.9.2 decides nothing about a code point nothing lists. Either the
         advance decides -- proportional is Western, anything else ideographic --
         or the caller has asked for ideographic outright. *)
      if unlisted_is_ideographic || frame <> Model.Proportional then ideograph else 27
    else begin
      let narrowed = narrow_by_usage found first second writing_mode in
      let narrowed = narrow_by_frame narrowed first second frame in
      let narrowed = narrow_by_role narrowed role first grouped_numeral_requires_role in
      let select set = if highest_ambiguous_class then last_class set else first_class set in
      match select (narrowed land lnot construct_classes) with
      | Some klass -> klass
      | None -> ( match select narrowed with Some klass -> klass | None -> ideograph)
    end

(** Whether two scalars are one of Appendix A's indivisible two-code-point keys. *)
let is_pair (first : int) (second : int) : bool = listings first second <> []

(* ----------------------------------------------------------------------------- *)
(* The matrices *)
(* ----------------------------------------------------------------------------- *)

(** [units] of an em, at [size], rounded away from zero.

    Rounding up rather than truncating is what keeps a quarter em of a 1001-unit em
    from being narrower than a quarter em of a 1000-unit one. *)
let scale_spec_units (size : int) (units : int) : int =
  let product = Int64.mul (Int64.of_int size) (Int64.of_int units) in
  let denominator = Int64.of_int units_per_em in
  let whole = Int64.div product denominator in
  let remainder = Int64.rem product denominator in
  let rounded = if Int64.equal remainder 0L then whole else Int64.add whole 1L in
  Num.clamp_i32 rounded

(** The two halves of a Table 1 cell: what the preceding character's em
    contributes and what the following character's does.

    A term is dropped when the neighbor it is measured from is set solid -- a
    middle dot inside a unit symbol is §B.2 note 12's zero on both sides, and the
    zero comes from ignoring the term rather than from a different cell. *)
let table_one_space_components ~(before : int) ~(after : int) ~(before_size : Model.size)
    ~(after_size : Model.size) ~(before_solid : bool) ~(after_solid : bool) : int * int =
  match Tables.cell Tables.table1 before after with
  | Tables.Spacing terms ->
    List.fold_left
      (fun (leading, trailing) (term : Tables.term) ->
        let is_trailing = term.Tables.term_side = Tables.After in
        if (is_trailing && after_solid) || ((not is_trailing) && before_solid) then
          (leading, trailing)
        else
          let size = if is_trailing then after_size.Model.inline else before_size.Model.inline in
          let amount = scale_spec_units size term.Tables.term_amount in
          if is_trailing then (leading, Num.i32_add trailing amount)
          else (Num.i32_add leading amount, trailing))
      (0, 0) terms
  | _ -> (0, 0)

let table_one_space ~(before : int) ~(after : int) ~(before_size : Model.size)
    ~(after_size : Model.size) ~(before_solid : bool) ~(after_solid : bool) : int =
  let leading, trailing =
    table_one_space_components ~before ~after ~before_size ~after_size ~before_solid ~after_solid
  in
  Num.i32_add leading trailing

(** Whether Table 1 states no space at all for an adjacency.

    An empty cell and a prohibited one are both blank here: neither contributes a
    term. The distinction matters to §3.1.6's optional quarter em, which is added
    only where the matrix itself is silent. *)
let table_one_is_blank ~(before : int) ~(after : int) : bool =
  match Tables.cell Tables.table1 before after with
  | Tables.Spacing (_ :: _) -> false
  | _ -> true

type break_cell = {
  break_prohibited : bool;  (** [×]: the adjacency itself cannot occur. *)
  break_levels : int;  (** The §C.3 levels a break is refused at, as a bit set. *)
}

let all_levels = 0b1111

(** Table 2 at a coordinate, or [None] where the matrix has no such coordinate --
    cl-17 and cl-18 carry no adjacency, and a break beside one is unconstrained. *)
let table_two_cell (before : int) (after : int) : break_cell option =
  if not (Tables.states Tables.table2 before after) then None
  else
    match Tables.cell Tables.table2 before after with
    | Tables.Prohibited -> Some { break_prohibited = true; break_levels = 0 }
    | Tables.No_break [] -> Some { break_prohibited = false; break_levels = all_levels }
    | Tables.No_break levels ->
      Some
        {
          break_prohibited = false;
          break_levels = List.fold_left (fun set level -> set lor (1 lsl (level - 1))) 0 levels;
        }
    | _ -> Some { break_prohibited = false; break_levels = 0 }

type ranged_cell = {
  ranged_limit : int option;  (** The amount the space may be moved to. *)
  ranged_two_valued : bool;  (** [=]: the amount or the limit, nothing between. *)
  ranged_residual : bool;  (** §3.8.4 step (d). *)
  ranged_stage : int;  (** The priority stage, or [0] for a cell with none. *)
}

(** One cell of Table 3, 4, 5 or 6. *)
let ranged_cell (table : Tables.matrix) (before : int) (after : int) : ranged_cell option =
  if not (Tables.states table before after) then None
  else
    match Tables.cell table before after with
    | Tables.Movable { limit; two_valued; stage; _ } ->
      Some
        {
          ranged_limit = Some limit;
          ranged_two_valued = two_valued;
          ranged_residual = false;
          ranged_stage = stage;
        }
    | Tables.Rigid { stage; _ } ->
      Some
        {
          ranged_limit = None;
          ranged_two_valued = false;
          ranged_residual = false;
          ranged_stage = (match stage with Some stage -> stage | None -> 0);
        }
    | Tables.Residual ->
      Some
        {
          ranged_limit = None;
          ranged_two_valued = false;
          ranged_residual = true;
          ranged_stage = 0;
        }
    | _ ->
      Some
        {
          ranged_limit = None;
          ranged_two_valued = false;
          ranged_residual = false;
          ranged_stage = 0;
        }
