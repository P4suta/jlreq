(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** The whole test framework.

    A failing check prints and is counted rather than aborting, so one run reports
    every problem instead of the first one. {!report} sets the exit status. *)

let checked = ref 0
let failed = ref 0

let record_failure what detail =
  incr failed;
  Printf.printf "FAIL %s: %s\n" what detail

let ok (what : string) (condition : bool) : unit =
  incr checked;
  if not condition then record_failure what "the condition does not hold"

let equal_int (what : string) ~(expected : int) ~(actual : int) : unit =
  incr checked;
  if expected <> actual then
    record_failure what (Printf.sprintf "expected %d, got %d" expected actual)

let equal_int64 (what : string) ~(expected : int64) ~(actual : int64) : unit =
  incr checked;
  if not (Int64.equal expected actual) then
    record_failure what (Printf.sprintf "expected %Ld, got %Ld" expected actual)

let equal_string (what : string) ~(expected : string) ~(actual : string) : unit =
  incr checked;
  if not (String.equal expected actual) then
    record_failure what (Printf.sprintf "expected `%s`, got `%s`" expected actual)

let equal_bool (what : string) ~(expected : bool) ~(actual : bool) : unit =
  incr checked;
  if expected <> actual then
    record_failure what (Printf.sprintf "expected %b, got %b" expected actual)

(** [raises what f] passes when [f ()] raises anything at all.

    Every exception this engine raises on bad input carries a message and no
    payload worth matching, and pinning the message would make the tests a
    transcription of the error strings. *)
let raises (what : string) (f : unit -> 'a) : unit =
  incr checked;
  match f () with
  | _ -> record_failure what "it was accepted, and it should not have been"
  | exception _ -> ()

(** [returns what f] passes when [f ()] does not raise. *)
let returns (what : string) (f : unit -> 'a) : unit =
  incr checked;
  match f () with
  | _ -> ()
  | exception exn -> record_failure what (Printexc.to_string exn)

let report () =
  Printf.printf "%d check(s), %d failure(s)\n" !checked !failed;
  if !failed > 0 then exit 1
