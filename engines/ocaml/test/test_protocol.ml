(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** {!Jlreq_proto.Protocol}: the envelope, and what it refuses. *)

open Jlreq_proto

let envelope ?(protocol = Protocol.protocol) ?(spec = Protocol.spec)
    ?(id = "quick-start/two-lines") ?(extra = "") () =
  Printf.sprintf "{\"protocol\":%S,\"spec\":%S,\"id\":%S,\"request\":{\"source\":\"\"}%s}"
    protocol spec id extra

let run () =
  (* The shape the runner sends. *)
  let request = Protocol.request_of_json (Json.parse (envelope ())) in
  Check.equal_string "the id is carried through" ~expected:"quick-start/two-lines"
    ~actual:request.Protocol.id;
  Check.ok "the request body is the request object"
    (Json.member "source" request.Protocol.body = Some (Json.String ""));

  (* Versioning. A mismatch is an input error, never a difference. *)
  Check.raises "another protocol" (fun () ->
      Protocol.request_of_json (Json.parse (envelope ~protocol:"jlreq.conformance/2" ())));
  Check.raises "the old protocol name" (fun () ->
      Protocol.request_of_json (Json.parse (envelope ~protocol:"kumihan.conformance/1" ())));
  Check.raises "another specification revision" (fun () ->
      Protocol.request_of_json
        (Json.parse (envelope ~spec:"jlreq-2020-08-11+unicode-16.0.0" ())));
  Check.equal_string "the protocol identifier" ~expected:"jlreq.conformance/1"
    ~actual:Protocol.protocol;
  Check.equal_string "the specification identifier"
    ~expected:"jlreq-2020-08-11+unicode-17.0.0" ~actual:Protocol.spec;

  (* The envelope is closed. *)
  Check.raises "an empty id" (fun () ->
      Protocol.request_of_json (Json.parse (envelope ~id:"" ())));
  Check.raises "no request" (fun () ->
      Protocol.request_of_json
        (Json.parse
           (Printf.sprintf "{\"protocol\":%S,\"spec\":%S,\"id\":\"x\"}" Protocol.protocol
              Protocol.spec)));
  Check.raises "no id" (fun () ->
      Protocol.request_of_json
        (Json.parse
           (Printf.sprintf "{\"protocol\":%S,\"spec\":%S,\"request\":{}}" Protocol.protocol
              Protocol.spec)));
  Check.raises "no protocol" (fun () ->
      Protocol.request_of_json
        (Json.parse (Printf.sprintf "{\"spec\":%S,\"id\":\"x\",\"request\":{}}" Protocol.spec)));
  Check.raises "a request that is not an object" (fun () ->
      Protocol.request_of_json
        (Json.parse
           (Printf.sprintf "{\"protocol\":%S,\"spec\":%S,\"id\":\"x\",\"request\":[]}"
              Protocol.protocol Protocol.spec)));
  Check.raises "an unknown field" (fun () ->
      Protocol.request_of_json (Json.parse (envelope ~extra:",\"lenient\":true" ())));
  Check.raises "the expected answer, which the runner strips" (fun () ->
      Protocol.request_of_json
        (Json.parse (envelope ~extra:",\"expected\":{\"lines\":[],\"diagnostics\":[]}" ())));
  Check.raises "a message that is not an object" (fun () ->
      Protocol.request_of_json (Json.parse "[]"));

  (* Blank lines are ignored and do not consume an answer. *)
  Check.ok "an empty line" (Protocol.request_of_line "" = None);
  Check.ok "a whitespace line" (Protocol.request_of_line "   \t " = None);
  Check.ok "a line with only a carriage return" (Protocol.request_of_line "\r" = None);
  Check.ok "a real line is a request"
    (match Protocol.request_of_line (envelope ()) with Some _ -> true | None -> false);
  Check.raises "a line that is not JSON" (fun () -> Protocol.request_of_line "{");

  (* The answer. *)
  Check.equal_string "the empty response" ~expected:"{\"lines\":[],\"diagnostics\":[]}"
    ~actual:(Json.to_string Protocol.empty_response);
  Check.equal_string "a response envelope"
    ~expected:
      "{\"protocol\":\"jlreq.conformance/1\",\"spec\":\"jlreq-2020-08-11+unicode-17.0.0\",\"id\":\"quick-start/two-lines\",\"response\":{\"lines\":[],\"diagnostics\":[]}}"
    ~actual:
      (Json.to_string
         (Protocol.envelope_of_response ~id:"quick-start/two-lines"
            ~response:Protocol.empty_response));
  Check.ok "an id with a solidus is not escaped away"
    (match
       Json.member "id"
         (Json.parse
            (Json.to_string
               (Protocol.envelope_of_response ~id:"C.2-note-11/quantity-symbol-is-unbreakable"
                  ~response:Protocol.empty_response)))
     with
    | Some (Json.String "C.2-note-11/quantity-symbol-is-unbreakable") -> true
    | _ -> false)
