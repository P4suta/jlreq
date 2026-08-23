(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** What composition produces.

    A [Layout] is lines and diagnostics, and both are geometry: every number here
    is a bounded [i32] in the caller's unit, and a renderer needs nothing else to
    draw the paragraph.

    Two conventions are worth naming.

    {b Inline and block are logical.} In horizontal writing inline runs left and
    block runs down; in vertical writing inline runs down and block runs right to
    left, which is why a vertical line's [block_origin] is negative. Nothing in
    this module knows which is which -- the pipeline decides, and the placement
    carries its own [writing_mode] so a rotated run can be drawn without the caller
    reconstructing the context.

    {b The diagnostic vocabulary is two codes.} [layout.overfull] is a line wider
    than the measure after every adjustment the style permits, and [layout.widow]
    is a last line with fewer placements than the caller asked for. Anything a
    caller got wrong is refused before composition and never appears here. *)

type placement_origin =
  | From_cluster of int  (** The ordinal of a cluster of the paragraph's text. *)
  | From_construct of int  (** The ordinal of a construct. *)

type transform =
  | Identity
  | Rotate_clockwise
  | Tate_chu_yoko

let transform_name = function
  | Identity -> "identity"
  | Rotate_clockwise -> "rotate-clockwise"
  | Tate_chu_yoko -> "tate-chu-yoko"

type placement = {
  origin : placement_origin;
  range : int * int;
  inline : int;
  block : int;
  advance : int;
      (** The advance the cluster contributes {i before} line adjustment: its own
          shaped advance plus the space Table 1 puts after it. The distance to the
          next placement can be smaller, because reduction is applied to the
          cursor and not to the advance -- a caller drawing a reduced line reads
          the next placement's [inline] rather than adding this. *)
  size : Model.size;
  frame : Model.frame;
  writing_mode : Model.writing_mode;
  transform : transform;
}

type attachment = {
  attachment_construct : int;
  attachment_range : int * int;
  attachment_inline : int;
  attachment_block : int;
  attachment_advance : int;
  attachment_size : Model.size;
  attachment_writing_mode : Model.writing_mode;
  attachment_transform : transform;
  attachment_symbol : int option;
      (** The repeated scalar of an emphasis run, or [None]. The protocol requires
          the field and permits it to be null; absence and null are different
          messages there. *)
}

type line = {
  line_range : int * int;
  inline_origin : int;
  block_origin : int;
  inline_extent : int;
  block_extent : int;
  clusters : placement list;
  attachments : attachment list;
}

type severity =
  | Info
  | Warning
  | Error

let severity_name = function Info -> "info" | Warning -> "warning" | Error -> "error"

type diagnostic = {
  code : string;
  severity : severity;
  diagnostic_range : (int * int) option;
  jlreq : string;  (** The section the diagnostic is about. *)
}

type t = {
  lines : line list;
  diagnostics : diagnostic list;
}

let empty : t = { lines = []; diagnostics = [] }

(** A line wider than the measure, after every adjustment the style permits. *)
let overfull (range : int * int) : diagnostic =
  { code = "layout.overfull"; severity = Warning; diagnostic_range = Some range; jlreq = "3.8.1" }

(** A last line with fewer placements than {!Paragraph} asked for. *)
let widow (range : int * int) : diagnostic =
  { code = "layout.widow"; severity = Warning; diagnostic_range = Some range; jlreq = "3.1.9" }
