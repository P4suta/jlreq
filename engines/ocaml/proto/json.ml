(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** A JSON reader and writer for exactly the protocol's JSON.

    Written here rather than taken from opam because the engine's whole claim is
    that it shares nothing with the Rust implementation, and a dependency-free
    build is the cheapest way to keep that claim checkable. The protocol needs
    six of JSON's types and no more.

    Two properties matter more than anything else in this file.

    {b Numbers are integers.} There is no float constructor. The runner compares
    responses by parsing both sides into [serde_json::Value] and testing
    structural equality, and [Number(1)] is not [Number(1.0)] there: one emitted
    [2500.0] fails a case that [2500] passes, with a diff that looks like an
    arithmetic bug. The parser refuses [.], [e] and [E] in a number rather than
    rounding them away, so a malformed request is an error and never a silent
    reinterpretation.

    {b Null is not absence.} [{"symbol": null}] and [{}] are different messages:
    an attachment with no repeated symbol must state [null], and the schema
    rejects the field's absence. {!member} answers [None] for an absent field and
    [Some Null] for a null one, and nothing in this engine may collapse the two. *)

type t =
  | Null
  | Bool of bool
  | Int of int64
  | String of string
  | Array of t list
  | Object of (string * t) list  (** In the order written; keys are unique. *)

exception Invalid of string
(** Raised on input that is not the JSON this protocol uses. *)

let fail format = Printf.ksprintf (fun message -> raise (Invalid message)) format

(* ----------------------------------------------------------------------------- *)
(* Reading *)
(* ----------------------------------------------------------------------------- *)

type reader = { text : string; mutable at : int }

let peek reader = if reader.at < String.length reader.text then Some reader.text.[reader.at] else None

let advance reader = reader.at <- reader.at + 1

let is_space character =
  character = ' ' || character = '\t' || character = '\n' || character = '\r'

let skip_space reader =
  while match peek reader with Some character -> is_space character | None -> false do
    advance reader
  done

let expect reader character =
  match peek reader with
  | Some found when found = character -> advance reader
  | Some found -> fail "byte %d: expected `%c`, found `%c`" reader.at character found
  | None -> fail "byte %d: expected `%c`, found the end of the message" reader.at character

let literal reader word value =
  let width = String.length word in
  if
    reader.at + width <= String.length reader.text
    && String.equal (String.sub reader.text reader.at width) word
  then begin
    reader.at <- reader.at + width;
    value
  end
  else fail "byte %d: not a JSON value" reader.at

(** Four hex digits, as a number. *)
let hex4 reader =
  let digit () =
    match peek reader with
    | Some character ->
      advance reader;
      let value = Char.code character in
      if value >= Char.code '0' && value <= Char.code '9' then value - Char.code '0'
      else if value >= Char.code 'a' && value <= Char.code 'f' then value - Char.code 'a' + 10
      else if value >= Char.code 'A' && value <= Char.code 'F' then value - Char.code 'A' + 10
      else fail "byte %d: `\\u` needs four hexadecimal digits" reader.at
    | None -> fail "byte %d: `\\u` needs four hexadecimal digits" reader.at
  in
  let a = digit () in
  let b = digit () in
  let c = digit () in
  let d = digit () in
  (a lsl 12) lor (b lsl 8) lor (c lsl 4) lor d

(** Append the UTF-8 encoding of one scalar. Kept local so [jlreq_proto] does not
    reach into [jlreq]: the protocol layer knows JSON, the engine layer knows text,
    and neither borrows the other's job. *)
let add_scalar buffer code =
  if code < 0x80 then Buffer.add_char buffer (Char.chr code)
  else if code < 0x800 then begin
    Buffer.add_char buffer (Char.chr (0xC0 lor (code lsr 6)));
    Buffer.add_char buffer (Char.chr (0x80 lor (code land 0x3F)))
  end
  else if code < 0x10000 then begin
    Buffer.add_char buffer (Char.chr (0xE0 lor (code lsr 12)));
    Buffer.add_char buffer (Char.chr (0x80 lor ((code lsr 6) land 0x3F)));
    Buffer.add_char buffer (Char.chr (0x80 lor (code land 0x3F)))
  end
  else begin
    Buffer.add_char buffer (Char.chr (0xF0 lor (code lsr 18)));
    Buffer.add_char buffer (Char.chr (0x80 lor ((code lsr 12) land 0x3F)));
    Buffer.add_char buffer (Char.chr (0x80 lor ((code lsr 6) land 0x3F)));
    Buffer.add_char buffer (Char.chr (0x80 lor (code land 0x3F)))
  end

let read_string reader =
  expect reader '"';
  let buffer = Buffer.create 32 in
  let rec step () =
    match peek reader with
    | None -> fail "byte %d: the string is not closed" reader.at
    | Some '"' ->
      advance reader;
      Buffer.contents buffer
    | Some '\\' -> (
      advance reader;
      match peek reader with
      | None -> fail "byte %d: the string is not closed" reader.at
      | Some character ->
        advance reader;
        (match character with
        | '"' -> Buffer.add_char buffer '"'
        | '\\' -> Buffer.add_char buffer '\\'
        | '/' -> Buffer.add_char buffer '/'
        | 'b' -> Buffer.add_char buffer '\b'
        | 'f' -> Buffer.add_char buffer '\012'
        | 'n' -> Buffer.add_char buffer '\n'
        | 'r' -> Buffer.add_char buffer '\r'
        | 't' -> Buffer.add_char buffer '\t'
        | 'u' ->
          let first = hex4 reader in
          if first >= 0xD800 && first <= 0xDBFF then begin
            (* A high surrogate must be followed by its low half. *)
            expect reader '\\';
            expect reader 'u';
            let second = hex4 reader in
            if second < 0xDC00 || second > 0xDFFF then
              fail "byte %d: `\\u%04X` is a high surrogate without a low one" reader.at first;
            add_scalar buffer
              (0x10000 + ((first - 0xD800) lsl 10) + (second - 0xDC00))
          end
          else if first >= 0xDC00 && first <= 0xDFFF then
            fail "byte %d: `\\u%04X` is a low surrogate with no high one" reader.at first
          else add_scalar buffer first
        | other -> fail "byte %d: `\\%c` is not a JSON escape" reader.at other);
        step ())
    | Some character ->
      if Char.code character < 0x20 then
        fail "byte %d: a raw control character must be escaped" reader.at;
      Buffer.add_char buffer character;
      advance reader;
      step ()
  in
  step ()

let read_number reader =
  let start = reader.at in
  if peek reader = Some '-' then advance reader;
  let digits_from = reader.at in
  while match peek reader with Some c when c >= '0' && c <= '9' -> true | _ -> false do
    advance reader
  done;
  if reader.at = digits_from then fail "byte %d: not a JSON value" start;
  (match peek reader with
  | Some ('.' | 'e' | 'E') ->
    fail
      "byte %d: this protocol carries integers only, and `%s` is not one"
      start
      (String.sub reader.text start (min 24 (String.length reader.text - start)))
  | _ -> ());
  let text = String.sub reader.text start (reader.at - start) in
  (* A leading zero is not JSON: `01` is two tokens, not one. *)
  let body = if text.[0] = '-' then String.sub text 1 (String.length text - 1) else text in
  if String.length body > 1 && body.[0] = '0' then
    fail "byte %d: `%s` has a leading zero" start text;
  match Int64.of_string_opt text with
  | Some value -> Int value
  | None -> fail "byte %d: `%s` does not fit in 64 bits" start text

let rec read_value reader =
  skip_space reader;
  match peek reader with
  | None -> fail "byte %d: the message ends where a value belongs" reader.at
  | Some '{' -> read_object reader
  | Some '[' -> read_array reader
  | Some '"' -> String (read_string reader)
  | Some 't' -> literal reader "true" (Bool true)
  | Some 'f' -> literal reader "false" (Bool false)
  | Some 'n' -> literal reader "null" Null
  | Some ('-' | '0' .. '9') -> read_number reader
  | Some character -> fail "byte %d: `%c` starts no JSON value" reader.at character

and read_array reader =
  expect reader '[';
  skip_space reader;
  if peek reader = Some ']' then begin
    advance reader;
    Array []
  end
  else begin
    let items = ref [] in
    let rec step () =
      items := read_value reader :: !items;
      skip_space reader;
      match peek reader with
      | Some ',' ->
        advance reader;
        step ()
      | Some ']' -> advance reader
      | Some character -> fail "byte %d: expected `,` or `]`, found `%c`" reader.at character
      | None -> fail "byte %d: the array is not closed" reader.at
    in
    step ();
    Array (List.rev !items)
  end

and read_object reader =
  expect reader '{';
  skip_space reader;
  if peek reader = Some '}' then begin
    advance reader;
    Object []
  end
  else begin
    let fields = ref [] in
    let rec step () =
      skip_space reader;
      let name = read_string reader in
      if List.mem_assoc name !fields then fail "the object states `%s` twice" name;
      skip_space reader;
      expect reader ':';
      let value = read_value reader in
      fields := (name, value) :: !fields;
      skip_space reader;
      match peek reader with
      | Some ',' ->
        advance reader;
        step ()
      | Some '}' -> advance reader
      | Some character -> fail "byte %d: expected `,` or `}`, found `%c`" reader.at character
      | None -> fail "byte %d: the object is not closed" reader.at
    in
    step ();
    Object (List.rev !fields)
  end

(** One whole JSON value, with nothing but whitespace after it.

    @raise Invalid on anything else. *)
let parse (text : string) : t =
  let reader = { text; at = 0 } in
  let value = read_value reader in
  skip_space reader;
  if reader.at <> String.length text then
    fail "byte %d: the message continues after its value ends" reader.at;
  value

(* ----------------------------------------------------------------------------- *)
(* Writing *)
(* ----------------------------------------------------------------------------- *)

let write_string buffer text =
  Buffer.add_char buffer '"';
  String.iter
    (fun character ->
      match character with
      | '"' -> Buffer.add_string buffer "\\\""
      | '\\' -> Buffer.add_string buffer "\\\\"
      | '\b' -> Buffer.add_string buffer "\\b"
      | '\012' -> Buffer.add_string buffer "\\f"
      | '\n' -> Buffer.add_string buffer "\\n"
      | '\r' -> Buffer.add_string buffer "\\r"
      | '\t' -> Buffer.add_string buffer "\\t"
      | character when Char.code character < 0x20 ->
        Buffer.add_string buffer (Printf.sprintf "\\u%04X" (Char.code character))
      | character -> Buffer.add_char buffer character)
    text;
  Buffer.add_char buffer '"'

let rec write buffer value =
  match value with
  | Null -> Buffer.add_string buffer "null"
  | Bool true -> Buffer.add_string buffer "true"
  | Bool false -> Buffer.add_string buffer "false"
  | Int number -> Buffer.add_string buffer (Int64.to_string number)
  | String text -> write_string buffer text
  | Array items ->
    Buffer.add_char buffer '[';
    List.iteri
      (fun index item ->
        if index > 0 then Buffer.add_char buffer ',';
        write buffer item)
      items;
    Buffer.add_char buffer ']'
  | Object fields ->
    Buffer.add_char buffer '{';
    List.iteri
      (fun index (name, item) ->
        if index > 0 then Buffer.add_char buffer ',';
        write_string buffer name;
        Buffer.add_char buffer ':';
        write buffer item)
      fields;
    Buffer.add_char buffer '}'

(** One value as compact JSON on a single line, which is what NDJSON wants. *)
let to_string (value : t) : string =
  let buffer = Buffer.create 256 in
  write buffer value;
  Buffer.contents buffer

(* ----------------------------------------------------------------------------- *)
(* Reaching into a value *)
(* ----------------------------------------------------------------------------- *)

(** The field named [name], or [None] if the object does not state it.

    [Some Null] means the object states the field and states it null, which is a
    different message from not stating it. *)
let member (name : string) (value : t) : t option =
  match value with Object fields -> List.assoc_opt name fields | _ -> None

(** Whether the object states [name] at all, null or not. *)
let has (name : string) (value : t) : bool =
  match value with Object fields -> List.mem_assoc name fields | _ -> false

(** The field names of an object, in the order written. *)
let names (value : t) : string list =
  match value with Object fields -> List.map fst fields | _ -> []

(** A native [int] as a protocol number. *)
let of_int (value : int) : t = Int (Int64.of_int value)

(** The empty object. *)
let empty_object : t = Object []
