(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** The tab-separated files under [spec/].

    Both families this engine reads -- the derived tables under [spec/derived/] and
    the transcribed matrices under [spec/captured/] -- use one shape:

    - a preamble of [#] comment lines, then a blank line;
    - one header line naming the columns;
    - one data line per row, fields separated by [U+0009].

    A field never holds a raw tab or a raw newline. Prose fields (the Remarks
    column of Appendix A is the only one in this engine's inputs) escape a line
    break as [\n] and a backslash as [\\], and no other escape exists: an unknown
    escape is refused rather than passed through, because a field nothing writes is
    a field nobody can read.

    Blank lines and [#] lines are dropped wherever they appear, not only in the
    preamble, so the reader does not depend on a fixed number of header comments --
    the derived files carry a source digest in their preamble and gain a line
    whenever the tooling that writes them changes. *)

exception Invalid of string
(** Raised on a malformed file: a bad escape, a missing column, a short row. *)

let fail format = Printf.ksprintf (fun message -> raise (Invalid message)) format

type t = {
  header : string array;
  rows : string array list;  (** In file order. *)
}

(** One field with its escapes resolved. *)
let unescape (field : string) : string =
  if not (String.contains field '\\') then field
  else begin
    let out = Buffer.create (String.length field) in
    let length = String.length field in
    let rec step index =
      if index >= length then ()
      else if field.[index] <> '\\' then begin
        Buffer.add_char out field.[index];
        step (index + 1)
      end
      else if index + 1 >= length then
        fail "`%s` ends in a backslash, which is not an escape" field
      else
        match field.[index + 1] with
        | 'n' ->
          Buffer.add_char out '\n';
          step (index + 2)
        | '\\' ->
          Buffer.add_char out '\\';
          step (index + 2)
        | other -> fail "`%s` holds the escape `\\%c`, which nothing writes" field other
    in
    step 0;
    Buffer.contents out
  end

(** [text] split at every [U+0009], keeping empty fields at both ends. *)
let split_tabs (line : string) : string array =
  let count = ref 1 in
  String.iter (fun character -> if character = '\t' then incr count) line;
  let fields = Array.make !count "" in
  let start = ref 0 and index = ref 0 in
  String.iteri
    (fun position character ->
      if character = '\t' then begin
        fields.(!index) <- String.sub line !start (position - !start);
        incr index;
        start := position + 1
      end)
    line;
  fields.(!index) <- String.sub line !start (String.length line - !start);
  fields

(** [text] split at every [U+000A], with a trailing [U+000D] removed from each
    line so a file checked out with CRLF endings still reads. *)
let lines (text : string) : string list =
  let out = ref [] and start = ref 0 in
  let length = String.length text in
  let push stop =
    let stop = if stop > !start && text.[stop - 1] = '\r' then stop - 1 else stop in
    out := String.sub text !start (stop - !start) :: !out
  in
  String.iteri
    (fun position character ->
      if character = '\n' then begin
        push position;
        start := position + 1
      end)
    text;
  if !start < length then push length;
  List.rev !out

(** Whether a line carries data: not blank, not a comment. *)
let is_data (line : string) : bool =
  String.length line > 0 && line.[0] <> '#' && String.trim line <> ""

(** Parse a whole file.

    @raise Invalid if the file has no header line, or a row's field count differs
      from the header's. *)
let parse (text : string) : t =
  match List.filter is_data (lines text) with
  | [] -> fail "the file holds no header line"
  | header :: body ->
    let header = Array.map unescape (split_tabs header) in
    let width = Array.length header in
    let rows =
      List.mapi
        (fun index line ->
          let fields = split_tabs line in
          if Array.length fields <> width then
            fail "data row %d has %d field(s) where the header names %d" (index + 1)
              (Array.length fields) width;
          Array.map unescape fields)
        body
    in
    { header; rows }

(** The index of the column named [name].

    @raise Invalid if the file does not have that column. *)
let column (file : t) (name : string) : int =
  let rec search index =
    if index >= Array.length file.header then
      fail "the file has no column named `%s`" name
    else if String.equal (String.trim file.header.(index)) name then index
    else search (index + 1)
  in
  search 0

(** How many data rows the file holds. *)
let row_count (file : t) : int = List.length file.rows

(** [field row index] with a message that names the column when the row is short.

    Rows are already known to be the header's width, so this only fires on a
    programming error. *)
let field (row : string array) (index : int) : string = row.(index)
