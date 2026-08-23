(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** The unit tests, in dependency order: arithmetic, then text, then the file
    format, then the tables built out of it, then the wire format on top.

    Run with [dune runtest]. Everything is in one executable so that one run
    reports every failure; the sections are separate modules so that a failure
    names the layer it came from. *)

let () =
  print_endline "-- Num";
  Test_num.run ();
  print_endline "-- Utf8";
  Test_utf8.run ();
  print_endline "-- Tsv";
  Test_tsv.run ();
  print_endline "-- Tables";
  Test_tables.run ();
  print_endline "-- Json";
  Test_json.run ();
  print_endline "-- Protocol";
  Test_protocol.run ();
  Check.report ()
