(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** The nine inline structures.

    [docs/design/api-spine.md] names them and the protocol schema spells each one's
    fields. A construct is a byte range over the paragraph's source plus whatever
    that kind of structure needs: an annotation and its runs for ruby, a repeated
    mark for emphasis dots, a column count for furawake, a cell count for jidori.

    Milestone M1 composes none of them. It parses all of them, which is not the
    same thing as ignoring them: a construct decides the class of the characters it
    covers (§3.9.2 gives ruby bases, ornamented complexes and reference marks their
    own classes) and that much is answered here from the range alone. The geometry
    -- where the annotation sits, how a warichu splits, what a jidori's cells do to
    the advances -- arrives at the milestones that own it, and the shape of this
    module is what those milestones fill in rather than replace. *)

type ruby_kind =
  | Mono
  | Group
  | Jukugo

type ruby_run = {
  run_base : int * int;  (** A byte range of the paragraph's source. *)
  run_annotation : int * int;  (** A byte range of the annotation's source. *)
}

type kind =
  | Ruby of {
      ruby_kind : ruby_kind;
      annotation : Model.shaped_text;
      runs : ruby_run list;
    }
  | Tate_chu_yoko
  | Warichu
  | Formula
  | Emphasis_dots of { mark : int  (** One Unicode scalar. *) }
  | Furawake of { columns : int; line_gap : int }
  | Jidori of { cells : int }
  | Reference_mark of { annotation : Model.shaped_text }
  | Script of { annotation : Model.shaped_text }

type t = {
  range : int * int;
  kind : kind;
}

let range_start (construct : t) : int = fst construct.range
let range_end (construct : t) : int = snd construct.range

(** Whether two half-open ranges share a byte. *)
let ranges_overlap ((left_start, left_end) : int * int) ((right_start, right_end) : int * int) :
    bool =
  left_start < right_end && right_start < left_end

(** The class §3.9.2 gives a character because of the structure it sits in, or
    [None] when no construct covering it changes its class.

    Ruby splits by kind: a jukugo-ruby base is cl-23 and every other ruby base is
    cl-22. Emphasis dots and a script complex both make their base an ornamented
    character complex (cl-21). A reference mark's characters are cl-20. The
    remaining four structures leave their characters' own classification alone. *)
let structural_class (constructs : t array) (range : int * int) : int option =
  let rec search index =
    if index >= Array.length constructs then None
    else
      let construct = constructs.(index) in
      if not (ranges_overlap construct.range range) then search (index + 1)
      else
        match construct.kind with
        | Ruby { ruby_kind = Jukugo; _ } -> Some 23
        | Ruby _ -> Some 22
        | Emphasis_dots _ | Script _ -> Some 21
        | Reference_mark _ -> Some 20
        | Tate_chu_yoko | Warichu | Formula | Furawake _ | Jidori _ -> search (index + 1)
  in
  search 0

(** The ordinal and range of the first construct of a kind covering [range]. *)
let find (constructs : t array) (range : int * int) (matches : kind -> bool) : (int * t) option =
  let rec search index =
    if index >= Array.length constructs then None
    else
      let construct = constructs.(index) in
      if ranges_overlap construct.range range && matches construct.kind then
        Some (index, construct)
      else search (index + 1)
  in
  search 0

(* ----------------------------------------------------------------------------- *)
(* Math tokens *)
(* ----------------------------------------------------------------------------- *)

(* §3.7.4's formula spacing is stated in terms of two classes rather than a list
   of characters: cl-17 is the equals-sign family and cl-18 the operators. Both
   are read straight out of Appendix A. *)

let is_math_symbol (scalar : int) : bool = Spec.single_has_class scalar Spec.math_symbol
let is_math_operator (scalar : int) : bool = Spec.single_has_class scalar Spec.math_operator
let is_math_token (scalar : int) : bool = is_math_symbol scalar || is_math_operator scalar
