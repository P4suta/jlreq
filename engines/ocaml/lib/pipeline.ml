(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** Composition: a validated paragraph in, a layout out.

    The order of operations is the one JLReq describes, and the one the Rust
    implementation observes, because the two have to agree bit for bit:

    + choose the line breaks over the {i whole} paragraph, not greedily;
    + measure each chosen line, classify each adjacency, and take Table 1's space;
    + reduce the line where it is too wide (Table 3, 4 or 5), by priority stage;
    + open a justified line that is too narrow (Table 6), by priority stage;
    + place each cluster at a cursor, and shift the whole line for alignment;
    + report an overfull line and a widow.

    {1 What the search costs}

    The cost function is the part of this engine with no external source at all.
    JLReq says a line should look even and says nothing about how to weigh one
    uneven line against another, so every constant below is a policy: the square of
    the leftover measure, a thousand times that plus a fixed surcharge when the
    line overruns, a hundredth of it on the last line, twice it when the style
    prefers even texture over least adjustment, a fixed penalty for taking a
    discretionary break, and a very large one for a widowed last line. They are
    reproduced here deliberately, and they are the first thing to suspect when two
    engines choose different break points.

    Ties are broken by strict inequality, so the {i earliest} start that reaches a
    given break at equal cost wins: a later candidate never displaces an equal one.

    {1 Advance and cursor are not the same number}

    A placement's [advance] is what the cluster contributes before the line is
    adjusted -- its shaped advance plus Table 1's space after it. The cursor also
    carries the adjustment applied at that boundary, negative where the line was
    reduced and positive where it was opened up. On a line that fits, the two walks
    agree; on an adjusted line the placements sit closer together or further apart
    than their advances say. That is observable, and it is written down nowhere
    else. *)

open Model

let ( +| ) = Num.sadd
let ( -| ) = Num.ssub
let ( *| ) = Num.smul
let i64 = Int64.of_int
let lt (left : int64) (right : int64) : bool = Int64.compare left right < 0
let le (left : int64) (right : int64) : bool = Int64.compare left right <= 0
let min64 (left : int64) (right : int64) : int64 = if lt left right then left else right

(* ----------------------------------------------------------------------------- *)
(* Structures a cluster sits inside *)
(* ----------------------------------------------------------------------------- *)

let cluster_range (paragraph : Paragraph.t) (ordinal : int) : int * int =
  let cluster = paragraph.Paragraph.text.clusters.(ordinal) in
  (cluster.first, cluster.last)

let cluster_at (paragraph : Paragraph.t) (ordinal : int) : Model.cluster option =
  let clusters = paragraph.Paragraph.text.clusters in
  if ordinal < 0 || ordinal >= Array.length clusters then None else Some clusters.(ordinal)

(** The cluster ordinals a construct covers, when one matching [matches] covers
    the cluster at [ordinal] entirely. A construct that covers only part of a
    cluster covers none of it. *)
let enclosing (paragraph : Paragraph.t) (ordinal : int) (matches : Construct.kind -> bool) :
    (int * int) option =
  match cluster_at paragraph ordinal with
  | None -> None
  | Some cluster ->
    let rec search index =
      if index >= Array.length paragraph.Paragraph.constructs then None
      else
        let construct = paragraph.Paragraph.constructs.(index) in
        let first, last = construct.Construct.range in
        if matches construct.Construct.kind && first <= cluster.first && cluster.last <= last then
          Some
            ( Paragraph.cluster_index_at_or_after paragraph first,
              Paragraph.cluster_index_at_or_after paragraph last )
        else search (index + 1)
    in
    search 0

(** The cluster ordinals of the tate-chu-yoko run [ordinal] is a member of.

    Horizontal composition has none: §3.2.5 is a rule about setting a horizontal
    string inside a vertical line, and a paragraph that is already horizontal has
    nothing to turn. A tate-chu-yoko construct in a horizontal paragraph therefore
    leaves its characters entirely alone -- their class, their orientation and their
    spacing are what they would have been without it. *)
let tate_chu_yoko_range (paragraph : Paragraph.t) (ordinal : int) : (int * int) option =
  if paragraph.Paragraph.writing_mode <> Vertical_rl then None
  else enclosing paragraph ordinal (function Construct.Tate_chu_yoko -> true | _ -> false)

let formula_range (paragraph : Paragraph.t) (ordinal : int) : (int * int) option =
  enclosing paragraph ordinal (function Construct.Formula -> true | _ -> false)

(** Whether the boundary after [ordinal] falls {i inside} one tate-chu-yoko run.

    §C.2 note 13 and §E.2 note 12 are the same sentence about two different tables:
    two characters in tate-chu-yoko (cl-30) belonging to the same run are one thing
    on the line, so no line break falls between them and no space is opened between
    them, and the [(cl-30, cl-30)] cell each table states applies only where the two
    belong to {i different} runs. *)
let is_internal_tate_chu_yoko_boundary (paragraph : Paragraph.t) (ordinal : int) : bool =
  match tate_chu_yoko_range paragraph ordinal with
  | Some (_, last) -> ordinal + 1 < last
  | None -> false

(** How wide the run's members set, taken left to right and solid (§3.2.5).

    This is the run's extent {i across} the line rather than along it: the string is
    set horizontally, and only then is the whole of it placed on the vertical line.
    Each member contributes its own shaped advance, because "solid" is exactly the
    statement that nothing is added between them. *)
let tate_chu_yoko_run_width (paragraph : Paragraph.t) ~(first : int) ~(last : int) : int =
  let total = ref 0 in
  for member = first to last - 1 do
    match cluster_at paragraph member with
    | Some cluster -> total := Num.i32_add !total cluster.advance
    | None -> ()
  done;
  !total

(** Where the run's member at [ordinal] sits across the line, measured from the
    line's own block origin.

    §3.2.5: "set from left to right using solid setting, then align the whole string
    to the center of the vertical line". Centering the string on the line puts its
    first member half the run's width back from where an ordinary cluster of the
    line would sit, and every member after that follows the one before it by that
    one's own advance. Half is taken toward zero, so a run of an odd total width
    leans by one unit toward the line's own origin rather than away from it. *)
let tate_chu_yoko_member_offset (paragraph : Paragraph.t) (ordinal : int) ~(first : int)
    ~(last : int) : int =
  let preceding = ref 0 in
  for member = first to ordinal - 1 do
    match cluster_at paragraph member with
    | Some cluster -> preceding := Num.i32_add !preceding cluster.advance
    | None -> ()
  done;
  Num.i32_sub !preceding (tate_chu_yoko_run_width paragraph ~first ~last / 2)

let is_internal_jidori_boundary (paragraph : Paragraph.t) (ordinal : int) : bool =
  match cluster_at paragraph ordinal with
  | None -> false
  | Some cluster ->
    Array.exists
      (fun (construct : Construct.t) ->
        match construct.Construct.kind with
        | Construct.Jidori _ ->
          let first, last = construct.Construct.range in
          first < cluster.last && cluster.last < last
        | _ -> false)
      paragraph.Paragraph.constructs

let is_internal_furawake_offset (paragraph : Paragraph.t) (offset : int) : bool =
  Array.exists
    (fun (construct : Construct.t) ->
      match construct.Construct.kind with
      | Construct.Furawake _ ->
        let first, last = construct.Construct.range in
        first < offset && offset < last
      | _ -> false)
    paragraph.Paragraph.constructs

(* ----------------------------------------------------------------------------- *)
(* Classification in context *)
(* ----------------------------------------------------------------------------- *)

let single_scalar (paragraph : Paragraph.t) (cluster : Model.cluster) : int option =
  Model.single_scalar paragraph.Paragraph.text cluster

let piece_of (paragraph : Paragraph.t) (cluster : Model.cluster) : string =
  Model.cluster_piece paragraph.Paragraph.text cluster

let size_of (paragraph : Paragraph.t) (cluster : Model.cluster) : Model.size =
  Model.cluster_size paragraph.Paragraph.text cluster

let frame_of (paragraph : Paragraph.t) (cluster : Model.cluster) : Model.frame =
  Model.cluster_frame paragraph.Paragraph.text cluster

(** The class of the cluster at [ordinal].

    A structure decides first: a tate-chu-yoko member is cl-30 whatever it holds,
    and a ruby base, an ornamented complex or a reference mark's characters take
    the class §3.9.2 gives that structure. Only a cluster in no structure is
    classified from its own text. *)
let class_of_cluster (paragraph : Paragraph.t) (style : Style.t) (ordinal : int) : int =
  if tate_chu_yoko_range paragraph ordinal <> None then 30
  else
    let cluster = paragraph.Paragraph.text.clusters.(ordinal) in
    match
      Construct.structural_class paragraph.Paragraph.constructs (cluster.first, cluster.last)
    with
    | Some klass -> klass
    | None ->
      Spec.class_of ~piece:(piece_of paragraph cluster) ~frame:(frame_of paragraph cluster)
        ~role:cluster.role ~writing_mode:paragraph.Paragraph.writing_mode
        ~unlisted_is_ideographic:(Style.unlisted_code_point style = "ideographic")
        ~highest_ambiguous_class:(Style.ambiguous_context style = "highest-class")
        ~grouped_numeral_requires_role:(Style.grouped_numeral_qualification style = "by-role")

let space = 0x0020
let tab = 0x0009
let ideographic_space = 0x3000
let ideographic_comma = 0x3001
let katakana_middle_dot = 0x30FB
let iteration_mark = 0x3005
let em_dash = 0x2014
let horizontal_ellipsis = 0x2026
let two_dot_leader = 0x2025
let kana_repeat_upper = 0x3033
let kana_repeat_voiced_upper = 0x3034
let kana_repeat_lower = 0x3035
let percent_sign = 0x0025
let fullwidth_percent_sign = 0xFF05

let is_opening_bracket scalar = Spec.single_has_class scalar Spec.opening_bracket
let is_closing_bracket scalar = Spec.single_has_class scalar Spec.closing_bracket
let is_full_stop scalar = Spec.single_has_class scalar Spec.full_stop
let is_comma scalar = Spec.single_has_class scalar Spec.comma
let is_middle_dot scalar = Spec.single_has_class scalar Spec.middle_dot

let scalar_satisfies (paragraph : Paragraph.t) (cluster : Model.cluster) (predicate : int -> bool)
    : bool =
  match single_scalar paragraph cluster with Some scalar -> predicate scalar | None -> false

(** Whether an occurrence of a punctuation mark is set solid because of the role
    the caller gave it.

    §B.2 note 12: a katakana middle dot used {i as} a unit symbol takes no space on
    either side. §3.1.3: the same mark used as a decimal point, and an ideographic
    comma used as a digit group separator, take none in vertical composition. A
    warichu's own brackets sit solid against the note's text. *)
let contextual_punctuation_is_solid (paragraph : Paragraph.t) (cluster : Model.cluster)
    (scalar : int) : bool =
  match cluster.role with
  | Some Warichu_bracket -> is_opening_bracket scalar || is_closing_bracket scalar
  | Some Decimal_point ->
    scalar = katakana_middle_dot && paragraph.Paragraph.writing_mode = Vertical_rl
  | Some Digit_group_separator ->
    scalar = ideographic_comma && paragraph.Paragraph.writing_mode = Vertical_rl
  | Some (Grouped_numeral | Unit_symbol | Formula) -> scalar = katakana_middle_dot
  | _ -> false

let is_solid (paragraph : Paragraph.t) (cluster : Model.cluster) : bool =
  scalar_satisfies paragraph cluster (contextual_punctuation_is_solid paragraph cluster)

let is_western_word_space (paragraph : Paragraph.t) (ordinal : int) : bool =
  match cluster_at paragraph ordinal with
  | None -> false
  | Some cluster ->
    String.equal (piece_of paragraph cluster) " "
    && frame_of paragraph cluster = Proportional
    && (cluster.role = None || cluster.role = Some Text)

let is_math_token_cluster (paragraph : Paragraph.t) (cluster : Model.cluster) : bool =
  scalar_satisfies paragraph cluster Construct.is_math_token

(** §C.2 note 5's {i kinds} of inseparable character, as §C.3's very loose level
    enumerates them: each mark is its own kind, and the three code points of the
    vertical kana repeat mark are one kind between them
    (docs/decisions/inseparable-character-kind.md). Two occurrences of the same kind
    are read as one character, which is what §C.2 note 5 refuses to break and what
    §E.2 note 4 refuses to open. *)
let cl_08_same_kind (before : int option) (after : int option) : bool =
  match (before, after) with
  | Some left, Some right when left = right -> true
  | Some left, Some right ->
    let repeat scalar =
      scalar = kana_repeat_upper || scalar = kana_repeat_voiced_upper
      || scalar = kana_repeat_lower
    in
    repeat left && repeat right
  | _ -> false

(** The ten European numerals, read from the occurrence's own key rather than from
    a role the caller declares (docs/decisions/european-numeral-by-code-point.md).
    §C.2 note 11 and §E.2 note 10 both name them, and both read them here. *)
let is_european_numeral (scalar : int option) : bool =
  match scalar with Some scalar -> scalar >= 0x30 && scalar <= 0x39 | None -> false

(* ----------------------------------------------------------------------------- *)
(* Table 1: the space after a cluster *)
(* ----------------------------------------------------------------------------- *)

(** §3.7.4's spacing inside and around a formula, or [None] where no formula is
    involved in this boundary. *)
let formula_boundary_space_after (paragraph : Paragraph.t) (ordinal : int) : int option =
  let current_formula = formula_range paragraph ordinal in
  let following_formula = formula_range paragraph (ordinal + 1) in
  if current_formula = None && following_formula = None then None
  else
    match (cluster_at paragraph ordinal, cluster_at paragraph (ordinal + 1)) with
    | Some current, Some following ->
      let count = Array.length paragraph.Paragraph.text.clusters in
      let endpoint_needs_quarter cluster =
        (not (is_math_token_cluster paragraph cluster))
        && (frame_of paragraph cluster = Proportional || cluster.role = Some Grouped_numeral)
      in
      let japanese_neighbor cluster =
        if frame_of paragraph cluster = Proportional then false
        else
          scalar_satisfies paragraph cluster (fun scalar ->
              scalar <> space && scalar <> tab && scalar <> ideographic_space
              && (not (is_opening_bracket scalar))
              && (not (is_closing_bracket scalar))
              && (not (is_full_stop scalar))
              && (not (is_comma scalar))
              && (not (is_middle_dot scalar))
              && not (Construct.is_math_token scalar))
      in
      let outer outside endpoint =
        if japanese_neighbor outside && endpoint_needs_quarter endpoint then
          Model.quarter_inline_size paragraph.Paragraph.text endpoint
        else 0
      in
      (match (current_formula, following_formula) with
      | Some current_range, Some following_range when current_range = following_range ->
        if fst current_range = 0 && snd current_range = count then begin
          let current_symbol =
            scalar_satisfies paragraph current Construct.is_math_symbol
          in
          let following_symbol =
            scalar_satisfies paragraph following Construct.is_math_symbol
          in
          if current_symbol <> following_symbol then
            Some
              (Model.quarter_inline_size paragraph.Paragraph.text
                 (if current_symbol then current else following))
          else Some 0
        end
        else Some 0
      | None, Some (first, _) when first = ordinal + 1 -> Some (outer current following)
      | Some (_, last), None when last = ordinal + 1 -> Some (outer following current)
      | _ -> Some 0)
    | _ -> None

(** §3.2.5's spacing beside a tate-chu-yoko run, as Table 1's two components, or
    [None] where no run stands on either side of the boundary.

    §3.2.5 states four amounts and nothing else. A run takes a half em after a comma
    (cl-07), after a closing bracket (cl-02) and after a full stop (cl-06); it takes
    a half em before an opening bracket (cl-01); and it is solid against everything
    else, hiragana, katakana and ideographic characters included. Table 1's cl-30 row
    and column state those four coordinates and six more -- a quarter em against a
    middle dot (cl-05) and against the four Western and ornamented classes, in both
    directions -- and §3.2.5's own Note says that the table is the complete
    statement. The reference implementation reads the prose as exhaustive and the
    Note as a pointer rather than a widening, and this engine matches it. The
    disagreement is between two sentences of JLReq and is settled by neither; see
    README.md, "Observable policies with no written source".

    Two characters of one run are solid because §3.2.5 sets the string solid, and
    two characters of {i different} runs are solid because Table 1 says so at
    [(cl-30, cl-30)] -- the one coordinate where the prose and the table already
    agree on nothing being added.

    The classes are the ones §3.9.2 gives the occurrences, so an opening bracket
    that is itself a run's first member is cl-30 and not cl-01: what a run stands
    against is the run, never the character the run happens to begin with. *)
let tate_chu_yoko_boundary_components (paragraph : Paragraph.t) (style : Style.t)
    (ordinal : int) : (int * int) option =
  let text = paragraph.Paragraph.text in
  let half_of_cluster cluster = Model.half_inline_size text cluster in
  match tate_chu_yoko_range paragraph ordinal with
  | Some (_, last) ->
    if ordinal + 1 <> last then Some (0, 0)
    else (
      match cluster_at paragraph (ordinal + 1) with
      | Some following when class_of_cluster paragraph style (ordinal + 1) = Spec.opening_bracket
        ->
        Some (0, half_of_cluster following)
      | _ -> Some (0, 0))
  | None -> (
    match (cluster_at paragraph ordinal, tate_chu_yoko_range paragraph (ordinal + 1)) with
    | Some current, Some (first, _) when first = ordinal + 1 ->
      let klass = class_of_cluster paragraph style ordinal in
      if klass = Spec.closing_bracket || klass = Spec.full_stop || klass = Spec.comma then
        Some (half_of_cluster current, 0)
      else Some (0, 0)
    | _ -> None)

(** Table 1's space at the boundary after [ordinal], with the answers §3.1.6 leaves
    open applied.

    Two of those answers are not in the matrix at all. A dividing punctuation mark
    the caller calls a sentence terminator takes a full em after it -- withdrawn
    entirely when a closing bracket follows -- and one the caller calls sentence
    medial takes an optional quarter em on each side, added only where the matrix
    itself says nothing. *)
let ordinary_boundary_space_after (paragraph : Paragraph.t) (style : Style.t) (ordinal : int) :
    int =
  match (cluster_at paragraph ordinal, cluster_at paragraph (ordinal + 1)) with
  | Some current, Some following ->
    let before = class_of_cluster paragraph style ordinal in
    let after = class_of_cluster paragraph style (ordinal + 1) in
    let current_size = size_of paragraph current in
    let following_size = size_of paragraph following in
    if before = 4 && current.role = Some Sentence_terminator then
      if after = Spec.closing_bracket then 0 else current_size.inline
    else begin
      let table =
        Spec.table_one_space ~before ~after ~before_size:current_size ~after_size:following_size
          ~before_solid:(is_solid paragraph current) ~after_solid:(is_solid paragraph following)
      in
      if
        (not (Spec.table_one_is_blank ~before ~after))
        || Style.sentence_medial_dividing_mark style <> "quarter-em"
      then table
      else
        let before_quarter =
          if before = 4 && current.role = Some Sentence_medial then
            Model.quarter_inline_size paragraph.Paragraph.text current
          else 0
        in
        let after_quarter =
          if after = 4 && following.role = Some Sentence_medial then
            Model.quarter_inline_size paragraph.Paragraph.text following
          else 0
        in
        Num.i32_add table (Num.i32_add before_quarter after_quarter)
    end
  | _ -> 0

(** The space at the boundary after [ordinal]: §3.2.5's where a tate-chu-yoko run is
    on one side of it, §3.7.4's where a formula is, and Table 1's otherwise.

    Only the space {i set on the line} is overridden. §3.8.3's and §3.8.4's ladders
    read Tables 3 through 6 and Table 1's own two components at face value, which at
    a cl-30 coordinate is not the same number; that is deliberate and it is where the
    two readings of §3.2.5 become observable. *)
let boundary_space_after (paragraph : Paragraph.t) (style : Style.t) (ordinal : int) : int =
  match tate_chu_yoko_boundary_components paragraph style ordinal with
  | Some (leading, trailing) -> Num.i32_add leading trailing
  | None -> (
    match formula_boundary_space_after paragraph ordinal with
    | Some amount -> amount
    | None -> ordinary_boundary_space_after paragraph style ordinal)

(** The space Table 1 puts between the last cluster of a line and the line end.

    §B.2 notes 2 and 6 make two of those cells a question the style answers: the
    space after a closing bracket, and the space after a comma, are a half em by
    JLReq's preference and solid by the JIS reading. Answering solid removes the
    space rather than shrinking it. *)
let line_end_space_after (paragraph : Paragraph.t) (style : Style.t) (ordinal : int) : int =
  match cluster_at paragraph ordinal with
  | None -> 0
  | Some cluster ->
    let klass = class_of_cluster paragraph style ordinal in
    if
      (klass = Spec.closing_bracket && Style.line_end_punctuation style = "solid")
      || (klass = Spec.comma && Style.line_end_full_stop_comma style = "jis")
    then 0
    else
      let size = size_of paragraph cluster in
      Spec.table_one_space ~before:klass ~after:0 ~before_size:size ~after_size:size
        ~before_solid:(is_solid paragraph cluster) ~after_solid:false

(* ----------------------------------------------------------------------------- *)
(* Advances *)
(* ----------------------------------------------------------------------------- *)

(** A cluster's own contribution to the line, before any spacing.

    A tate-chu-yoko run is one thing on the line, and takes up along the line what
    its tallest member is across one: the string inside it is set horizontally, so
    what the vertical line sees is that string's height. The whole of it is charged
    to the run's {i last} member and nothing to the members before, so that the
    cursor stands still while a run is placed and moves on once, past the run and
    past the space Table 1 puts after it -- which is stated at the run's last
    boundary and nowhere else.

    A math token inside a formula occupies its em rather than its shaped advance.
    Everything else is what the shaper measured. *)
let cluster_body_advance (paragraph : Paragraph.t) (ordinal : int) : int =
  match tate_chu_yoko_range paragraph ordinal with
  | Some (first, last) ->
    if ordinal + 1 <> last then 0
    else begin
      let widest = ref 0 in
      for member = first to last - 1 do
        match cluster_at paragraph member with
        | Some cluster ->
          let size = size_of paragraph cluster in
          if size.block > !widest then widest := size.block
        | None -> ()
      done;
      !widest
    end
  | None ->
    let cluster = paragraph.Paragraph.text.clusters.(ordinal) in
    if formula_range paragraph ordinal <> None && is_math_token_cluster paragraph cluster then
      (size_of paragraph cluster).inline
    else cluster.advance

(** A cluster's advance as {i this} line sets it.

    Three things depend on where the line ends. A Western word space at either edge
    of a line disappears entirely (§3.2.2). The last cluster takes Table 1's
    line-end cell rather than the cell for its neighbor. Everything between takes
    the ordinary boundary space.

    A tate-chu-yoko member is not asked where the line ends at all. §3.2.5 makes the
    run one thing on the line, so what follows it is stated by §3.2.5 against the
    next character of the {i paragraph} -- the run keeps the half em it takes before
    an opening bracket even when the bracket is the first thing on the next line --
    and Table 1's line-end column, which states nothing at cl-30 anyway, is not
    consulted. *)
let cluster_advance_on_line (paragraph : Paragraph.t) (style : Style.t) (ordinal : int)
    ~(line_start : int) ~(line_end : int) : int =
  if is_western_word_space paragraph ordinal && (ordinal = line_start || ordinal + 1 = line_end)
  then 0
  else if tate_chu_yoko_range paragraph ordinal <> None then
    Num.i32_add
      (cluster_body_advance paragraph ordinal)
      (boundary_space_after paragraph style ordinal)
  else if ordinal + 1 = line_end then
    Num.i32_add
      (cluster_body_advance paragraph ordinal)
      (line_end_space_after paragraph style ordinal)
  else
    Num.i32_add
      (cluster_body_advance paragraph ordinal)
      (boundary_space_after paragraph style ordinal)

(* ----------------------------------------------------------------------------- *)
(* Line reduction (§3.8.3, Tables 3 through 5) *)
(* ----------------------------------------------------------------------------- *)

type reduction_site = {
  site_boundary : int;  (** The boundary's index within the line. *)
  site_weight : int;  (** The em the amount was measured from. *)
  site_capacity : int;  (** How much may be taken away here. *)
  site_stage : int;  (** The priority stage §3.8.3 takes it at. *)
  site_discrete : bool;  (** [=] rather than [-]: all of it or none of it. *)
}

let push_site sites ~boundary ~weight ~capacity ~stage ~discrete =
  if capacity > 0 && stage <> 0 then
    sites :=
      {
        site_boundary = boundary;
        site_weight = weight;
        site_capacity = capacity;
        site_stage = stage;
        site_discrete = discrete;
      }
      :: !sites

(** The adjacencies §D's notes take out of the matrix and give their own ladders:
    two middle dots, and a full stop or comma before a middle dot. Each reduces its
    two halves separately, at a stage no single cell can express. Returns whether
    the adjacency was one of them. *)
let append_special_reduction_sites (table : string) ~(before : int) ~(after : int)
    ~(components : int * int) ~(weights : int * int) ~(boundary : int) sites : bool =
  let leading, trailing = components in
  let leading_weight, trailing_weight = weights in
  let push_leading floor stage =
    push_site sites ~boundary ~weight:leading_weight ~capacity:(Num.i32_sub leading floor) ~stage
      ~discrete:false
  in
  let push_trailing floor stage =
    push_site sites ~boundary ~weight:trailing_weight ~capacity:(Num.i32_sub trailing floor) ~stage
      ~discrete:false
  in
  match (before, after, table) with
  | 5, 5, "table-3" ->
    push_leading 0 4;
    push_trailing 0 4;
    true
  | 5, 5, "table-4" ->
    push_leading 0 2;
    push_trailing 0 2;
    true
  | (5 | 6), 5, "table-5" -> true
  | 6, 5, "table-3" ->
    push_trailing 0 4;
    true
  | (6 | 7), 5, "table-4" ->
    push_trailing 0 2;
    true
  | 7, 5, "table-3" ->
    push_leading 0 5;
    push_trailing 0 4;
    true
  | 7, 5, "table-5" ->
    push_leading (Model.quarter_of leading_weight) 3;
    true
  | _ -> false

let append_table_reduction_sites (paragraph : Paragraph.t) (style : Style.t) (ordinal : int)
    (boundary : int) sites : unit =
  match (cluster_at paragraph ordinal, cluster_at paragraph (ordinal + 1)) with
  | Some before_cluster, Some after_cluster -> (
    let before = class_of_cluster paragraph style ordinal in
    let after = class_of_cluster paragraph style (ordinal + 1) in
    let before_size = size_of paragraph before_cluster in
    let after_size = size_of paragraph after_cluster in
    (* Table 1 as the matrix states it, not as §3.2.5 overrides it. Tables 3 through
       5 state their own cl-30 cells -- a quarter em against a middle dot, an eighth
       against the Western classes -- and the ladder reads them at face value even
       where §3.2.5 put no space at that boundary for them to take back. The
       observable consequence is a run that ends up a quarter em inside the character
       before it on a line that had to give space back; it is the reference
       implementation's answer and it is written down nowhere. See README.md,
       "Observable policies with no written source". *)
    let components =
      Spec.table_one_space_components ~before ~after ~before_size ~after_size
        ~before_solid:(is_solid paragraph before_cluster)
        ~after_solid:(is_solid paragraph after_cluster)
    in
    if
      append_special_reduction_sites (Style.reduction_table style) ~before ~after ~components
        ~weights:(before_size.inline, after_size.inline) ~boundary sites
    then ()
    else
      match Spec.ranged_cell (Style.reduction_matrix style) before after with
      | None -> ()
      | Some cell -> (
        let active =
          match components with
          | amount, 0 when amount > 0 -> Some (amount, before_size.inline)
          | 0, amount when amount > 0 -> Some (amount, after_size.inline)
          | _ -> None
        in
        match (active, cell.Spec.ranged_limit) with
        | Some (amount, weight), Some limit when cell.Spec.ranged_stage <> 0 ->
          let floor = Spec.scale_spec_units weight limit in
          push_site sites ~boundary ~weight ~capacity:(Num.i32_sub amount floor)
            ~stage:cell.Spec.ranged_stage ~discrete:cell.Spec.ranged_two_valued
        | _ -> ()))
  | _ -> ()

let append_line_end_reduction_site (paragraph : Paragraph.t) (style : Style.t) (ordinal : int)
    (boundary : int) sites : unit =
  match cluster_at paragraph ordinal with
  | None -> ()
  | Some cluster -> (
    let before = class_of_cluster paragraph style ordinal in
    match Spec.ranged_cell (Style.reduction_matrix style) before 0 with
    | None -> ()
    | Some cell -> (
      match cell.Spec.ranged_limit with
      | Some limit when cell.Spec.ranged_stage <> 0 ->
        let size = size_of paragraph cluster in
        let current = line_end_space_after paragraph style ordinal in
        let floor = Spec.scale_spec_units size.inline limit in
        push_site sites ~boundary ~weight:size.inline ~capacity:(Num.i32_sub current floor)
          ~stage:cell.Spec.ranged_stage ~discrete:cell.Spec.ranged_two_valued
      | _ -> ()))

let reduction_sites (paragraph : Paragraph.t) (style : Style.t) ~(line_start : int)
    ~(line_end : int) : reduction_site list =
  let sites = ref [] in
  for ordinal = line_start to line_end - 2 do
    if not (is_internal_jidori_boundary paragraph ordinal) then begin
      (* §3.2.2: a Western word space is elastic down to a quarter em. *)
      if is_western_word_space paragraph ordinal then begin
        let cluster = paragraph.Paragraph.text.clusters.(ordinal) in
        let minimum = Model.quarter_inline_size paragraph.Paragraph.text cluster in
        let capacity = max 0 (Num.i32_sub (cluster_body_advance paragraph ordinal) minimum) in
        push_site sites ~boundary:(ordinal - line_start)
          ~weight:(size_of paragraph cluster).inline ~capacity ~stage:1 ~discrete:false
      end;
      append_table_reduction_sites paragraph style ordinal (ordinal - line_start) sites
    end
  done;
  if line_start < line_end then
    append_line_end_reduction_site paragraph style (line_end - 1) (line_end - 1 - line_start) sites;
  List.rev !sites

let apply_reduction (boundary : int) (amount : int64) (adjustments : int array) : unit =
  if boundary >= 0 && boundary < Array.length adjustments then
    adjustments.(boundary) <- Num.i32_sub adjustments.(boundary) (Num.clamp_i32 amount)

(** Share [amount] out over a stage's sites, proportionally to the em each was
    measured from and never past the room each has, handing the rounding remainder
    to the leading or the trailing site as §3.8.3 and §3.8.4 leave open.

    Both ladders apportion the same way -- "equally with proportional character
    size" is §3.8.4's phrase and §3.8.3's method -- so both call this, one to take
    space away and one to add it. *)
let apportion (amount : int64) (weights : int array) (capacities : int64 array)
    (remainder : string) : int64 array =
  let count = Array.length weights in
  let weight_sum = Array.fold_left (fun sum weight -> sum +| i64 (max 1 weight)) 0L weights in
  let divisor = if le weight_sum 1L then 1L else weight_sum in
  let assigned =
    Array.init count (fun index ->
        min64 (Int64.div (amount *| i64 (max 1 weights.(index))) divisor) capacities.(index))
  in
  let left = ref (amount -| Array.fold_left ( +| ) 0L assigned) in
  let order =
    let ascending = List.init count (fun index -> index) in
    if String.equal remainder "trailing" then List.rev ascending else ascending
  in
  let running = ref true in
  while (not (le !left 0L)) && !running do
    let progressed = ref false in
    List.iter
      (fun index ->
        if (not (le !left 0L)) && lt assigned.(index) capacities.(index) then begin
          assigned.(index) <- assigned.(index) +| 1L;
          left := !left -| 1L;
          progressed := true
        end)
      order;
    if not !progressed then running := false
  done;
  assigned

(** Share [amount] out over one stage's sites, proportionally to the em each was
    measured from, and hand the rounding remainder to the leading or the trailing
    site as §3.8.3 leaves open. *)
let distribute_reduction (amount : int64) (sites : reduction_site list) (remainder : string)
    (adjustments : int array) : unit =
  if (not (le amount 0L)) && sites <> [] then begin
    let sites = Array.of_list sites in
    let assigned =
      apportion amount
        (Array.map (fun site -> site.site_weight) sites)
        (Array.map (fun site -> i64 site.site_capacity) sites)
        remainder
    in
    Array.iteri
      (fun index site -> apply_reduction site.site_boundary assigned.(index) adjustments)
      sites
  end

let prepare_line_reductions (paragraph : Paragraph.t) (style : Style.t) ~(line_start : int)
    ~(line_end : int) (need : int64) (adjustments : int array) : unit =
  let sites = reduction_sites paragraph style ~line_start ~line_end in
  let remainder = Style.remainder style in
  let need = ref need in
  let stage = ref 1 in
  while !stage <= 6 && not (le !need 0L) do
    (* A two-valued cell is all or nothing: it cannot give back a part of itself. *)
    let discrete =
      List.filter (fun site -> site.site_stage = !stage && site.site_discrete) sites
    in
    let discrete = if String.equal remainder "trailing" then List.rev discrete else discrete in
    List.iter
      (fun site ->
        if not (le !need 0L) then begin
          apply_reduction site.site_boundary (i64 site.site_capacity) adjustments;
          need := !need -| i64 site.site_capacity
        end)
      discrete;
    if not (le !need 0L) then begin
      let continuous =
        List.filter (fun site -> site.site_stage = !stage && not site.site_discrete) sites
      in
      let capacity =
        List.fold_left (fun sum site -> sum +| i64 site.site_capacity) 0L continuous
      in
      let take = min64 !need capacity in
      distribute_reduction take continuous remainder adjustments;
      need := !need -| take
    end;
    incr stage
  done

(* ----------------------------------------------------------------------------- *)
(* Line expansion (§3.8.4, Table 6) *)
(* ----------------------------------------------------------------------------- *)

(** A place a justified line may be opened up at.

    Reduction and expansion are not the same shape. A reduction shrinks an amount
    Table 1 stated, so it rides along with whichever of the two neighbors
    contributed that amount, and one boundary can carry two of them. Table 6 states
    one cell per class pair and names no neighbor at all (ADR 0021), so a boundary
    carries at most one expansion opportunity -- and carries one even where Table 1
    left the boundary solid, which is most of ordinary Japanese running text. *)
type expansion_site = {
  grow_boundary : int;  (** The boundary's index within the line. *)
  grow_weight : int;  (** The em the ceiling was measured from. *)
  grow_bounded : (int * int) option;
      (** How much one of the first three stages may add here, and which stage. *)
  grow_residual : bool;  (** Whether step (d) may keep adding here after that. *)
}

(** Class 26, the Western word space. Not one of {!Spec}'s named classes, because
    nothing but §3.8.4 step (a) asks for it by number. *)
let western_word_space_class = 26

let push_grow sites ~boundary ~weight ~bounded ~residual =
  if bounded <> None || residual then
    sites :=
      {
        grow_boundary = boundary;
        grow_weight = weight;
        grow_bounded = bounded;
        grow_residual = residual;
      }
      :: !sites

(** §3.2.2: a Western word space at either edge of a line disappears entirely.

    It is not merely set to zero -- there is nothing there any more -- so neither
    the space itself nor the boundary against it is a place §3.8.4 may open up.
    Reduction reads the same edge differently, and deliberately: it shrinks the
    space's own stated advance, which still exists as a number even where the line
    does not show it. *)
let is_collapsed_word_space (paragraph : Paragraph.t) (ordinal : int) ~(line_start : int)
    ~(line_end : int) : bool =
  is_western_word_space paragraph ordinal && (ordinal = line_start || ordinal + 1 = line_end)

(** A third of [value], rounded up, which {!Model.half_of} and {!Model.quarter_of}
    are the other two of. *)
let third_of (value : int) : int = (value / 3) + if value mod 3 <> 0 then 1 else 0

(** How wide the space at a boundary may become, measured from [weight].

    Every cell states its own ceiling and most of them are the whole answer. The
    exception is §3.8.4 step (b)'s quarter em between Japanese text and Latin
    script text, which the style opens to a half em, to a third of an em, or -- when
    it reads that quarter em as a fixed space adjustment does not touch -- no
    further than the quarter em it already is, which leaves nothing to add.

    {b Which boundary is step (b)'s} is a policy and not a reading. Step (b)'s own
    sentence names three Japanese classes and three Latin ones, nine coordinates in
    each direction; §3.8.4's Note, which is where the third answer comes from,
    names [漢字等（cl-19）など] and the same three Latin classes in Japanese and
    expands that to all three Japanese classes in English. The reference
    implementation answers the narrowest reading either sentence supports -- cl-19
    against cl-27 and nothing else -- and that is what this engine matches. It is
    observable and it is written down nowhere; see README.md, "Observable policies
    with no written source". *)
let expansion_ceiling (style : Style.t) ~(before : int) ~(after : int) ~(weight : int)
    ~(limit : int) : int =
  if not ((before = Spec.ideograph && after = 27) || (before = 27 && after = Spec.ideograph))
  then Spec.scale_spec_units weight limit
  else
    match Style.japanese_latin_expansion_ceiling style with
    | "third-em" -> third_of weight
    | "rigid" -> Model.quarter_of weight
    | _ -> Model.half_of weight

(** The em Table 6's amount is measured from.

    Table 6 names a class pair and no neighbor, so the em has to come from
    somewhere else. Where Table 1 stated an amount at the same coordinate, that
    amount's own referent is the em the boundary is already measured in -- and
    [expansion-needs-no-referent] holds that no expansion coordinate carries two
    terms, so the choice is never between two. Where Table 1 left the boundary
    solid there is no referent at all and the preceding character's em is taken,
    which is the one of the two the boundary is stated after. *)
let expansion_weight ~(components : int * int) ~(before_size : Model.size)
    ~(after_size : Model.size) : int =
  match components with
  | 0, amount when amount > 0 -> after_size.Model.inline
  | _ -> before_size.Model.inline

(** §E.2's two notes that withdraw the opportunity their own cell states.

    Note 4: two inseparable characters open a quarter em only when they are of
    different kinds; two of the same kind are one character and stay solid. Note
    10: a Western character keeps its postfixed abbreviation when it is being used
    as a quantity symbol or as a European numeral -- the same exception §C.2 note
    11 states for the break at the same coordinate, read the same way. *)
let expansion_is_withdrawn ~(before : int) ~(after : int) ~(before_cluster : Model.cluster)
    ~(before_scalar : int option) ~(after_scalar : int option) : bool =
  if before = Spec.inseparable && after = Spec.inseparable then
    cl_08_same_kind before_scalar after_scalar
  else if before = 27 && after = 13 then
    before_cluster.Model.role = Some Model.Quantity_symbol
    || is_european_numeral before_scalar
  else false

let append_table_expansion_site (paragraph : Paragraph.t) (style : Style.t) (ordinal : int)
    (boundary : int) sites : unit =
  match (cluster_at paragraph ordinal, cluster_at paragraph (ordinal + 1)) with
  | Some before_cluster, Some after_cluster -> (
    let before = class_of_cluster paragraph style ordinal in
    let after = class_of_cluster paragraph style (ordinal + 1) in
    let before_scalar = single_scalar paragraph before_cluster in
    let after_scalar = single_scalar paragraph after_cluster in
    if expansion_is_withdrawn ~before ~after ~before_cluster ~before_scalar ~after_scalar then ()
    else
      match Spec.ranged_cell Tables.table6 before after with
      | None -> ()
      | Some cell -> (
        let before_size = size_of paragraph before_cluster in
        let after_size = size_of paragraph after_cluster in
        let components =
          Spec.table_one_space_components ~before ~after ~before_size ~after_size
            ~before_solid:(is_solid paragraph before_cluster)
            ~after_solid:(is_solid paragraph after_cluster)
        in
        let weight = expansion_weight ~components ~before_size ~after_size in
        if cell.Spec.ranged_residual then
          push_grow sites ~boundary ~weight ~bounded:None ~residual:true
        else
          match cell.Spec.ranged_limit with
          | Some limit when cell.Spec.ranged_stage <> 0 ->
            let ceiling = expansion_ceiling style ~before ~after ~weight ~limit in
            let capacity = Num.i32_sub ceiling (boundary_space_after paragraph style ordinal) in
            if capacity > 0 then
              push_grow sites ~boundary ~weight
                ~bounded:(Some (capacity, cell.Spec.ranged_stage))
                ~residual:false
          | _ -> ()))
  | _ -> ()

(** §3.8.4 step (a)'s site: the Western word space itself, which opens to a half em
    before anything else on the line opens at all.

    Table 6 is not asked what the boundary after a word space may do -- step (a)
    owns it -- with one exception. Where the table's own cl-26 row says the
    coordinate is a residual one, step (d) may keep opening the same space after
    step (a) has taken it to its half em, so the site carries both facts. *)
let append_word_space_expansion_site (paragraph : Paragraph.t) (style : Style.t) (ordinal : int)
    (boundary : int) sites : unit =
  let cluster = paragraph.Paragraph.text.clusters.(ordinal) in
  let weight = (size_of paragraph cluster).inline in
  let capacity =
    Num.i32_sub (Model.half_of weight) (cluster_body_advance paragraph ordinal)
  in
  let residual =
    match
      Spec.ranged_cell Tables.table6 western_word_space_class
        (class_of_cluster paragraph style (ordinal + 1))
    with
    | Some cell -> cell.Spec.ranged_residual
    | None -> false
  in
  push_grow sites ~boundary ~weight
    ~bounded:(if capacity > 0 then Some (capacity, 1) else None)
    ~residual

(** Every place this line may be opened up at, in line order. One boundary, one
    site.

    A boundary inside a tate-chu-yoko run is not one of them: §E.2 note 12 gives the
    [(cl-30, cl-30)] cell only to two characters of {i different} runs, and the
    inside of a run is set solid (§3.2.5) and stays that way however short the line
    is. That is checked here rather than in {!append_table_expansion_site}, because
    §3.2.2's word space would otherwise open inside a run without Table 6 being
    asked at all. *)
let expansion_sites (paragraph : Paragraph.t) (style : Style.t) ~(line_start : int)
    ~(line_end : int) : expansion_site list =
  let sites = ref [] in
  for ordinal = line_start to line_end - 2 do
    if
      (not (is_internal_jidori_boundary paragraph ordinal))
      && (not (is_internal_tate_chu_yoko_boundary paragraph ordinal))
      && (not (is_collapsed_word_space paragraph ordinal ~line_start ~line_end))
      && not (is_collapsed_word_space paragraph (ordinal + 1) ~line_start ~line_end)
    then
      if is_western_word_space paragraph ordinal then
        append_word_space_expansion_site paragraph style ordinal (ordinal - line_start) sites
      else append_table_expansion_site paragraph style ordinal (ordinal - line_start) sites
  done;
  List.rev !sites

let apply_expansion (boundary : int) (amount : int64) (adjustments : int array) : unit =
  if boundary >= 0 && boundary < Array.length adjustments then
    adjustments.(boundary) <- Num.i32_add adjustments.(boundary) (Num.clamp_i32 amount)

(** Share [amount] out over these sites, each taking at most [capacity index].

    The first three stages pass their own ceilings; step (d) passes [amount], which
    is a bound no site can reach and so is none at all. *)
let distribute_expansion (amount : int64) (sites : expansion_site list)
    (capacity : expansion_site -> int64) (remainder : string) (adjustments : int array) : unit =
  if (not (le amount 0L)) && sites <> [] then begin
    let sites = Array.of_list sites in
    let assigned =
      apportion amount
        (Array.map (fun site -> site.grow_weight) sites)
        (Array.map capacity sites)
        remainder
    in
    Array.iteri
      (fun index site -> apply_expansion site.grow_boundary assigned.(index) adjustments)
      sites
  end

let bounded_at (stage : int) (site : expansion_site) : int option =
  match site.grow_bounded with
  | Some (capacity, own) when own = stage -> Some capacity
  | _ -> None

(** §3.8.4's ladder: the Western word spaces, then the Japanese-Latin quarter ems,
    then everything Table 6 leaves open to a quarter em -- each stage taking only as
    much as its own ceilings allow.

    Then step (d), if the line is still short. §E.1 states it as adding space "to
    equalize the spacing of 1st, 2nd, 3rd and 4th steps", so a boundary that already
    sits at a ceiling opens past it rather than the line staying short: the third
    stage's own quarter em is where the residual is re-leveled when nothing else on
    the line is open.

    {b Which sites step (d) re-levels} is a policy and not a reading. §E.1 says the
    first four stages and the reference implementation includes the second, the
    third and the residual cells -- but a first-stage Western word space only when
    Table 6's own cl-26 row makes that same boundary residual, never on the strength
    of step (a) alone. It is observable and it is written down nowhere; see
    README.md, "Observable policies with no written source". *)
let prepare_line_expansions (paragraph : Paragraph.t) (style : Style.t) ~(line_start : int)
    ~(line_end : int) (need : int64) (adjustments : int array) : unit =
  let sites = expansion_sites paragraph style ~line_start ~line_end in
  let remainder = Style.remainder style in
  let need = ref need in
  for stage = 1 to 3 do
    if not (le !need 0L) then begin
      let current = List.filter (fun site -> bounded_at stage site <> None) sites in
      let capacity site = i64 (Option.value ~default:0 (bounded_at stage site)) in
      let available = List.fold_left (fun sum site -> sum +| capacity site) 0L current in
      let take = min64 !need available in
      distribute_expansion take current capacity remainder adjustments;
      need := !need -| take
    end
  done;
  if not (le !need 0L) then begin
    let union =
      List.filter
        (fun site ->
          site.grow_residual
          || bounded_at 2 site <> None
          || bounded_at 3 site <> None)
        sites
    in
    distribute_expansion !need union (fun _ -> !need) remainder adjustments
  end

(** §3.8.2's hanging punctuation: a full stop or a comma at the end of an overfull
    line may sit outside the measure rather than force a wrap. *)
let hanging_amount (paragraph : Paragraph.t) (style : Style.t) ~(line_end : int)
    (occupied : int64) (available : int64) : int64 =
  if Style.hanging_punctuation style <> "hanging" || le occupied available then 0L
  else if line_end < 1 then 0L
  else
    let ordinal = line_end - 1 in
    let klass = class_of_cluster paragraph style ordinal in
    if klass <> Spec.full_stop && klass <> Spec.comma then 0L
    else
      min64 (occupied -| available) (i64 (max 0 (cluster_body_advance paragraph ordinal)))

(** How wide the line would be once every reduction the style permits is taken.

    The search asks this rather than the raw measure, so a line that only overruns
    because nothing has been reduced yet is not rejected for a fault the next step
    fixes. *)
let width_after_available_reduction (paragraph : Paragraph.t) (style : Style.t) ~(start : int)
    ~(finish : int) (width : int64) (available : int64) : int64 =
  let need = width -| available in
  if le need 0L then width
  else
    let line_start = Paragraph.cluster_index_at_or_after paragraph start in
    let line_end = Paragraph.cluster_index_at_or_after paragraph finish in
    let capacity =
      List.fold_left
        (fun sum site -> sum +| i64 site.site_capacity)
        0L
        (reduction_sites paragraph style ~line_start ~line_end)
    in
    let reduced = width -| min64 need capacity in
    reduced -| hanging_amount paragraph style ~line_end reduced available

(* ----------------------------------------------------------------------------- *)
(* Line geometry *)
(* ----------------------------------------------------------------------------- *)

(** §3.1.5's three patterns for an opening bracket at the head of a line.

    Pattern 1 leaves the indent alone. Pattern 2 adds a half em to every line that
    starts with one. Pattern 3 pulls the first line's bracket a half em back into
    the indent and leaves later lines alone. *)
let line_head_indent (paragraph : Paragraph.t) (style : Style.t) ~(line_start : int)
    ~(line_index : int) : int =
  let ordinary = if line_index = 0 then paragraph.Paragraph.first_line_indent else 0 in
  match cluster_at paragraph line_start with
  | None -> ordinary
  | Some cluster ->
    if class_of_cluster paragraph style line_start <> Spec.opening_bracket then ordinary
    else
      let half = Model.half_inline_size paragraph.Paragraph.text cluster in
      (match (line_index = 0, Style.line_head_opening_bracket style) with
      | _, "pattern-2" -> Num.i32_add ordinary half
      | true, "pattern-3" -> Num.i32_sub ordinary half
      | _ -> ordinary)

let is_tab (paragraph : Paragraph.t) (cluster : Model.cluster) : bool =
  String.equal (piece_of paragraph cluster) "\t"

(** Where a tab's segment must start for the stop to align it. *)
let tab_target (paragraph : Paragraph.t) (style : Style.t) (stop : Paragraph.tab_stop)
    ~(start : int) ~(finish : int) ~(line_start : int) ~(line_end : int) (width : int64) : int64 =
  let position = i64 stop.Paragraph.position in
  match stop.Paragraph.tab_alignment with
  | Paragraph.Tab_start -> position
  | Paragraph.Tab_center -> position -| Int64.div width 2L
  | Paragraph.Tab_end -> position -| width
  | Paragraph.Tab_character scalar ->
    let before = ref 0L in
    (try
       for ordinal = start to finish - 1 do
         let cluster = paragraph.Paragraph.text.clusters.(ordinal) in
         let piece = piece_of paragraph cluster in
         if String.equal piece "\t" || List.mem scalar (Utf8.scalars piece) then raise Exit;
         before :=
           !before +| i64 (cluster_advance_on_line paragraph style ordinal ~line_start ~line_end)
       done
     with Exit -> ());
    position -| !before

let segment_width (paragraph : Paragraph.t) (style : Style.t) ~(start : int) ~(finish : int)
    ~(line_start : int) ~(line_end : int) : int64 =
  let total = ref 0L in
  (try
     for ordinal = start to finish - 1 do
       if is_tab paragraph paragraph.Paragraph.text.clusters.(ordinal) then raise Exit;
       total :=
         !total +| i64 (cluster_advance_on_line paragraph style ordinal ~line_start ~line_end)
     done
   with Exit -> ());
  !total

(** The first tab stop past [cursor], searched from [index] onward. *)
let find_tab_stop (paragraph : Paragraph.t) (index : int) (cursor : int64) :
    Paragraph.tab_stop option =
  let rec search position =
    if position >= Array.length paragraph.Paragraph.tab_stops then None
    else
      let stop = paragraph.Paragraph.tab_stops.(position) in
      if lt cursor (i64 stop.Paragraph.position) then Some stop else search (position + 1)
  in
  search index

(** How wide the clusters between two offsets set, before adjustment.

    A tab that finds no stop past the cursor makes the line unmeasurable rather
    than wrong: the width is [i64::MAX], which the cost function reads as a line
    that cannot be chosen at all. *)
let measure_line (paragraph : Paragraph.t) (style : Style.t) ~(start : int) ~(finish : int)
    ~(line_index : int) : int64 =
  let line_start = Paragraph.cluster_index_at_or_after paragraph start in
  let line_end = Paragraph.cluster_index_at_or_after paragraph finish in
  let cursor = ref (i64 (line_head_indent paragraph style ~line_start ~line_index)) in
  let tab_index = ref 0 in
  let exhausted = ref false in
  (try
     for ordinal = line_start to line_end - 1 do
       let cluster = paragraph.Paragraph.text.clusters.(ordinal) in
       if is_tab paragraph cluster then begin
         let width =
           segment_width paragraph style ~start:(ordinal + 1) ~finish:line_end ~line_start
             ~line_end
         in
         match find_tab_stop paragraph !tab_index !cursor with
         | Some stop ->
           tab_index := !tab_index + 1;
           let target =
             tab_target paragraph style stop ~start:(ordinal + 1) ~finish:line_end ~line_start
               ~line_end width
           in
           if lt !cursor target then cursor := target
         | None ->
           exhausted := true;
           raise Exit
       end
       else
         cursor :=
           !cursor +| i64 (cluster_advance_on_line paragraph style ordinal ~line_start ~line_end)
     done
   with Exit -> ());
  if !exhausted then Num.i64_max else !cursor

(** §3.2's orientation of one cluster.

    Horizontal composition sets everything upright, and a cluster's own writing mode
    is the paragraph's. Vertical composition has three answers. A tate-chu-yoko
    member is set horizontally (§3.2.5), so it carries the {i other} writing mode and
    the transform that says the line turned it. A proportional cluster is a Western
    character being set as Western text and is rotated a quarter turn clockwise
    (§3.2.6). Everything else -- which includes a Western character in a full-em or a
    half-em frame, because §3.2.4 reads a fixed-width Western character as
    quasi-Japanese -- stands upright.

    The run is the same one {!class_of_cluster} asks about, so a cluster the
    construct covers only part of is not a member here either: one rule decides
    whether a cluster is inside a run, and the class, the orientation, the spacing
    and the geometry all follow it. *)
let local_orientation (paragraph : Paragraph.t) (ordinal : int) (frame : Model.frame) :
    Model.writing_mode * Layout.transform =
  if paragraph.Paragraph.writing_mode = Horizontal_tb then (Horizontal_tb, Layout.Identity)
  else if tate_chu_yoko_range paragraph ordinal <> None then
    (Horizontal_tb, Layout.Tate_chu_yoko)
  else if frame = Proportional then (Vertical_rl, Layout.Rotate_clockwise)
  else (Vertical_rl, Layout.Identity)

(* ----------------------------------------------------------------------------- *)
(* The search *)
(* ----------------------------------------------------------------------------- *)

(** How bad a line with this much measure left over is.

    Every number here is this project's, not JLReq's. A line that overruns costs a
    thousand times a line that comes up short by the same amount, plus a surcharge
    that puts any overrun above any shortfall. The last line of a paragraph is
    meant to be short and is charged a hundredth. A style that prefers even texture
    doubles an ordinary line's cost, which changes nothing on its own and changes
    which of two candidate break sets wins once a discretionary break or a widow is
    also in play. *)
let line_badness (delta : int64) (is_last : bool) (preference : string) : int64 =
  let magnitude = min64 (Num.sabs delta) 1_000_000L in
  let square = magnitude *| magnitude in
  if lt delta 0L then (square *| 1_000L) +| 10_000_000L
  else if is_last then Int64.div square 100L
  else if String.equal preference "even-texture" then square *| 2L
  else square

let widow_penalty (paragraph : Paragraph.t) ~(start : int) ~(finish : int) : int64 =
  match paragraph.Paragraph.widow with
  | Paragraph.No_widow -> 0L
  | Paragraph.Minimum_clusters minimum ->
    let count =
      Num.usub
        (Paragraph.cluster_index_at_or_after paragraph finish)
        (Paragraph.cluster_index_at_or_after paragraph start)
    in
    if count < minimum then 1_000_000_000L else 0L

(** §3.7.2: an inline cutting note is meant to split into two even halves, so a
    break inside one costs a million per cluster of imbalance. *)
let warichu_break_penalty (paragraph : Paragraph.t) (offset : int) : int64 =
  Array.fold_left
    (fun total (construct : Construct.t) ->
      match construct.Construct.kind with
      | Construct.Warichu ->
        let first, last = construct.Construct.range in
        if first < offset && offset < last then
          let start = Paragraph.cluster_index_at_or_after paragraph first in
          let split = Paragraph.cluster_index_at_or_after paragraph offset in
          let finish = Paragraph.cluster_index_at_or_after paragraph last in
          let before = Num.usub split start and after = Num.usub finish split in
          total +| (i64 (abs (before - after)) *| 1_000_000L)
        else total
      | _ -> total)
    0L paragraph.Paragraph.constructs

(** §3.7.4: a formula set on its own breaks at an equals sign for preference, at an
    operator reluctantly, and anywhere else only under duress. *)
let formula_break_penalty (paragraph : Paragraph.t) (offset : int) : int64 =
  let source = paragraph.Paragraph.text.source in
  let length = String.length source in
  let independent =
    Array.exists
      (fun (construct : Construct.t) ->
        match construct.Construct.kind with
        | Construct.Formula ->
          let first, last = construct.Construct.range in
          first = 0 && last = length && first < offset && offset < last
        | _ -> false)
      paragraph.Paragraph.constructs
  in
  if not independent then 0L
  else
    match (if offset < length then Some (fst (Utf8.decode source offset)) else None) with
    | Some scalar when Construct.is_math_symbol scalar -> 0L
    | Some scalar when Construct.is_math_operator scalar -> 100_000_000L
    | _ -> 200_000_000L

(* ----------------------------------------------------------------------------- *)
(* Breakability (§C.2, §C.3, Table 2) *)
(* ----------------------------------------------------------------------------- *)

(** §C.2 note 5's inseparable pairs: two of the same mark are one character, and
    the two halves of a vertical kana repeat mark belong together. Two {i
    different} inseparable characters do separate. *)
let inseparable_member_pair (before : int option) (after : int option) : bool =
  match (before, after) with
  | Some left, Some right ->
    (left = em_dash && right = em_dash)
    || (left = horizontal_ellipsis && right = horizontal_ellipsis)
    || (left = two_dot_leader && right = two_dot_leader)
    || ((left = kana_repeat_upper || left = kana_repeat_voiced_upper)
       && right = kana_repeat_lower)
  | _ -> false

(** §C.3's relaxation by reclassification: at every level but the very strict one a
    prolonged sound mark and a small kana are treated as the script they belong to,
    and an iteration mark the style permits at a line head is an ordinary
    ideographic character. *)
let reclassified_break_class (style : Style.t) (klass : int) (scalar : int option) : int =
  if
    scalar = Some iteration_mark
    && Style.iteration_mark_at_line_head style <> "prohibited"
    && Style.kinsoku_level style <> "very-strict"
  then Spec.ideograph
  else if
    Style.relaxation_mechanism style = "reclassify" && Style.kinsoku_level style <> "very-strict"
  then
    if klass = 10 then 16
    else if klass = 11 then
      match scalar with Some scalar when Spec.is_hiragana scalar -> 15 | _ -> 16
    else klass
  else klass

(** §C.3's relaxation by matrix: the four conventions differ in which classes they
    let a line break beside even where Table 2 prohibits it. *)
let c_3_relaxes_boundary (style : Style.t) ~(before : int) ~(after : int)
    ~(before_scalar : int option) ~(after_scalar : int option) : bool =
  let either_class classes = List.mem before classes || List.mem after classes in
  let either_scalar scalar = before_scalar = Some scalar || after_scalar = Some scalar in
  let iteration_relaxed =
    Style.iteration_mark_at_line_head style <> "prohibited" && either_scalar iteration_mark
  in
  let matrix_kana = Style.relaxation_mechanism style = "matrix" && either_class [ 10; 11 ] in
  match Style.kinsoku_level style with
  | "very-loose" ->
    either_class [ 3; 4; 5; 9; 12; 13 ]
    || matrix_kana
    || cl_08_same_kind before_scalar after_scalar
  | "loose" ->
    either_class [ 3 ]
    || either_scalar katakana_middle_dot
    || (before_scalar = Some horizontal_ellipsis && after_scalar = Some horizontal_ellipsis)
    || (before_scalar = Some two_dot_leader && after_scalar = Some two_dot_leader)
    || iteration_relaxed || matrix_kana
    || either_scalar percent_sign
    || either_scalar fullwidth_percent_sign
  | "strict" -> iteration_relaxed || matrix_kana
  | _ -> false

(** Whether a line may end at [offset].

    §C.3 states four prohibitions that hold at every convention level: nothing
    separates an opening bracket from what follows, and nothing separates a closing
    bracket, a full stop or a comma from what precedes. They are checked before the
    level's own relaxations, which is why the newspaper convention still refuses to
    strand a closing bracket.

    §C.2 note 13's prohibition is not among the ones asked here. Table 2 states the
    [(cl-30, cl-30)] coordinate blank and leaves the note to say which of the two
    readings of blank is meant, and the note's answer -- that two characters of the
    same tate-chu-yoko run have no opportunity between them -- is settled before the
    search runs at all: {!Paragraph.check_indivisible_constructs} refuses the
    request. A candidate that reached here inside a run would be one the paragraph
    could not have been built with. *)
let break_is_legal (paragraph : Paragraph.t) (style : Style.t) (offset : int) : bool =
  let text = paragraph.Paragraph.text in
  if offset = String.length text.source then true
  else
    let after_ordinal = Paragraph.cluster_index_at_or_after paragraph offset in
    if after_ordinal = 0 then true
    else if after_ordinal >= Array.length text.clusters then true
    else if is_tab paragraph text.clusters.(after_ordinal) then true
    else begin
      let before_ordinal = after_ordinal - 1 in
      let raw_before = class_of_cluster paragraph style before_ordinal in
      let raw_after = class_of_cluster paragraph style after_ordinal in
      let before_scalar = single_scalar paragraph text.clusters.(before_ordinal) in
      let after_scalar = single_scalar paragraph text.clusters.(after_ordinal) in
      if
        raw_before = Spec.opening_bracket
        || raw_after = Spec.closing_bracket
        || raw_after = Spec.full_stop
        || raw_after = Spec.comma
      then false
      else if
        c_3_relaxes_boundary style ~before:raw_before ~after:raw_after ~before_scalar ~after_scalar
      then true
      else begin
        let before = reclassified_break_class style raw_before before_scalar in
        let after = reclassified_break_class style raw_after after_scalar in
        match Spec.table_two_cell before after with
        | None -> true
        | Some cell ->
          if cell.Spec.break_prohibited then false
          else if before = Spec.inseparable && after = Spec.inseparable then
            not (inseparable_member_pair before_scalar after_scalar)
          else if before = 24 && after = 27 then
            (* §C.2 note 10 leaves this one to the style. *)
            Style.grouped_numeral_before_western style = "breakable"
          else if before = 27 && after = 13 then
            (* §C.2 note 11: a Western character used as a quantity symbol, or as a
               European numeral, keeps its postfixed abbreviation. *)
            text.clusters.(before_ordinal).role <> Some Quantity_symbol
            && not (is_european_numeral before_scalar)
          else cell.Spec.break_levels land Style.kinsoku_level_bit style = 0
      end
    end

(* ----------------------------------------------------------------------------- *)
(* Composition *)
(* ----------------------------------------------------------------------------- *)

type candidate = {
  candidate_offset : int;
  candidate_mandatory : bool;
  candidate_discretionary : bool;
}

type node = {
  cost : int64;
  previous : int;
  line_count : int;
}

let prepare_candidates (paragraph : Paragraph.t) : candidate array =
  let head =
    { candidate_offset = 0; candidate_mandatory = true; candidate_discretionary = false }
  in
  let rest =
    List.filter_map
      (fun (opportunity : Paragraph.break_opportunity) ->
        if is_internal_furawake_offset paragraph opportunity.Paragraph.offset then None
        else
          Some
            {
              candidate_offset = opportunity.Paragraph.offset;
              candidate_mandatory = Paragraph.is_mandatory opportunity;
              candidate_discretionary = Paragraph.is_discretionary opportunity;
            })
      (Array.to_list paragraph.Paragraph.breaks)
  in
  Array.of_list (head :: rest)

let search (paragraph : Paragraph.t) (style : Style.t) (candidates : candidate array) : node array
    =
  let count = Array.length candidates in
  let nodes = Array.make count { cost = Num.infinite_cost; previous = 0; line_count = 0 } in
  nodes.(0) <- { cost = 0L; previous = 0; line_count = 0 };
  let preference = Style.adjustment_preference style in
  let available = i64 paragraph.Paragraph.line_extent in
  for finish = 1 to count - 1 do
    let candidate = candidates.(finish) in
    if candidate.candidate_mandatory || break_is_legal paragraph style candidate.candidate_offset
    then
      for start = 0 to finish - 1 do
        let blocked = ref (Int64.equal nodes.(start).cost Num.infinite_cost) in
        for between = start + 1 to finish - 1 do
          if candidates.(between).candidate_mandatory then blocked := true
        done;
        if not !blocked then begin
          let line_index = nodes.(start).line_count in
          let start_offset = candidates.(start).candidate_offset in
          let measured =
            measure_line paragraph style ~start:start_offset
              ~finish:candidate.candidate_offset ~line_index
          in
          let width =
            width_after_available_reduction paragraph style ~start:start_offset
              ~finish:candidate.candidate_offset measured available
          in
          let delta = available -| width in
          let is_last = finish + 1 = count in
          let cost = ref (line_badness delta is_last preference) in
          if candidate.candidate_discretionary then cost := !cost +| 100_000L;
          cost := !cost +| warichu_break_penalty paragraph candidate.candidate_offset;
          cost := !cost +| formula_break_penalty paragraph candidate.candidate_offset;
          if is_last then
            cost :=
              !cost
              +| widow_penalty paragraph ~start:start_offset ~finish:candidate.candidate_offset;
          cost := !cost +| nodes.(start).cost;
          if lt !cost nodes.(finish).cost then
            nodes.(finish) <- { cost = !cost; previous = start; line_count = line_index + 1 }
        end
      done
  done;
  let last = count - 1 in
  if Int64.equal nodes.(last).cost Num.infinite_cost then
    nodes.(last) <- { cost = 0L; previous = 0; line_count = 1 };
  nodes

let backtrack (nodes : node array) : int array =
  let rec walk cursor chosen =
    if cursor = 0 then cursor :: chosen
    else
      let previous = nodes.(cursor).previous in
      if previous = cursor then cursor :: chosen else walk previous (cursor :: chosen)
  in
  Array.of_list (walk (Array.length nodes - 1) [])

(** Replace each tab's advance with the distance to the stop that aligns what
    follows it. A tab with no stop left is one em wide. *)
let apply_tabs (paragraph : Paragraph.t) (style : Style.t) ~(line_start : int) ~(line_end : int)
    ~(line_index : int) (advances : int array) : unit =
  let text = paragraph.Paragraph.text in
  let cursor = ref (i64 (line_head_indent paragraph style ~line_start ~line_index)) in
  let tab_index = ref 0 in
  for local = 0 to Array.length advances - 1 do
    let ordinal = line_start + local in
    if is_tab paragraph text.clusters.(ordinal) then begin
      let width = ref 0L in
      (try
         for following = local + 1 to Array.length advances - 1 do
           if is_tab paragraph text.clusters.(line_start + following) then raise Exit;
           width := !width +| i64 advances.(following)
         done
       with Exit -> ());
      match find_tab_stop paragraph !tab_index !cursor with
      | Some stop ->
        tab_index := !tab_index + 1;
        let target =
          tab_target paragraph style stop ~start:(ordinal + 1) ~finish:line_end ~line_start
            ~line_end !width
        in
        let distance = target -| !cursor in
        advances.(local) <- Num.clamp_i32 (if lt distance 0L then 0L else distance)
      | None -> advances.(local) <- text.size.inline
    end;
    cursor := !cursor +| i64 advances.(local)
  done

let place_line (paragraph : Paragraph.t) (style : Style.t) ~(line_start : int) ~(line_end : int)
    ~(line_index : int) ~(block_origin : int) ~(is_last : bool) : Layout.line =
  let text = paragraph.Paragraph.text in
  let count = Num.usub line_end line_start in
  let advances =
    Array.init count (fun local ->
        cluster_advance_on_line paragraph style (line_start + local) ~line_start ~line_end)
  in
  apply_tabs paragraph style ~line_start ~line_end ~line_index advances;
  let indent = line_head_indent paragraph style ~line_start ~line_index in
  let content_width =
    Array.fold_left (fun sum advance -> sum +| i64 advance) (i64 indent) advances
  in
  let remaining = i64 paragraph.Paragraph.line_extent -| content_width in
  let non_negative = if lt remaining 0L then 0L else remaining in
  let alignment_offset =
    match paragraph.Paragraph.alignment with
    | Paragraph.Start | Paragraph.Justify -> 0L
    | Paragraph.Center -> Int64.div non_negative 2L
    | Paragraph.End -> non_negative
  in
  (* Only a justified line takes up the measure it did not fill, and only one that
     is not the paragraph's last -- §3.5.3 sets a last line flush rather than
     adjusting it. Every other alignment sets a short line flush too and shifts the
     whole line instead, which `alignment_offset` above has already done. *)
  let justify =
    paragraph.Paragraph.alignment = Paragraph.Justify
    && (not is_last) && lt 0L remaining && count > 1
  in
  let adjustments = Array.make (max count 1) 0 in
  if count > 0 && lt remaining 0L then
    prepare_line_reductions paragraph style ~line_start ~line_end (Num.sabs remaining) adjustments
  else if justify then
    prepare_line_expansions paragraph style ~line_start ~line_end remaining adjustments;
  let placed = ref [] in
  let cursor = ref (i64 indent +| alignment_offset) in
  let block_extent = ref text.size.block in
  for local = 0 to count - 1 do
    let ordinal = line_start + local in
    let cluster = text.clusters.(ordinal) in
    let size = size_of paragraph cluster in
    let frame = frame_of paragraph cluster in
    (* A tate-chu-yoko run is placed across the line rather than along it. Every
       member sits at the cursor, because the run occupies one position on the line
       and `advances` charges the whole of it to the run's last member; what tells
       the members apart is the block coordinate §3.2.5's centering gives each. The
       advance reported is the member's own, which is what it contributes to the
       horizontal string, and not the step the line took.

       A member's two ems change places with it. Its block em is how tall it is in
       the string it is set in, which runs along the vertical line, so it is not what
       the line has to make room for across itself: that is the run's width, which
       may be more than any one em on the line -- a five-digit run in a one-em
       measure is wider than the line's own characters -- or less. *)
    let block, advance =
      match tate_chu_yoko_range paragraph ordinal with
      | Some (first, last) ->
        if ordinal = first then begin
          let width = tate_chu_yoko_run_width paragraph ~first ~last in
          if width > !block_extent then block_extent := width
        end;
        ( Num.i32_add block_origin (tate_chu_yoko_member_offset paragraph ordinal ~first ~last),
          cluster.advance )
      | None ->
        if size.block > !block_extent then block_extent := size.block;
        (block_origin, advances.(local))
    in
    let writing_mode, transform = local_orientation paragraph ordinal frame in
    placed :=
      {
        Layout.origin = Layout.From_cluster ordinal;
        Layout.range = (cluster.first, cluster.last);
        Layout.inline = Num.clamp_i32 !cursor;
        Layout.block;
        Layout.advance;
        Layout.size = size;
        Layout.frame = frame;
        Layout.writing_mode = writing_mode;
        Layout.transform = transform;
      }
      :: !placed;
    cursor := !cursor +| i64 advances.(local);
    cursor := !cursor +| i64 adjustments.(local)
  done;
  let range =
    if count = 0 then (0, 0)
    else (text.clusters.(line_start).first, text.clusters.(line_end - 1).last)
  in
  let occupied = !cursor -| alignment_offset in
  let hanging =
    hanging_amount paragraph style ~line_end occupied (i64 paragraph.Paragraph.line_extent)
  in
  {
    Layout.line_range = range;
    Layout.inline_origin = Num.clamp_i32 alignment_offset;
    Layout.block_origin = block_origin;
    Layout.inline_extent = Num.clamp_i32 (occupied -| hanging);
    Layout.block_extent = !block_extent;
    Layout.clusters = List.rev !placed;
    Layout.attachments = [];
  }

(** Compose one validated paragraph. *)
let compose (paragraph : Paragraph.t) (style : Style.t) : Layout.t =
  if Array.length paragraph.Paragraph.text.clusters = 0 then Layout.empty
  else begin
    let candidates = prepare_candidates paragraph in
    let nodes = search paragraph style candidates in
    let chosen = backtrack nodes in
    let lines = ref [] and diagnostics = ref [] in
    let block_cursor = ref 0L in
    for line_index = 0 to Array.length chosen - 2 do
      let start_offset = candidates.(chosen.(line_index)).candidate_offset in
      let end_offset = candidates.(chosen.(line_index + 1)).candidate_offset in
      let line_start = Paragraph.cluster_index_at_or_after paragraph start_offset in
      let line_end = Paragraph.cluster_index_at_or_after paragraph end_offset in
      let is_last = line_index + 2 = Array.length chosen in
      let block_origin =
        match paragraph.Paragraph.writing_mode with
        | Horizontal_tb -> Num.clamp_i32 !block_cursor
        | Vertical_rl -> Num.clamp_i32 (Num.sneg !block_cursor)
      in
      let line =
        place_line paragraph style ~line_start ~line_end ~line_index ~block_origin ~is_last
      in
      if lt (i64 paragraph.Paragraph.line_extent) (i64 line.Layout.inline_extent) then
        diagnostics := Layout.overfull line.Layout.line_range :: !diagnostics;
      block_cursor := !block_cursor +| i64 (max 1 line.Layout.block_extent);
      lines := line :: !lines
    done;
    let ordered_lines = List.rev !lines in
    let ordered_diagnostics = List.rev !diagnostics in
    let ordered_diagnostics =
      match (paragraph.Paragraph.widow, !lines) with
      | Paragraph.Minimum_clusters minimum, last :: _
        when List.length last.Layout.clusters < minimum ->
        ordered_diagnostics @ [ Layout.widow last.Layout.line_range ]
      | _ -> ordered_diagnostics
    in
    { Layout.lines = ordered_lines; Layout.diagnostics = ordered_diagnostics }
  end
