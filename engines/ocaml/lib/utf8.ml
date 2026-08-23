(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** UTF-8 scalars over an OCaml [string].

    The protocol addresses text by UTF-8 byte offset -- every [range] in a request
    or a response is a pair of byte offsets into [source] -- so the engine works on
    bytes and needs to know, at any offset, whether that offset starts a scalar and
    which scalar it starts.

    Decoding is strict. An overlong encoding, a surrogate, a value above
    [U+10FFFF], a truncated sequence, and a continuation byte where a lead byte
    belongs are all refused rather than replaced: a replacement character would
    turn malformed input into a plausible-looking wrong answer, and the protocol
    says malformed input is an error. *)

exception Malformed of string
(** Raised by the strict entry points on input that is not UTF-8. The payload is
    the message the engine prints before exiting with 2. *)

let fail offset reason =
  raise (Malformed (Printf.sprintf "byte %d: %s" offset reason))

(** How many bytes the scalar led by [byte] occupies, or [None] if [byte] cannot
    lead one. *)
let lead_length (byte : int) : int option =
  if byte < 0x80 then Some 1
  else if byte < 0xC0 then None (* continuation byte *)
  else if byte < 0xE0 then Some 2
  else if byte < 0xF0 then Some 3
  else if byte < 0xF8 then Some 4
  else None

(** Whether [offset] is a scalar boundary of [text].

    The end of the string is a boundary; an offset in the middle of a multi-byte
    sequence is not. An offset past the end is not a boundary of anything. *)
let is_boundary (text : string) (offset : int) : bool =
  if offset < 0 || offset > String.length text then false
  else if offset = String.length text then true
  else
    let byte = Char.code (String.unsafe_get text offset) in
    byte < 0x80 || byte >= 0xC0

(** The scalar starting at [offset], as [(code_point, byte_length)].

    @raise Malformed if the bytes there are not a well-formed scalar.
    @raise Invalid_argument if [offset] is outside the string. *)
let decode (text : string) (offset : int) : int * int =
  let length = String.length text in
  if offset < 0 || offset >= length then
    invalid_arg "Utf8.decode: offset out of range";
  let byte index = Char.code (String.unsafe_get text index) in
  let lead = byte offset in
  let width =
    match lead_length lead with
    | Some width -> width
    | None -> fail offset "not a UTF-8 lead byte"
  in
  if offset + width > length then fail offset "truncated UTF-8 sequence";
  let continuation index =
    let value = byte index in
    if value land 0xC0 <> 0x80 then fail offset "truncated UTF-8 sequence";
    value land 0x3F
  in
  let code =
    match width with
    | 1 -> lead
    | 2 -> ((lead land 0x1F) lsl 6) lor continuation (offset + 1)
    | 3 ->
      ((lead land 0x0F) lsl 12)
      lor (continuation (offset + 1) lsl 6)
      lor continuation (offset + 2)
    | _ ->
      ((lead land 0x07) lsl 18)
      lor (continuation (offset + 1) lsl 12)
      lor (continuation (offset + 2) lsl 6)
      lor continuation (offset + 3)
  in
  let shortest =
    match width with
    | 1 -> code < 0x80
    | 2 -> code >= 0x80
    | 3 -> code >= 0x800
    | _ -> code >= 0x10000
  in
  if not shortest then fail offset "overlong UTF-8 sequence";
  if code >= 0xD800 && code <= 0xDFFF then
    fail offset "UTF-8 sequence encodes a surrogate";
  if code > 0x10FFFF then fail offset "UTF-8 sequence is above U+10FFFF";
  (code, width)

(** [fold f init text] applies [f] to each scalar in order, as
    [f accumulator ~offset ~code ~width]. *)
let fold (f : 'a -> offset:int -> code:int -> width:int -> 'a) (init : 'a)
    (text : string) : 'a =
  let length = String.length text in
  let rec step accumulator offset =
    if offset >= length then accumulator
    else
      let code, width = decode text offset in
      step (f accumulator ~offset ~code ~width) (offset + width)
  in
  step init 0

(** Every scalar of [text], in order. *)
let scalars (text : string) : int list =
  List.rev (fold (fun acc ~offset:_ ~code ~width:_ -> code :: acc) [] text)

(** How many scalars [text] holds.

    @raise Malformed if [text] is not UTF-8. *)
let length (text : string) : int =
  fold (fun count ~offset:_ ~code:_ ~width:_ -> count + 1) 0 text

(** Whether [text] is well-formed UTF-8 from end to end. *)
let is_valid (text : string) : bool =
  match length text with _ -> true | exception Malformed _ -> false

(** Append the UTF-8 encoding of [code] to [buffer].

    @raise Invalid_argument on a surrogate or a value above [U+10FFFF]. *)
let encode (buffer : Buffer.t) (code : int) : unit =
  if code < 0 || code > 0x10FFFF || (code >= 0xD800 && code <= 0xDFFF) then
    invalid_arg "Utf8.encode: not a Unicode scalar value";
  let add value = Buffer.add_char buffer (Char.chr value) in
  if code < 0x80 then add code
  else if code < 0x800 then begin
    add (0xC0 lor (code lsr 6));
    add (0x80 lor (code land 0x3F))
  end
  else if code < 0x10000 then begin
    add (0xE0 lor (code lsr 12));
    add (0x80 lor ((code lsr 6) land 0x3F));
    add (0x80 lor (code land 0x3F))
  end
  else begin
    add (0xF0 lor (code lsr 18));
    add (0x80 lor ((code lsr 12) land 0x3F));
    add (0x80 lor ((code lsr 6) land 0x3F));
    add (0x80 lor (code land 0x3F))
  end

(** One scalar as a string. *)
let of_scalar (code : int) : string =
  let buffer = Buffer.create 4 in
  encode buffer code;
  Buffer.contents buffer

(** The single scalar [text] holds, or [None] if it holds none or more than one.

    The protocol's [scalar] type -- an emphasis mark, a character tab stop's
    alignment character -- is a string of exactly one scalar, and the schema
    writes that bound in UTF-16 code units. A scalar above the basic plane is two
    of those, so the check has to be made on scalars and not on lengths. *)
let single_scalar (text : string) : int option =
  match length text with
  | 1 -> Some (fst (decode text 0))
  | _ -> None
  | exception Malformed _ -> None
