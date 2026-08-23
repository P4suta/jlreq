(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** The conformance protocol envelope.

    Every message the runner sends and every message the engine sends back names
    the protocol it speaks and the revision of the specification it implements.
    A mismatch on either is an input error and not a conformance difference: the
    runner exits 2 on it, and so does the engine.

    {v
    {"protocol":"jlreq.conformance/1",
     "spec":"jlreq-2020-08-11+unicode-17.0.0",
     "id":"quick-start/two-lines",
     "request":{...}}
    v}

    The response repeats [protocol], [spec] and [id] and replaces [request] with
    [response]. The runner checks the ids in order, so a dropped or reordered
    answer is a protocol error rather than a wrong answer, and every request gets
    exactly one line back.

    The suite files on disk carry two more fields beside [request]: [rules], which
    is the coverage gate's provenance, and [expected]. The runner strips both
    before sending, so an engine never sees them and this reader refuses them --
    an envelope carrying [expected] into an engine would mean the runner is asking
    the engine to grade itself. *)

(** The wire protocol this engine implements. *)
let protocol = "jlreq.conformance/1"

(** The revision of JLReq, with the Unicode version its derived tables were built
    from. This is a specification identifier and not a version of this engine. *)
let spec = "jlreq-2020-08-11+unicode-17.0.0"

exception Invalid of string
(** Raised on an envelope this engine will not answer. *)

let fail format = Printf.ksprintf (fun message -> raise (Invalid message)) format

type request = {
  id : string;  (** The case identifier, echoed verbatim in the response. *)
  body : Json.t;  (** The [request] object, unexamined at this layer. *)
}

(** The one string a field must hold, or an error naming what was found instead. *)
let exact_string (envelope : Json.t) (name : string) (wanted : string) : unit =
  match Json.member name envelope with
  | Some (Json.String found) when String.equal found wanted -> ()
  | Some (Json.String found) -> fail "%s is `%s`, and this engine speaks `%s`" name found wanted
  | Some _ -> fail "%s is not a string" name
  | None -> fail "the envelope does not state %s" name

(** Read one request envelope.

    The envelope is closed: [protocol], [spec], [id] and [request], in any order,
    and nothing else. Rejecting an unknown field here is the same discipline the
    schema applies with [additionalProperties: false] -- a field the engine does
    not understand may be the one carrying the meaning of the request. *)
let request_of_json (envelope : Json.t) : request =
  (match envelope with
  | Json.Object _ -> ()
  | _ -> fail "the message is not a JSON object");
  exact_string envelope "protocol" protocol;
  exact_string envelope "spec" spec;
  let id =
    match Json.member "id" envelope with
    | Some (Json.String "") -> fail "id is empty"
    | Some (Json.String found) -> found
    | Some _ -> fail "id is not a string"
    | None -> fail "the envelope does not state id"
  in
  let body =
    match Json.member "request" envelope with
    | Some (Json.Object _ as found) -> found
    | Some _ -> fail "request is not a JSON object"
    | None -> fail "the envelope does not state request"
  in
  List.iter
    (fun name ->
      match name with
      | "protocol" | "spec" | "id" | "request" -> ()
      | "expected" ->
        fail "the envelope states expected, which the runner strips before sending"
      | other -> fail "the envelope states `%s`, which protocol v1 does not carry" other)
    (Json.names envelope);
  { id; body }

(** A layout result as the [response] object.

    Order is fixed rather than incidental: the runner compares parsed values, so
    field order cannot change an answer, but a stable order makes a captured
    stdout diffable by eye during development. *)
let response ~(lines : Json.t list) ~(diagnostics : Json.t list) : Json.t =
  Json.Object [ ("lines", Json.Array lines); ("diagnostics", Json.Array diagnostics) ]

(** A paragraph that produced no lines and no diagnostics.

    This is what M0 answers with. It is a well-formed, schema-valid response that
    is wrong for all but the emptiest case, which is exactly the intent: the
    runner reports every case as DIFF and exits 1, and the milestones after this
    one replace the answer without touching anything else in this file. *)
let empty_response : Json.t = response ~lines:[] ~diagnostics:[]

(** The envelope carrying one answer. *)
let envelope_of_response ~(id : string) ~(response : Json.t) : Json.t =
  Json.Object
    [
      ("protocol", Json.String protocol);
      ("spec", Json.String spec);
      ("id", Json.String id);
      ("response", response);
    ]

(** One NDJSON line as a request, or [None] for a blank line.

    Blank lines are ignored by the protocol, so they are not an error and do not
    consume an answer. *)
let request_of_line (line : string) : request option =
  if String.equal (String.trim line) "" then None
  else Some (request_of_json (Json.parse line))
