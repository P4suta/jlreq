(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** The validation boundary for shaped text.

    [docs/design/api-spine.md] puts every way a caller can be wrong on one side of
    a line and says composition never fails on the other. This module is that line
    for {!Model.shaped_text}: past it, the cluster array covers the source exactly
    once in order, every offset starts a scalar, every size is positive, and every
    advance is a number the protocol can carry.

    The rule that is easiest to miss is the last one below. A cluster is one
    positioned thing, and Appendix A keys some of its entries by two code points --
    a kana with a combining semi-voiced sound mark, a phonetic letter with a tone
    mark. A proportional cluster may hold several scalars, because a Latin ligature
    is one shaped glyph and Appendix A never keys Latin by pairs. A full-em or
    half-em cluster may not, unless the several scalars are exactly one of those
    keys: a full-em box holding two ideographs is two characters that were told
    they are one, and every class, space and break decision downstream would be
    made for a character that is not there. *)

exception Invalid of string

let fail format = Printf.ksprintf (fun message -> raise (Invalid message)) format

let check_size (what : string) (size : Model.size) : unit =
  if size.Model.inline <= 0 || size.Model.block <= 0 then
    fail "%s states a size of %d by %d, and both must be positive" what size.Model.inline
      size.Model.block;
  if not (Num.is_i32 size.Model.inline && Num.is_i32 size.Model.block) then
    fail "%s states a size outside the protocol's range" what

(** Whether a cluster is one thing the specification can classify. *)
let check_indivisible (text : Model.shaped_text) (ordinal : int) (cluster : Model.cluster) : unit =
  let piece = Model.cluster_piece text cluster in
  match Utf8.scalars piece with
  | [] | [ _ ] -> ()
  | [ first; second ]
    when Model.cluster_frame text cluster <> Model.Proportional && Spec.is_pair first second ->
    ()
  | _ :: _ :: _ ->
    if Model.cluster_frame text cluster <> Model.Proportional then
      fail
        "cluster %d holds more than one code point in a %s box, and Appendix A does not key them \
         together"
        ordinal
        (Model.frame_name (Model.cluster_frame text cluster))

(** Check a whole shaped text.

    @raise Invalid on anything a validated paragraph may not contain.
    @raise Utf8.Malformed if [source] is not UTF-8. *)
let check (text : Model.shaped_text) : unit =
  ignore (Utf8.length text.Model.source);
  check_size "the text" text.Model.size;
  let length = String.length text.Model.source in
  let cursor = ref 0 in
  Array.iteri
    (fun ordinal (cluster : Model.cluster) ->
      if cluster.Model.first <> !cursor then
        fail "cluster %d starts at byte %d, where byte %d is the next uncovered one" ordinal
          cluster.Model.first !cursor;
      if cluster.Model.last < cluster.Model.first then
        fail "cluster %d runs from byte %d back to byte %d" ordinal cluster.Model.first
          cluster.Model.last;
      if cluster.Model.last > length then
        fail "cluster %d ends at byte %d, past the %d byte source" ordinal cluster.Model.last
          length;
      if not (Utf8.is_boundary text.Model.source cluster.Model.first) then
        fail "cluster %d starts at byte %d, which is inside a UTF-8 sequence" ordinal
          cluster.Model.first;
      if not (Utf8.is_boundary text.Model.source cluster.Model.last) then
        fail "cluster %d ends at byte %d, which is inside a UTF-8 sequence" ordinal
          cluster.Model.last;
      if cluster.Model.advance < 0 then
        fail "cluster %d has an advance of %d" ordinal cluster.Model.advance;
      if not (Num.is_i32 cluster.Model.advance) then
        fail "cluster %d has an advance outside the protocol's range" ordinal;
      (match cluster.Model.size_override with
      | Some size -> check_size (Printf.sprintf "cluster %d" ordinal) size
      | None -> ());
      check_indivisible text ordinal cluster;
      cursor := cluster.Model.last)
    text.Model.clusters;
  if !cursor <> length then
    fail "the clusters cover %d of the source's %d bytes" !cursor length
