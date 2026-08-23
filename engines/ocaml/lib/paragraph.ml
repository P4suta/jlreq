(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** A validated paragraph: shaped text, a measure, and everything the caller says
    about how it is to be set.

    {!build} is the validation boundary [docs/design/api-spine.md] describes, and
    it is the last place composition can fail. Everything past it is arithmetic.

    {b The paragraph end is a break.} JLReq's line breaking is a search over break
    opportunities, and the end of the text is the one opportunity that is always
    taken. It is stored here as a mandatory break rather than special-cased in the
    search, so the search has exactly one kind of thing to look at. A caller who
    states a break at the end of the source gets that break promoted to mandatory
    rather than duplicated. *)

exception Invalid of string

let fail format = Printf.ksprintf (fun message -> raise (Invalid message)) format

type alignment =
  | Start
  | Center
  | End
  | Justify

type break_kind =
  | Allowed
  | Mandatory
  | Discretionary

type break_opportunity = {
  offset : int;
  kind : break_kind;
}

type tab_alignment =
  | Tab_start
  | Tab_center
  | Tab_end
  | Tab_character of int  (** One Unicode scalar. *)

type tab_stop = {
  position : int;
  tab_alignment : tab_alignment;
}

type widow =
  | No_widow
  | Minimum_clusters of int

type t = {
  text : Model.shaped_text;
  line_extent : int;
  breaks : break_opportunity array;
      (** In ascending offset order, ending with the mandatory break at the end of
          the source. Offset zero is not here: the search starts there. *)
  constructs : Construct.t array;
  tab_stops : tab_stop array;
  first_line_indent : int;
  alignment : alignment;
  widow : widow;
  writing_mode : Model.writing_mode;
}

let is_mandatory (opportunity : break_opportunity) : bool = opportunity.kind = Mandatory
let is_discretionary (opportunity : break_opportunity) : bool = opportunity.kind = Discretionary

(** The ordinal of the first cluster whose range starts at or after [offset].

    Lines are cut at byte offsets and placed as cluster runs, so this is the one
    conversion between the two coordinate systems. An offset past every cluster is
    the cluster count, which makes the last line's end well defined. *)
let cluster_index_at_or_after (paragraph : t) (offset : int) : int =
  let clusters = paragraph.text.Model.clusters in
  let count = Array.length clusters in
  let rec search index =
    if index >= count then count
    else if clusters.(index).Model.first >= offset then index
    else search (index + 1)
  in
  search 0

let check_boundary (text : Model.shaped_text) (what : string) (offset : int) : unit =
  if not (Utf8.is_boundary text.Model.source offset) then
    fail "%s is at byte %d, which is not a UTF-8 boundary of the source" what offset

(** §3.3.1's association, as the shape a request has to have.

    A ruby construct is a base and a reading {i paired up}: "mono-ruby is the method
    of attaching ruby to each individual base character", "group-ruby is the method of
    attaching ruby to a group of base characters". The runs are how the caller states
    which reading belongs to which base characters, so they have to be a partition of
    both -- every base character in exactly one run, every ruby character in exactly
    one run, and both walked in source order. A run that covers nothing, one that
    starts where the previous one has not ended, and a set that leaves either side
    partly uncovered all describe an association that does not exist, and are refused
    rather than composed into a plausible wrong answer.

    Mono ruby is narrower still: §3.3.1 attaches it to {i each individual base
    character}, so a run of it covers one shaped cluster and no other number. Group
    ruby is the mirror image: it is "the method of attaching ruby to {i a group} of
    base characters", one group and one reading, so it takes one run over as many
    base characters as the caller likes and no second run. Only jukugo ruby takes
    several -- §3.3.7 is about a compound {i of} base characters, each with a reading
    of its own. *)
let check_ruby_runs (text : Model.shaped_text) (ordinal : int) ~(first : int) ~(last : int)
    (ruby_kind : Construct.ruby_kind) (annotation : Model.shaped_text)
    (runs : Construct.ruby_run list) : unit =
  if runs = [] then fail "construct %d is ruby and states no base-to-annotation run" ordinal;
  if ruby_kind = Construct.Group && List.length runs <> 1 then
    fail "construct %d is group ruby and states %d runs" ordinal (List.length runs);
  let reading = String.length annotation.Model.source in
  let base_cursor = ref first and mark_cursor = ref 0 in
  List.iter
    (fun (run : Construct.ruby_run) ->
      let base_first, base_last = run.Construct.run_base in
      let mark_first, mark_last = run.Construct.run_annotation in
      if base_first <> !base_cursor || base_last <= base_first || base_last > last then
        fail "construct %d's runs do not partition its base characters in source order" ordinal;
      if mark_first <> !mark_cursor || mark_last <= mark_first || mark_last > reading then
        fail "construct %d's runs do not partition its annotation in source order" ordinal;
      check_boundary annotation (Printf.sprintf "construct %d's annotation run" ordinal)
        mark_first;
      check_boundary annotation (Printf.sprintf "construct %d's annotation run" ordinal) mark_last;
      base_cursor := base_last;
      mark_cursor := mark_last;
      if ruby_kind = Construct.Mono then begin
        let covered = ref 0 in
        Array.iter
          (fun (cluster : Model.cluster) ->
            if cluster.Model.first >= base_first && cluster.Model.last <= base_last then
              incr covered)
          text.Model.clusters;
        if !covered <> 1 then
          fail "construct %d is mono ruby and one of its runs covers %d base clusters" ordinal
            !covered
      end)
    runs;
  if !base_cursor <> last || !mark_cursor <> reading then
    fail "construct %d's runs leave part of its base or its annotation unread" ordinal

let check_constructs (text : Model.shaped_text) (constructs : Construct.t array) : unit =
  let length = String.length text.Model.source in
  Array.iteri
    (fun ordinal (construct : Construct.t) ->
      let first, last = construct.Construct.range in
      if first > last || last > length then
        fail "construct %d covers bytes %d..%d of a %d byte source" ordinal first last length;
      check_boundary text (Printf.sprintf "construct %d's start" ordinal) first;
      check_boundary text (Printf.sprintf "construct %d's end" ordinal) last;
      (match construct.Construct.kind with
      | Construct.Furawake { columns; line_gap } ->
        if columns < 1 then fail "construct %d distributes into %d columns" ordinal columns;
        if line_gap < 0 then fail "construct %d has a line gap of %d" ordinal line_gap
      | Construct.Jidori { cells } ->
        if cells < 1 then fail "construct %d fits into %d cells" ordinal cells
      | Construct.Ruby { annotation; ruby_kind; runs } ->
        Normalize.check annotation;
        check_ruby_runs text ordinal ~first ~last ruby_kind annotation runs
      | Construct.Reference_mark { annotation } | Construct.Script { annotation } ->
        Normalize.check annotation
      | Construct.Emphasis_dots _ | Construct.Tate_chu_yoko | Construct.Warichu
      | Construct.Formula ->
        ());
      (* Two structures may nest but may not overlap partway: a range that starts
         inside one and ends outside it describes no structure. *)
      Array.iteri
        (fun other_ordinal (other : Construct.t) ->
          if other_ordinal > ordinal then begin
            let other_first, other_last = other.Construct.range in
            let crosses =
              (first < other_first && other_first < last && last < other_last)
              || (other_first < first && first < other_last && other_last < last)
            in
            if crosses then
              fail "constructs %d and %d cross rather than nest" ordinal other_ordinal
          end)
        constructs)
    constructs

(** The scalar that ends at [offset] and the one that starts there.

    A boundary is between two characters, and §3.7.4 asks about both of them. Either
    is [None] at the two ends of the source. *)
let scalars_across (source : string) (offset : int) : int option * int option =
  let length = String.length source in
  let after = if offset >= 0 && offset < length then Some (fst (Utf8.decode source offset)) else None in
  let rec back index =
    if index < 0 then None
    else if Utf8.is_boundary source index then Some (fst (Utf8.decode source index))
    else back (index - 1)
  in
  let before = if offset <= 0 || offset > length then None else back (offset - 1) in
  (before, after)

(** §C.2 note 13, as a refusal rather than as a break the search declines to take.

    A tate-chu-yoko run is one thing on the line -- a horizontal string set inside a
    vertical one (§3.2.5) -- and the note says there is no line break opportunity
    between two of its characters. A caller who states one is describing a paragraph
    that does not exist, because half a run is not something a line can end with, so
    the request is refused rather than answered with the opportunity quietly dropped.
    The run's own two edges are ordinary break opportunities, and so is the boundary
    between two runs set one after the other, which is the coordinate the note's
    second sentence is about.

    It is refused in horizontal composition too, where a tate-chu-yoko construct
    changes nothing else at all: the structure is indivisible because of what it is
    and not because of the direction the line happens to run in.

    Ruby is refused too, and at a narrower coordinate: the run rather than the
    construct. §3.3.5 and §3.3.6 say the same thing in the same words about mono ruby
    and about group ruby -- "base characters and attached ruby characters are handled
    as one object, and internal line-breaks are prohibited" -- and §C.2 note 8 says it
    of jukugo ruby's own runs while permitting the break {i between} two of them.
    A caller who states a break inside one run is describing half a base character
    group, which is not something a line can end with; a caller who states one at a
    run boundary is describing exactly what §3.3.7 and §C.2 note 8 allow, and gets it.

    §C.2 note 6 draws the same line around an ornamented character complex: "there is
    no line break opportunity between two consecutive characters belonging to the same
    ornamented character complex (cl-21)", which §3.7.1 states again in its own words
    -- "it is prohibited to break lines within an ornamented character complex (cl-21)
    sequence". A [script] construct is one such complex and a [reference-mark] is read
    the same way, so a break inside either is refused. {b Emphasis dots are not}: §3.3.9
    attaches a mark to each base character on its own, so each of them is its own
    complex and every boundary inside the run is a boundary between two of them. That
    reading is observable -- it is what decides whether §E.2 note 5's quarter em opens
    inside an emphasis run -- and JLReq states it in no sentence; see README.md,
    "Observable policies with no written source".

    §3.7.3's jidori is refused too: the construct is a length the caller specified for
    a run of text, and half of that run is not something a line can end with.

    §3.7.4's formula is refused at every boundary {i except} the ones the section
    names. "A line break in a mathematical formula is done, when possible, at an
    equals sign (cl-17) ... or at an operator (cl-18)", and the reference engine reads
    that as the whole of where a formula may break: a break with a math symbol or a
    math operator on either side of it is answered, and any other break inside a
    formula is refused. That holds for a formula set on its own and for one inside a
    line alike.

    Two structures are left divisible, and the built-in suite states breaks inside
    both. A warichu splits into two sublines (§3.4.2) and a furawake into its declared
    columns (§3.7.2), so a break inside one of those is exactly what the caller means
    by it. *)
let check_indivisible_constructs (text : Model.shaped_text) (breaks : break_opportunity list)
    (constructs : Construct.t array) : unit =
  let refuse ?(unless = fun _ -> false) ordinal (first, last) =
    List.iter
      (fun opportunity ->
        if
          first < opportunity.offset
          && opportunity.offset < last
          && not (unless opportunity.offset)
        then
          fail
            "a break is at byte %d, inside construct %d, which covers bytes %d..%d and is \
             indivisible"
            opportunity.offset ordinal first last)
      breaks
  in
  let at_a_math_token offset =
    let before, after = scalars_across text.Model.source offset in
    let token = function Some scalar -> Construct.is_math_token scalar | None -> false in
    token before || token after
  in
  Array.iteri
    (fun ordinal (construct : Construct.t) ->
      match construct.Construct.kind with
      | Construct.Tate_chu_yoko | Construct.Jidori _ | Construct.Reference_mark _
      | Construct.Script _ ->
        refuse ordinal construct.Construct.range
      | Construct.Formula -> refuse ~unless:at_a_math_token ordinal construct.Construct.range
      | Construct.Ruby { runs; _ } ->
        List.iter (fun (run : Construct.ruby_run) -> refuse ordinal run.Construct.run_base) runs
      | Construct.Warichu | Construct.Emphasis_dots _ | Construct.Furawake _ -> ())
    constructs

(** §3.7.2's columns, as a shape the request has to have.

    "When there are line break marks in the furiwake-gyou, the line is broken in the
    indicated places": the caller states where one furawake-gyou ends and the next
    begins, so a furawake of [columns] columns needs exactly [columns - 1] break
    opportunities inside itself. Fewer describes a block with a column the caller
    never divided off, more describes one with a column too many, and neither is a
    furawake the caller could have meant. *)
let check_furawake_splits (breaks : break_opportunity list) (constructs : Construct.t array) : unit
    =
  Array.iteri
    (fun ordinal (construct : Construct.t) ->
      match construct.Construct.kind with
      | Construct.Furawake { columns; _ } ->
        let first, last = construct.Construct.range in
        let stated =
          List.length
            (List.filter
               (fun opportunity -> first < opportunity.offset && opportunity.offset < last)
               breaks)
        in
        if stated <> columns - 1 then
          fail "construct %d distributes into %d columns and states %d break(s) inside itself"
            ordinal columns stated
      | _ -> ())
    constructs

(** §3.6.1's count of stops: "If there is more than one tab sign, it is necessary to
    set the same numbers of tab positions and tab types as the number of tab signs."

    That sentence counts tab signs {i in a line}, and which line a sign lands on is
    the question composition exists to answer, so the only division into lines that
    validation can see is the one the caller has already made -- the mandatory
    breaks. Every stretch between two of them is one line or more, so a stretch
    carrying more tab signs than there are stops describes a line that cannot be
    set however the search cuts it.

    {b Two readings here have no written source.} The count is taken between
    mandatory breaks rather than over the whole paragraph, and a surplus of stops
    is not an error -- a stop the line never reaches is simply never used. *)
let check_tab_stop_supply (text : Model.shaped_text) (breaks : break_opportunity list)
    (tab_stops : tab_stop list) : unit =
  let supply = List.length tab_stops in
  let boundaries =
    ref
      (List.filter_map
         (fun opportunity -> if opportunity.kind = Mandatory then Some opportunity.offset else None)
         breaks)
  in
  let signs = ref 0 in
  Array.iter
    (fun (cluster : Model.cluster) ->
      let rec reach () =
        match !boundaries with
        | offset :: rest when offset <= cluster.Model.first ->
          signs := 0;
          boundaries := rest;
          reach ()
        | _ -> ()
      in
      reach ();
      if String.equal (Model.cluster_piece text cluster) "\t" then begin
        incr signs;
        if !signs > supply then
          fail "a line holds %d tab sign(s) and the request states %d tab stop(s)" !signs supply
      end)
    text.Model.clusters

(** Build a paragraph, or refuse the input.

    The alignment a caller who states none gets is [Justify], because §3.8.1 is
    what happens to a line by default: "lines are created by separating character
    sequences at places where line breaking is not prohibited", and every line but
    a short last one is then adjusted to the measure. [Start] is a caller {i
    asking} for §3.5.3's flush setting instead, and is never the absence of an
    answer.

    @raise Invalid on anything a composed paragraph may not contain.
    @raise Normalize.Invalid on shaped text that is not well formed. *)
let build ~(text : Model.shaped_text) ~(line_extent : int)
    ?(breaks : break_opportunity list = []) ?(constructs : Construct.t list = [])
    ?(tab_stops : tab_stop list = []) ?(first_line_indent = 0) ?(alignment = Justify)
    ?(widow = No_widow) ?(writing_mode = Model.Horizontal_tb) () : t =
  Normalize.check text;
  if line_extent <= 0 then fail "the measure is %d, and a line must be wider than nothing" line_extent;
  if not (Num.is_i32 first_line_indent) then
    fail "the first line indent is outside the protocol's range";
  let length = String.length text.Model.source in
  List.iter
    (fun opportunity ->
      if opportunity.offset > length then
        fail "a break is at byte %d, past the %d byte source" opportunity.offset length;
      check_boundary text "a break" opportunity.offset)
    breaks;
  let sorted =
    List.sort (fun left right -> compare left.offset right.offset)
      (List.filter (fun opportunity -> opportunity.offset > 0) breaks)
  in
  List.iteri
    (fun index opportunity ->
      if index > 0 && (List.nth sorted (index - 1)).offset = opportunity.offset then
        fail "two breaks are stated at byte %d" opportunity.offset)
    sorted;
  (* The end of the source is always a break, and always a mandatory one. *)
  let terminal_stated = List.exists (fun opportunity -> opportunity.offset = length) sorted in
  let breaks =
    if terminal_stated then
      List.map
        (fun opportunity ->
          if opportunity.offset = length then { opportunity with kind = Mandatory }
          else opportunity)
        sorted
    else sorted @ [ { offset = length; kind = Mandatory } ]
  in
  (* A stop is a position {i in the line}, so one at or past the measure is a
     position no line reaches, and one at or before its head is not a position at
     all. Both are refused whether or not the source holds a tab sign: the stop
     list is a description of the line, and a line the caller has described wrongly
     is refused before it is set. *)
  List.iteri
    (fun index stop ->
      if stop.position <= 0 then fail "a tab stop is at %d" stop.position;
      if stop.position >= line_extent then
        fail "a tab stop is at %d, which the %d measure does not reach" stop.position line_extent;
      List.iteri
        (fun other_index other ->
          if other_index > index && other.position = stop.position then
            fail "two tab stops are stated at %d" stop.position)
        tab_stops)
    tab_stops;
  check_tab_stop_supply text breaks tab_stops;
  (* §3.6.3 walks the stops "in order", and the order of positions along the line
     is the only order a line knows: the caller's listing order is how the stops
     were written down, not where they are. Sorting here means the search and the
     placement never have to ask. *)
  let tab_stops = List.sort (fun left right -> compare left.position right.position) tab_stops in
  let constructs = Array.of_list constructs in
  check_constructs text constructs;
  check_indivisible_constructs text breaks constructs;
  check_furawake_splits breaks constructs;
  (match widow with
  | Minimum_clusters minimum when minimum < 1 ->
    fail "the widow minimum is %d clusters" minimum
  | _ -> ());
  {
    text;
    line_extent;
    breaks = Array.of_list breaks;
    constructs;
    tab_stops = Array.of_list tab_stops;
    first_line_indent;
    alignment;
    widow;
    writing_mode;
  }
