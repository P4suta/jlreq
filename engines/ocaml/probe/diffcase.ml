(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** One conformance case, run and compared field by field.

    [jlreq-conformance run] answers a wrong case with the single word [DIFF] and
    the case identifier. That is the right report for a gate and the wrong one for
    the person who has to fix it: the engine returned five hundred bytes of JSON,
    the suite expected six hundred, and the question is always which number moved.
    This probe answers that question and nothing else.

    {v
    diffcase quick-start/two-lines
    lines: expected 2 element(s), got 0
    lines[0]: expected {"attachments":[],"block_extent":1000,...}, got nothing
    lines[1]: expected {"attachments":[],"block_extent":1000,...}, got nothing
    DIFF quick-start/two-lines: 3 difference(s)
    v}

    The comparison is structural and not textual, for the same reason the runner's
    is: the two engines write the same object with different key orders -- the Rust
    side's [serde_json] sorts them, this side writes [lines] before [diagnostics] --
    and a textual diff would report every response as different. Order is not part
    of the answer; a missing key is.

    The reference side is the suite's own [expected] by default, or the Rust sample
    engine's live answer under [--rust]. The second is what a milestone is actually
    debugged against: the sample engine can be handed a request the suite does not
    contain (a census case, a reduced repro) and still say what the answer should
    be.

    Exit codes are the ones a shell wants: [0] the answers match, [1] they differ,
    [2] the probe could not get an answer at all -- no such case, an engine that
    exited nonzero, output that is not the protocol. A difference is a result, and
    only a broken run is an error.

    This is a development probe. It is outside the engine, the runner never starts
    it, and it may read files at run time, which the engine itself may not
    (docs/design/conformance.md, "The runner contract"). *)

let program = "diffcase"

exception Fault of string
(** Raised on anything that stops the probe from producing a comparison. *)

let fault format = Printf.ksprintf (fun message -> raise (Fault message)) format

(* ----------------------------------------------------------------------------- *)
(* Files and subprocesses *)
(* ----------------------------------------------------------------------------- *)

let read_file (path : string) : string =
  match open_in_bin path with
  | exception Sys_error message -> fault "%s" message
  | channel ->
    Fun.protect
      ~finally:(fun () -> close_in_noerr channel)
      (fun () -> really_input_string channel (in_channel_length channel))

let write_file (path : string) (text : string) : unit =
  match open_out_bin path with
  | exception Sys_error message -> fault "%s" message
  | channel ->
    Fun.protect
      ~finally:(fun () -> close_out_noerr channel)
      (fun () -> output_string channel text)

(** [text] split on newlines, with a trailing newline producing no final piece. *)
let lines_of (text : string) : string list =
  let pieces = String.split_on_char '\n' text in
  match List.rev pieces with "" :: rest -> List.rev rest | _ -> pieces

(** Run [engine] with [input] on its stdin and return its stdout.

    The engine is started the way the runner starts it -- the path, no arguments --
    through two temporary files rather than a pipe, because a pipe needs [Unix] and
    this tree's OCaml depends on the compiler and dune and nothing else. Stderr is
    left alone: an engine that exits 2 has already explained itself there, and the
    message is the most useful thing on the screen.

    @raise Fault if the engine exits nonzero, which the protocol reserves for a
      transport, JSON or specification-table failure rather than a wrong answer. *)
let run_engine ~(engine : string) ~(input : string) : string =
  let request_path = Filename.temp_file "jlreq-diffcase" ".request" in
  let response_path = Filename.temp_file "jlreq-diffcase" ".response" in
  Fun.protect
    ~finally:(fun () ->
      (try Sys.remove request_path with Sys_error _ -> ());
      try Sys.remove response_path with Sys_error _ -> ())
    (fun () ->
      write_file request_path input;
      let command =
        Filename.quote_command engine [] ~stdin:request_path ~stdout:response_path
      in
      let status = Sys.command command in
      if status <> 0 then fault "engine `%s` exited with %d" engine status;
      read_file response_path)

(* ----------------------------------------------------------------------------- *)
(* The suite file *)
(* ----------------------------------------------------------------------------- *)

(** The case [id] states, as the whole suite object.

    A suite line carries two fields an engine never sees: [rules], which is the
    coverage gate's provenance, and [expected]. They stay here and are stripped
    when the envelope is built. *)
let find_case ~(suite : string) ~(id : string) : Jlreq_proto.Json.t =
  let text = read_file suite in
  let rec search number remaining =
    match remaining with
    | [] -> fault "%s states no case `%s`" suite id
    | line :: rest ->
      if String.equal (String.trim line) "" then search (number + 1) rest
      else
        let value =
          match Jlreq_proto.Json.parse line with
          | value -> value
          | exception Jlreq_proto.Json.Invalid message ->
            fault "%s line %d: %s" suite number message
        in
        if Jlreq_proto.Json.member "id" value = Some (Jlreq_proto.Json.String id) then value
        else search (number + 1) rest
  in
  search 1 (lines_of text)

(** The request envelope for a case, with [rules] and [expected] removed.

    The protocol and specification identifiers are this engine's own rather than
    the suite's, and a suite that disagrees with them is a fault: it would mean the
    tree's engines and its suite were built against different revisions, which is a
    thing to fix and not a thing to paper over by echoing whatever the file says. *)
let envelope_of_case (case : Jlreq_proto.Json.t) ~(id : string) : Jlreq_proto.Json.t =
  let stated name wanted =
    match Jlreq_proto.Json.member name case with
    | Some (Jlreq_proto.Json.String found) when String.equal found wanted -> ()
    | Some (Jlreq_proto.Json.String found) ->
      fault "case `%s` states %s `%s`, and this tree's engines speak `%s`" id name found wanted
    | _ -> fault "case `%s` does not state %s" id name
  in
  stated "protocol" Jlreq_proto.Protocol.protocol;
  stated "spec" Jlreq_proto.Protocol.spec;
  let request =
    match Jlreq_proto.Json.member "request" case with
    | Some (Jlreq_proto.Json.Object _ as found) -> found
    | _ -> fault "case `%s` states no request object" id
  in
  Jlreq_proto.Json.Object
    [
      ("protocol", Jlreq_proto.Json.String Jlreq_proto.Protocol.protocol);
      ("spec", Jlreq_proto.Json.String Jlreq_proto.Protocol.spec);
      ("id", Jlreq_proto.Json.String id);
      ("request", request);
    ]

(** The [response] object of the one envelope [text] holds. *)
let response_of_output ~(engine : string) ~(id : string) (text : string) : Jlreq_proto.Json.t =
  let answers =
    List.filter (fun line -> String.trim line <> "") (lines_of text)
  in
  match answers with
  | [] -> fault "engine `%s` answered nothing" engine
  | _ :: _ :: _ -> fault "engine `%s` answered %d times for one request" engine (List.length answers)
  | [ line ] -> (
    let value =
      match Jlreq_proto.Json.parse line with
      | value -> value
      | exception Jlreq_proto.Json.Invalid message ->
        fault "engine `%s`: %s" engine message
    in
    (match Jlreq_proto.Json.member "id" value with
    | Some (Jlreq_proto.Json.String found) when String.equal found id -> ()
    | Some (Jlreq_proto.Json.String found) ->
      fault "engine `%s` answered case `%s` with id `%s`" engine id found
    | _ -> fault "engine `%s` answered without an id" engine);
    match Jlreq_proto.Json.member "response" value with
    | Some (Jlreq_proto.Json.Object _ as found) -> found
    | _ -> fault "engine `%s` answered without a response object" engine)

(* ----------------------------------------------------------------------------- *)
(* The structural comparison *)
(* ----------------------------------------------------------------------------- *)

(** How much of a value to show before cutting it off.

    A whole expected line is a few hundred bytes and repeating it teaches nothing;
    the path is what locates the defect. The cut lands on a UTF-8 boundary so the
    output stays text. *)
let preview_limit = 120

let preview (value : Jlreq_proto.Json.t) : string =
  let text = Jlreq_proto.Json.to_string value in
  if String.length text <= preview_limit then text
  else begin
    let cut = ref preview_limit in
    while !cut > 0 && not (Jlreq.Utf8.is_boundary text !cut) do
      decr cut
    done;
    String.sub text 0 !cut ^ "\xe2\x80\xa6"
  end

(** [lines[0].clusters[3]] one step deeper. The root path is empty, so the first
    key carries no leading dot. *)
let field (path : string) (name : string) : string =
  if String.equal path "" then name else path ^ "." ^ name

let element (path : string) (index : int) : string = Printf.sprintf "%s[%d]" path index

let rec compare_values (out : (string * string) list ref) (path : string)
    (expected : Jlreq_proto.Json.t) (actual : Jlreq_proto.Json.t) : unit =
  let note where message = out := (where, message) :: !out in
  match (expected, actual) with
  | Jlreq_proto.Json.Object left, Jlreq_proto.Json.Object right ->
    List.iter
      (fun (name, value) ->
        match List.assoc_opt name right with
        | Some found -> compare_values out (field path name) value found
        | None -> note (field path name) (Printf.sprintf "expected %s, got nothing" (preview value)))
      left;
    List.iter
      (fun (name, value) ->
        if not (List.mem_assoc name left) then
          note (field path name) (Printf.sprintf "nothing expected, got %s" (preview value)))
      right
  | Jlreq_proto.Json.Array left, Jlreq_proto.Json.Array right ->
    let wanted = Array.of_list left and found = Array.of_list right in
    if Array.length wanted <> Array.length found then
      note path
        (Printf.sprintf "expected %d element(s), got %d" (Array.length wanted) (Array.length found));
    Array.iteri
      (fun index value ->
        if index < Array.length found then
          compare_values out (element path index) value found.(index)
        else note (element path index) (Printf.sprintf "expected %s, got nothing" (preview value)))
      wanted;
    Array.iteri
      (fun index value ->
        if index >= Array.length wanted then
          note (element path index) (Printf.sprintf "nothing expected, got %s" (preview value)))
      found
  | _ ->
    if expected <> actual then
      note path (Printf.sprintf "expected %s, got %s" (preview expected) (preview actual))

(** Every difference between two responses, outermost first. *)
let differences (expected : Jlreq_proto.Json.t) (actual : Jlreq_proto.Json.t) :
    (string * string) list =
  let out = ref [] in
  compare_values out "" expected actual;
  List.rev !out

(* ----------------------------------------------------------------------------- *)
(* The command line *)
(* ----------------------------------------------------------------------------- *)

type options = {
  mutable case : string option;
  mutable engine : string;
  mutable reference : string;
  mutable suite : string;
  mutable against_rust : bool;
}

(* The defaults are the paths `just` builds into, resolved against the repository
   root, so `diffcase <case>` works from a checkout with nothing else said. The
   `just diffcase` recipe passes all three explicitly anyway, because it is the
   file that knows where dune and cargo were told to write. *)
let defaults () =
  {
    case = None;
    engine = "target/dune/default/engines/ocaml/bin/jlreq_ocaml_engine.exe";
    reference = "target/debug/jlreq-sample-engine";
    suite = "crates/jlreq-conformance/suite.ndjson";
    against_rust = false;
  }

let usage =
  "usage: diffcase <case-id> [--engine PATH] [--rust] [--reference PATH] [--suite PATH]"

let parse_arguments (arguments : string list) : options =
  let options = defaults () in
  let rec step remaining =
    match remaining with
    | [] -> ()
    | "--rust" :: rest ->
      options.against_rust <- true;
      step rest
    | "--engine" :: value :: rest ->
      options.engine <- value;
      step rest
    | "--reference" :: value :: rest ->
      options.reference <- value;
      step rest
    | "--suite" :: value :: rest ->
      options.suite <- value;
      step rest
    | ("--engine" | "--reference" | "--suite") :: [] ->
      fault "%s needs a value\n%s" (List.hd remaining) usage
    | argument :: rest ->
      if String.length argument > 0 && argument.[0] = '-' then
        fault "`%s` is not an option\n%s" argument usage;
      if options.case <> None then fault "more than one case identifier\n%s" usage;
      options.case <- Some argument;
      step rest
  in
  step arguments;
  options

let run (arguments : string list) : int =
  let options = parse_arguments arguments in
  let id = match options.case with Some id -> id | None -> fault "%s" usage in
  let case = find_case ~suite:options.suite ~id in
  let envelope = envelope_of_case case ~id in
  let message = Jlreq_proto.Json.to_string envelope ^ "\n" in
  let actual =
    response_of_output ~engine:options.engine ~id (run_engine ~engine:options.engine ~input:message)
  in
  let expected =
    if options.against_rust then
      response_of_output ~engine:options.reference ~id
        (run_engine ~engine:options.reference ~input:message)
    else
      match Jlreq_proto.Json.member "expected" case with
      | Some (Jlreq_proto.Json.Object _ as found) -> found
      | _ -> fault "case `%s` carries no expected response; try --rust" id
  in
  match differences expected actual with
  | [] ->
    Printf.printf "MATCH %s\n" id;
    0
  | found ->
    List.iter (fun (path, message) -> Printf.printf "%s: %s\n" path message) found;
    Printf.printf "DIFF %s: %d difference(s) against %s\n" id (List.length found)
      (if options.against_rust then options.reference else options.suite);
    1

let () =
  set_binary_mode_out stdout true;
  let arguments = List.tl (Array.to_list Sys.argv) in
  let status =
    try run arguments with
    | Fault message ->
      prerr_endline (program ^ ": " ^ message);
      2
    | Jlreq_proto.Json.Invalid message ->
      prerr_endline (program ^ ": " ^ message);
      2
    | Jlreq.Utf8.Malformed message ->
      prerr_endline (program ^ ": " ^ message);
      2
  in
  flush stdout;
  exit status
