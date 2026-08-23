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
      | Construct.Ruby { annotation; _ }
      | Construct.Reference_mark { annotation }
      | Construct.Script { annotation } ->
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

(** Build a paragraph, or refuse the input.

    @raise Invalid on anything a composed paragraph may not contain.
    @raise Normalize.Invalid on shaped text that is not well formed. *)
let build ~(text : Model.shaped_text) ~(line_extent : int)
    ?(breaks : break_opportunity list = []) ?(constructs : Construct.t list = [])
    ?(tab_stops : tab_stop list = []) ?(first_line_indent = 0) ?(alignment = Start)
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
  List.iteri
    (fun index stop ->
      if stop.position <= 0 then fail "a tab stop is at %d" stop.position;
      List.iteri
        (fun other_index other ->
          if other_index > index && other.position = stop.position then
            fail "two tab stops are stated at %d" stop.position)
        tab_stops)
    tab_stops;
  let constructs = Array.of_list constructs in
  check_constructs text constructs;
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
