(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** The shaped input the engine composes.

    This is the vocabulary [docs/design/api-spine.md] calls the model: a [Size], a
    [Frame], a [WritingMode], a [ClusterRole], and the [ShapedText] that carries a
    source string and the clusters covering it. Shaping has already happened; the
    engine never looks at a font and never re-segments text.

    Two properties of this layer are worth stating because getting either wrong is
    silent rather than loud.

    {b Every coordinate is a UTF-8 byte offset into [source].} Not a character
    index, not a cluster index. The protocol addresses text this way and so does
    every range that comes back out.

    {b A cluster's size and frame are overrides, not values.} A cluster that states
    neither takes the paragraph's, and the difference matters: {!cluster_size}
    resolves the override against the text, and a place that reads
    [cluster.size_override] directly and defaults to something else is a bug. *)

(** How a cluster's advance relates to its em box.

    [Full_em] is one em, [Half_em] a half em, [Proportional] whatever the shaper
    measured. §3.9.2's classification reads this: the same code point is a Western
    character when it is proportional and an ideographic character when it is not. *)
type frame =
  | Full_em
  | Half_em
  | Proportional

(** The direction lines advance in. *)
type writing_mode =
  | Horizontal_tb
  | Vertical_rl

(** What an occurrence of a character is being used {i as}.

    JLReq classifies several code points by role rather than by identity -- a
    middle dot inside a unit symbol is not a middle dot for spacing purposes
    (§B.2 note 12), a full stop used as a decimal point is not a full stop. The
    caller states the role because only the caller knows it.

    [Text] and no role at all are the same thing everywhere in this engine; the
    variant exists because the protocol spells it. *)
type role =
  | Text
  | Decimal_point
  | Digit_group_separator
  | Sentence_medial
  | Sentence_terminator
  | Grouped_numeral
  | Unit_symbol
  | Quantity_symbol
  | Formula
  | Warichu_bracket

type size = {
  inline : int;  (** The em width along the line. *)
  block : int;  (** The em width across it. *)
}

type cluster = {
  first : int;  (** Byte offset of the cluster's first byte. *)
  last : int;  (** Byte offset one past its last. *)
  advance : int;  (** The shaped inline advance, before any spacing. *)
  size_override : size option;
  frame_override : frame option;
  role : role option;
}

type shaped_text = {
  source : string;
  size : size;
  frame : frame;
  clusters : cluster array;
}

let frame_name = function
  | Full_em -> "full-em"
  | Half_em -> "half-em"
  | Proportional -> "proportional"

let writing_mode_name = function
  | Horizontal_tb -> "horizontal-tb"
  | Vertical_rl -> "vertical-rl"

(** The size in force for a cluster: its own override, or the text's. *)
let cluster_size (text : shaped_text) (cluster : cluster) : size =
  match cluster.size_override with Some size -> size | None -> text.size

(** The frame in force for a cluster: its own override, or the text's. *)
let cluster_frame (text : shaped_text) (cluster : cluster) : frame =
  match cluster.frame_override with Some frame -> frame | None -> text.frame

(** The source text a cluster covers. *)
let cluster_piece (text : shaped_text) (cluster : cluster) : string =
  String.sub text.source cluster.first (cluster.last - cluster.first)

(** The one scalar a cluster covers, or [None] when it covers zero or several.

    Several rules in Appendices B and C are stated about a character rather than a
    class -- §C.2 note 5's inseparable pairs, the middle dot a unit symbol makes
    solid -- and they apply to a cluster only when that cluster {i is} one
    character. *)
let single_scalar (text : shaped_text) (cluster : cluster) : int option =
  let width = cluster.last - cluster.first in
  if width <= 0 then None
  else
    match Utf8.lead_length (Char.code text.source.[cluster.first]) with
    | Some length when length = width ->
      let scalar, _ = Utf8.decode text.source cluster.first in
      Some scalar
    | _ -> None

(** Half the inline em, rounded up. *)
let half_of (value : int) : int = (value / 2) + (value mod 2)

(** A quarter of the inline em, rounded up. *)
let quarter_of (value : int) : int = (value / 4) + if value mod 4 <> 0 then 1 else 0

(** Half a cluster's inline em, rounded up. *)
let half_inline_size (text : shaped_text) (cluster : cluster) : int =
  half_of (cluster_size text cluster).inline

(** A quarter of a cluster's inline em, rounded up. *)
let quarter_inline_size (text : shaped_text) (cluster : cluster) : int =
  quarter_of (cluster_size text cluster).inline
