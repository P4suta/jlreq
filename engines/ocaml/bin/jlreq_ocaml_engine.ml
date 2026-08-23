(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** The engine process.

    [jlreq-conformance run] starts this executable once, with no arguments and no
    environment it tells us about, writes every request to stdin as one JSON
    object per line, closes stdin, and reads one response line per request after
    the process exits. There is no handshake and no way to ask a question back.

    Exit codes are the protocol's:

    - [0] every request was answered;
    - [2] the specification tables did not come out right, a line was not JSON, or
      an envelope named a protocol or a specification this engine does not speak.

    A wrong {i answer} is not an error here. It is reported by the runner as a
    DIFF and exits the {i runner} 1. At milestone M0 every non-empty case is a
    DIFF, because {!Protocol.empty_response} is all this engine knows how to say. *)

let program = "jlreq-ocaml-engine"

let die message =
  prerr_endline (program ^ ": " ^ message);
  exit 2

(** Answer every line of stdin. *)
let serve () =
  let rec loop () =
    match input_line stdin with
    | exception End_of_file -> ()
    | line ->
      (match Jlreq_proto.Protocol.request_of_line line with
      | None -> ()
      | Some request ->
        let envelope =
          Jlreq_proto.Protocol.envelope_of_response ~id:request.Jlreq_proto.Protocol.id
            ~response:Jlreq_proto.Protocol.empty_response
        in
        print_string (Jlreq_proto.Json.to_string envelope);
        print_char '\n');
      loop ()
  in
  loop ()

let () =
  (* Line endings are ours to choose and the runner splits on `\n`, so the streams
     are binary on every platform: a `\r` the runtime inserted would be a byte the
     engine did not decide to write. *)
  set_binary_mode_in stdin true;
  set_binary_mode_out stdout true;
  (try Jlreq.Tables.self_check () with
  | Jlreq.Tables.Invalid message -> die ("specification tables: " ^ message)
  | Jlreq.Tsv.Invalid message -> die ("specification tables: " ^ message));
  (try serve () with
  | Jlreq_proto.Json.Invalid message -> die message
  | Jlreq_proto.Protocol.Invalid message -> die message
  | Jlreq.Utf8.Malformed message -> die message
  | Jlreq.Tables.Invalid message -> die message
  | Jlreq.Tsv.Invalid message -> die message);
  flush stdout
