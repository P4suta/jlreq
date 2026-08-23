(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** {!Jlreq.Utf8}: scalar boundaries and strict decoding.

    The protocol addresses text by byte offset, so the engine's idea of where a
    scalar starts has to be the same as the caller's. The rejection cases matter
    as much as the acceptance ones: an overlong encoding that decoded to the
    "right" scalar would let two different byte strings address the same
    character, and a range would then mean two things. *)

open Jlreq.Utf8

let run () =
  (* Widths, one per lead-byte class. *)
  Check.equal_int "ASCII is one byte" ~expected:1 ~actual:(snd (decode "A" 0));
  Check.equal_int "U+00E9 is two bytes" ~expected:2 ~actual:(snd (decode "\xc3\xa9" 0));
  Check.equal_int "U+3042 is three bytes" ~expected:3 ~actual:(snd (decode "\xe3\x81\x82" 0));
  Check.equal_int "U+1F600 is four bytes" ~expected:4
    ~actual:(snd (decode "\xf0\x9f\x98\x80" 0));
  Check.equal_int "U+3042 decodes" ~expected:0x3042 ~actual:(fst (decode "\xe3\x81\x82" 0));
  Check.equal_int "U+1F600 decodes" ~expected:0x1F600
    ~actual:(fst (decode "\xf0\x9f\x98\x80" 0));
  Check.equal_int "U+0000 decodes" ~expected:0 ~actual:(fst (decode "\x00" 0));
  Check.equal_int "U+10FFFF decodes" ~expected:0x10FFFF
    ~actual:(fst (decode "\xf4\x8f\xbf\xbf" 0));

  (* Boundaries. `前` is three bytes, so 1 and 2 are interior. *)
  let text = "\xe5\x89\x8dA\xe3\x81\x82" in
  Check.equal_bool "offset 0 is a boundary" ~expected:true ~actual:(is_boundary text 0);
  Check.equal_bool "offset 1 is interior" ~expected:false ~actual:(is_boundary text 1);
  Check.equal_bool "offset 2 is interior" ~expected:false ~actual:(is_boundary text 2);
  Check.equal_bool "offset 3 is a boundary" ~expected:true ~actual:(is_boundary text 3);
  Check.equal_bool "offset 4 is a boundary" ~expected:true ~actual:(is_boundary text 4);
  Check.equal_bool "offset 5 is interior" ~expected:false ~actual:(is_boundary text 5);
  Check.equal_bool "the end is a boundary" ~expected:true
    ~actual:(is_boundary text (String.length text));
  Check.equal_bool "past the end is not" ~expected:false
    ~actual:(is_boundary text (String.length text + 1));
  Check.equal_bool "a negative offset is not" ~expected:false ~actual:(is_boundary text (-1));
  Check.equal_bool "the empty string's zero is a boundary" ~expected:true
    ~actual:(is_boundary "" 0);

  (* Counting and iteration. *)
  Check.equal_int "three scalars in a mixed string" ~expected:3 ~actual:(length text);
  Check.equal_int "the empty string has none" ~expected:0 ~actual:(length "");
  Check.ok "scalars come out in order"
    (scalars text = [ 0x524D; Char.code 'A'; 0x3042 ]);

  (* Strict rejection. *)
  Check.raises "an overlong two-byte NUL" (fun () -> decode "\xc0\x80" 0);
  Check.raises "an overlong three-byte solidus" (fun () -> decode "\xe0\x80\xaf" 0);
  Check.raises "an overlong four-byte sequence" (fun () -> decode "\xf0\x80\x80\xaf" 0);
  Check.raises "a high surrogate" (fun () -> decode "\xed\xa0\x80" 0);
  Check.raises "a low surrogate" (fun () -> decode "\xed\xb0\x80" 0);
  Check.raises "a scalar above U+10FFFF" (fun () -> decode "\xf4\x90\x80\x80" 0);
  Check.raises "a five-byte lead" (fun () -> decode "\xf8\x88\x80\x80\x80" 0);
  Check.raises "a bare continuation byte" (fun () -> decode "\x80" 0);
  Check.raises "a truncated three-byte sequence" (fun () -> decode "\xe3\x81" 0);
  Check.raises "a lead byte where a continuation belongs" (fun () -> decode "\xe3\x41\x82" 0);
  Check.raises "decoding past the end" (fun () -> decode "A" 1);
  Check.equal_bool "an invalid string is not valid" ~expected:false ~actual:(is_valid "\xff");
  Check.equal_bool "a valid string is" ~expected:true ~actual:(is_valid text);

  (* Encoding round-trips every boundary of every width. *)
  List.iter
    (fun code ->
      let encoded = of_scalar code in
      let decoded, width = decode encoded 0 in
      Check.equal_int (Printf.sprintf "U+%04X round-trips" code) ~expected:code ~actual:decoded;
      Check.equal_int
        (Printf.sprintf "U+%04X keeps its width" code)
        ~expected:(String.length encoded) ~actual:width)
    [ 0x0000; 0x007F; 0x0080; 0x07FF; 0x0800; 0xD7FF; 0xE000; 0xFFFF; 0x10000; 0x10FFFF ];
  Check.raises "encoding a surrogate" (fun () -> of_scalar 0xD800);
  Check.raises "encoding above U+10FFFF" (fun () -> of_scalar 0x110000);

  (* One scalar, for the protocol's `scalar` type. *)
  Check.ok "a single ASCII scalar" (single_scalar "A" = Some (Char.code 'A'));
  Check.ok "a single astral scalar" (single_scalar "\xf0\x9f\x98\x80" = Some 0x1F600);
  Check.ok "two scalars are not one" (single_scalar "AB" = None);
  Check.ok "no scalars are not one" (single_scalar "" = None);
  Check.ok "invalid bytes are not one scalar" (single_scalar "\xff" = None)
