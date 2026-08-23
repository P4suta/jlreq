(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** {!Jlreq.Tsv}: comment preambles, escapes, and ragged rows. *)

let sample =
  "# a preamble\n\
   #\n\
   # Source SHA-256: 00\n\
   \n\
   class\tkey\tremark-en\tremark-ja\n\
   cl-01\t2018\tused in horizontal composition\t\xe6\xa8\xaa\xe7\xb5\x84\n\
   cl-08\t3033\tone\\ntwo\tone\\ntwo\n\
   cl-24\t0020\t\t\n"

let run () =
  let file = Jlreq.Tsv.parse sample in
  Check.equal_int "three data rows" ~expected:3 ~actual:(Jlreq.Tsv.row_count file);
  Check.equal_int "four columns" ~expected:4 ~actual:(Array.length file.Jlreq.Tsv.header);
  Check.equal_int "class is column 0" ~expected:0 ~actual:(Jlreq.Tsv.column file "class");
  Check.equal_int "remark-ja is column 3" ~expected:3 ~actual:(Jlreq.Tsv.column file "remark-ja");
  Check.raises "an absent column" (fun () -> Jlreq.Tsv.column file "nonexistent");

  let rows = Array.of_list file.Jlreq.Tsv.rows in
  Check.equal_string "the first row's key" ~expected:"2018" ~actual:rows.(0).(1);
  Check.equal_string "an escaped newline is resolved" ~expected:"one\ntwo" ~actual:rows.(1).(2);
  Check.equal_string "an empty trailing field survives" ~expected:"" ~actual:rows.(2).(3);
  Check.equal_string "an empty interior field survives" ~expected:"" ~actual:rows.(2).(2);

  (* Escapes. *)
  Check.equal_string "a backslash escape" ~expected:"a\\b"
    ~actual:(Jlreq.Tsv.unescape "a\\\\b");
  Check.equal_string "a field with no escape is itself" ~expected:"plain"
    ~actual:(Jlreq.Tsv.unescape "plain");
  Check.equal_string "two escapes in one field" ~expected:"a\nb\\c"
    ~actual:(Jlreq.Tsv.unescape "a\\nb\\\\c");
  Check.raises "an escape nothing writes" (fun () -> Jlreq.Tsv.unescape "a\\tb");
  Check.raises "a trailing backslash" (fun () -> Jlreq.Tsv.unescape "a\\");

  (* Structure. *)
  Check.raises "a file with no header" (fun () -> Jlreq.Tsv.parse "# only comments\n\n");
  Check.raises "a short row" (fun () -> Jlreq.Tsv.parse "a\tb\tc\n1\t2\n");
  Check.raises "a long row" (fun () -> Jlreq.Tsv.parse "a\tb\n1\t2\t3\n");

  (* Comments and blank lines are dropped wherever they appear, and a file
     checked out with CRLF endings reads the same as one with LF. *)
  let interleaved = Jlreq.Tsv.parse "a\tb\r\n1\t2\r\n# a late comment\r\n\r\n3\t4\r\n" in
  Check.equal_int "CRLF and late comments" ~expected:2
    ~actual:(Jlreq.Tsv.row_count interleaved);
  Check.equal_string "no stray carriage return" ~expected:"4"
    ~actual:(List.nth interleaved.Jlreq.Tsv.rows 1).(1);
  Check.equal_string "the header loses its carriage return too" ~expected:"b"
    ~actual:interleaved.Jlreq.Tsv.header.(1);

  (* A file with no trailing newline still yields its last row. *)
  let unterminated = Jlreq.Tsv.parse "a\tb\n1\t2" in
  Check.equal_int "an unterminated last line is a row" ~expected:1
    ~actual:(Jlreq.Tsv.row_count unterminated)
