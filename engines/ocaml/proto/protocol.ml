(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** The conformance protocol: envelope, request, response.

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
    the engine to grade itself.

    {1 Closed at both ends}

    The schema is [additionalProperties: false] everywhere, and so is this reader.
    A field the engine does not understand may be the one carrying the meaning of
    the request, and answering it as though the field were absent produces a
    plausible wrong answer instead of an error. Every object below lists the names
    it accepts and refuses everything else.

    A parsed request is a complete one. Milestones that have not arrived yet change
    what the pipeline {i does} with a construct, never whether this layer reads it:
    an engine that cannot yet set ruby still has to know where the ruby was. *)

open Jlreq

(** The wire protocol this engine implements. *)
let protocol = "jlreq.conformance/1"

(** The revision of JLReq, with the Unicode version its derived tables were built
    from. This is a specification identifier and not a version of this engine. *)
let spec = "jlreq-2020-08-11+unicode-17.0.0"

exception Invalid of string
(** Raised on an envelope or a request this engine will not answer. *)

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
    and nothing else. *)
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

(* ----------------------------------------------------------------------------- *)
(* Reading a request *)
(* ----------------------------------------------------------------------------- *)

(** Refuse any field of [value] this engine does not read. *)
let closed (what : string) (value : Json.t) (accepted : string list) : unit =
  List.iter
    (fun name ->
      if not (List.mem name accepted) then
        fail "a %s states `%s`, which protocol v1 does not carry" what name)
    (Json.names value)

let object_field (what : string) (value : Json.t) (name : string) : Json.t option =
  match Json.member name value with
  | Some Json.Null -> fail "%s's %s is null" what name
  | found -> found

let required (what : string) (value : Json.t) (name : string) : Json.t =
  match object_field what value name with
  | Some found -> found
  | None -> fail "%s does not state %s" what name

(** A protocol number as a native [int], inside the [i32] range the schema states. *)
let integer (what : string) (value : Json.t) : int =
  match value with
  | Json.Int number ->
    let native = Num.clamp_i32 number in
    if not (Int64.equal (Int64.of_int native) number) then
      fail "%s is %Ld, outside the protocol's 32 bit range" what number;
    native
  | _ -> fail "%s is not an integer" what

let integer_field (what : string) (value : Json.t) (name : string) : int =
  integer (Printf.sprintf "%s's %s" what name) (required what value name)

let string_field (what : string) (value : Json.t) (name : string) : string =
  match required what value name with
  | Json.String found -> found
  | _ -> fail "%s's %s is not a string" what name

let array_field (what : string) (value : Json.t) (name : string) : Json.t list =
  match required what value name with
  | Json.Array items -> items
  | _ -> fail "%s's %s is not an array" what name

let optional_array (what : string) (value : Json.t) (name : string) : Json.t list =
  match object_field what value name with
  | None -> []
  | Some (Json.Array items) -> items
  | Some _ -> fail "%s's %s is not an array" what name

let non_negative (what : string) (value : Json.t) : int =
  let number = integer what value in
  if number < 0 then fail "%s is %d, and may not be negative" what number;
  number

let range_of_json (what : string) (value : Json.t) : int * int =
  match value with
  | Json.Array [ first; last ] ->
    (non_negative (what ^ "'s start") first, non_negative (what ^ "'s end") last)
  | Json.Array _ -> fail "%s is not a pair of offsets" what
  | _ -> fail "%s is not a range" what

let range_field (what : string) (value : Json.t) (name : string) : int * int =
  range_of_json (Printf.sprintf "%s's %s" what name) (required what value name)

let one_scalar (what : string) (text : string) : int =
  match Utf8.single_scalar text with
  | Some scalar -> scalar
  | None -> fail "%s is not one Unicode scalar" what

let size_of_json (what : string) (value : Json.t) : Model.size =
  closed (what ^ " size") value [ "inline"; "block" ];
  {
    Model.inline = integer_field what value "inline";
    Model.block = integer_field what value "block";
  }

let frame_of_string (what : string) = function
  | "full-em" -> Model.Full_em
  | "half-em" -> Model.Half_em
  | "proportional" -> Model.Proportional
  | other -> fail "%s names the frame `%s`" what other

let writing_mode_of_string (what : string) = function
  | "horizontal-tb" -> Model.Horizontal_tb
  | "vertical-rl" -> Model.Vertical_rl
  | other -> fail "%s names the writing mode `%s`" what other

let role_of_string (what : string) = function
  | "text" -> Model.Text
  | "decimal-point" -> Model.Decimal_point
  | "digit-group-separator" -> Model.Digit_group_separator
  | "sentence-medial" -> Model.Sentence_medial
  | "sentence-terminator" -> Model.Sentence_terminator
  | "grouped-numeral" -> Model.Grouped_numeral
  | "unit-symbol" -> Model.Unit_symbol
  | "quantity-symbol" -> Model.Quantity_symbol
  | "formula" -> Model.Formula
  | "warichu-bracket" -> Model.Warichu_bracket
  | other -> fail "%s names the role `%s`" what other

let cluster_of_json (value : Json.t) : Model.cluster =
  closed "cluster" value [ "range"; "advance"; "size"; "frame"; "role" ];
  let first, last = range_field "cluster" value "range" in
  {
    Model.first;
    Model.last;
    Model.advance = non_negative "a cluster's advance" (required "cluster" value "advance");
    Model.size_override =
      (match object_field "cluster" value "size" with
      | Some size -> Some (size_of_json "a cluster's" size)
      | None -> None);
    Model.frame_override =
      (match object_field "cluster" value "frame" with
      | Some (Json.String frame) -> Some (frame_of_string "a cluster" frame)
      | Some _ -> fail "a cluster's frame is not a string"
      | None -> None);
    Model.role =
      (match object_field "cluster" value "role" with
      | Some (Json.String role) -> Some (role_of_string "a cluster" role)
      | Some _ -> fail "a cluster's role is not a string"
      | None -> None);
  }

(** The [shapedText] object: a request's own text, and a ruby, reference mark or
    script annotation's. *)
let shaped_text_of_json (what : string) (value : Json.t) ~(extra : string list) :
    Model.shaped_text =
  closed what value ([ "source"; "size"; "frame"; "clusters" ] @ extra);
  let text =
    {
      Model.source = string_field what value "source";
      Model.size = size_of_json what (required what value "size");
      Model.frame = frame_of_string what (string_field what value "frame");
      Model.clusters =
        Array.of_list (List.map cluster_of_json (array_field what value "clusters"));
    }
  in
  Normalize.check text;
  text

let ruby_kind_of_string = function
  | "mono" -> Construct.Mono
  | "group" -> Construct.Group
  | "jukugo" -> Construct.Jukugo
  | other -> fail "a ruby states the kind `%s`" other

let construct_of_json (value : Json.t) : Construct.t =
  let kind_name = string_field "construct" value "kind" in
  let range () = range_field "construct" value "range" in
  match kind_name with
  | "ruby" ->
    closed "construct" value [ "kind"; "range"; "ruby_kind"; "annotation"; "runs" ];
    let runs =
      List.map
        (fun run ->
          closed "ruby run" run [ "base"; "annotation" ];
          {
            Construct.run_base = range_field "ruby run" run "base";
            run_annotation = range_field "ruby run" run "annotation";
          })
        (array_field "construct" value "runs")
    in
    {
      Construct.range = range ();
      Construct.kind =
        Construct.Ruby
          {
            ruby_kind = ruby_kind_of_string (string_field "construct" value "ruby_kind");
            annotation =
              shaped_text_of_json "a ruby annotation" (required "construct" value "annotation")
                ~extra:[];
            runs = runs;
          };
    }
  | "tate-chu-yoko" | "warichu" | "formula" ->
    closed "construct" value [ "kind"; "range" ];
    let kind =
      match kind_name with
      | "tate-chu-yoko" -> Construct.Tate_chu_yoko
      | "warichu" -> Construct.Warichu
      | _ -> Construct.Formula
    in
    { Construct.range = range (); Construct.kind }
  | "emphasis-dots" ->
    closed "construct" value [ "kind"; "range"; "mark" ];
    {
      Construct.range = range ();
      Construct.kind =
        Construct.Emphasis_dots
          { mark = one_scalar "an emphasis mark" (string_field "construct" value "mark") };
    }
  | "furawake" ->
    closed "construct" value [ "kind"; "range"; "columns"; "line_gap" ];
    {
      Construct.range = range ();
      Construct.kind =
        Construct.Furawake
          {
            columns = integer_field "construct" value "columns";
            line_gap = non_negative "a furawake's line gap"
                (required "construct" value "line_gap");
          };
    }
  | "jidori" ->
    closed "construct" value [ "kind"; "range"; "cells" ];
    {
      Construct.range = range ();
      Construct.kind = Construct.Jidori { cells = integer_field "construct" value "cells" };
    }
  | "reference-mark" | "script" ->
    closed "construct" value [ "kind"; "range"; "annotation" ];
    let annotation =
      shaped_text_of_json "a construct annotation" (required "construct" value "annotation")
        ~extra:[]
    in
    let kind =
      if String.equal kind_name "reference-mark" then
        Construct.Reference_mark { annotation }
      else Construct.Script { annotation }
    in
    { Construct.range = range (); Construct.kind }
  | other -> fail "a construct states the kind `%s`" other

let break_of_json (value : Json.t) : Paragraph.break_opportunity =
  closed "break" value [ "offset"; "kind" ];
  {
    Paragraph.offset = non_negative "a break's offset" (required "break" value "offset");
    Paragraph.kind =
      (match string_field "break" value "kind" with
      | "allowed" -> Paragraph.Allowed
      | "mandatory" -> Paragraph.Mandatory
      | "discretionary" -> Paragraph.Discretionary
      | other -> fail "a break states the kind `%s`" other);
  }

let tab_stop_of_json (value : Json.t) : Paragraph.tab_stop =
  closed "tab stop" value [ "position"; "alignment"; "character" ];
  let alignment_name = string_field "tab stop" value "alignment" in
  let character = object_field "tab stop" value "character" in
  let tab_alignment =
    match (alignment_name, character) with
    | "character", Some (Json.String text) ->
      Paragraph.Tab_character (one_scalar "a tab stop's character" text)
    | "character", _ -> fail "a character tab stop does not state its character"
    | _, Some _ -> fail "a `%s` tab stop states a character" alignment_name
    | "start", None -> Paragraph.Tab_start
    | "center", None -> Paragraph.Tab_center
    | "end", None -> Paragraph.Tab_end
    | other, None -> fail "a tab stop states the alignment `%s`" other
  in
  { Paragraph.position = integer_field "tab stop" value "position"; Paragraph.tab_alignment }

(** The [style] field: a profile name, or a profile with overrides. *)
let style_of_json (value : Json.t) : Style.t =
  match value with
  | Json.String profile -> Style.build ~profile []
  | Json.Object fields ->
    let profile =
      match List.assoc_opt "profile" fields with
      | Some (Json.String profile) -> profile
      | Some _ -> fail "the style's profile is not a string"
      | None -> "jlreq-2020"
    in
    let overrides =
      List.filter_map
        (fun (name, setting) ->
          if String.equal name "profile" then None
          else
            match setting with
            | Json.String answer -> Some (name, answer)
            | _ -> fail "the style setting `%s` is not a string" name)
        fields
    in
    Style.build ~profile overrides
  | _ -> fail "style is neither a profile name nor a set of settings"

(** One [request] object as a paragraph and the style to set it in. *)
let paragraph_of_json (body : Json.t) : Paragraph.t * Style.t =
  closed "request" body
    [
      "source"; "size"; "frame"; "clusters"; "line_extent"; "breaks"; "constructs"; "tab_stops";
      "first_line_indent"; "alignment"; "widow_minimum_clusters"; "writing_mode"; "style";
    ];
  let text = shaped_text_of_json "request" body
      ~extra:
        [
          "line_extent"; "breaks"; "constructs"; "tab_stops"; "first_line_indent"; "alignment";
          "widow_minimum_clusters"; "writing_mode"; "style";
        ]
  in
  (* A request that states no alignment is asking for ordinary Japanese setting,
     which §3.8.1 describes as adjusting every line but a short last one to the
     measure. `start` is one of §3.5.3's four answers, and a caller who wants it
     says so; the absence of the field is not that answer. *)
  let alignment =
    match object_field "request" body "alignment" with
    | None -> Paragraph.Justify
    | Some (Json.String "start") -> Paragraph.Start
    | Some (Json.String "center") -> Paragraph.Center
    | Some (Json.String "end") -> Paragraph.End
    | Some (Json.String "justify") -> Paragraph.Justify
    | Some (Json.String other) -> fail "the request states the alignment `%s`" other
    | Some _ -> fail "the request's alignment is not a string"
  in
  let widow =
    match object_field "request" body "widow_minimum_clusters" with
    | None -> Paragraph.No_widow
    | Some value ->
      let minimum = integer "the widow minimum" value in
      if minimum < 1 || minimum > 65535 then
        fail "the widow minimum is %d clusters" minimum;
      Paragraph.Minimum_clusters minimum
  in
  let writing_mode =
    match object_field "request" body "writing_mode" with
    | None -> Model.Horizontal_tb
    | Some (Json.String mode) -> writing_mode_of_string "the request" mode
    | Some _ -> fail "the request's writing mode is not a string"
  in
  let style =
    match object_field "request" body "style" with
    | None -> Style.default ()
    | Some value -> style_of_json value
  in
  let paragraph =
    Paragraph.build ~text
      ~line_extent:(integer_field "request" body "line_extent")
      ~breaks:(List.map break_of_json (optional_array "request" body "breaks"))
      ~constructs:(List.map construct_of_json (optional_array "request" body "constructs"))
      ~tab_stops:(List.map tab_stop_of_json (optional_array "request" body "tab_stops"))
      ~first_line_indent:
        (match object_field "request" body "first_line_indent" with
        | None -> 0
        | Some value -> integer "the first line indent" value)
      ~alignment ~widow ~writing_mode ()
  in
  (paragraph, style)

(* ----------------------------------------------------------------------------- *)
(* Writing a response *)
(* ----------------------------------------------------------------------------- *)

let json_of_range ((first, last) : int * int) : Json.t =
  Json.Array [ Json.of_int first; Json.of_int last ]

let json_of_size (size : Model.size) : Json.t =
  Json.Object
    [ ("inline", Json.of_int size.Model.inline); ("block", Json.of_int size.Model.block) ]

let json_of_placement (placement : Layout.placement) : Json.t =
  Json.Object
    [
      ( "origin",
        match placement.Layout.origin with
        | Layout.From_cluster ordinal -> Json.Object [ ("cluster", Json.of_int ordinal) ]
        | Layout.From_construct ordinal -> Json.Object [ ("construct", Json.of_int ordinal) ] );
      ("range", json_of_range placement.Layout.range);
      ("inline", Json.of_int placement.Layout.inline);
      ("block", Json.of_int placement.Layout.block);
      ("advance", Json.of_int placement.Layout.advance);
      ("size", json_of_size placement.Layout.size);
      ("frame", Json.String (Model.frame_name placement.Layout.frame));
      ("writing_mode", Json.String (Model.writing_mode_name placement.Layout.writing_mode));
      ("transform", Json.String (Layout.transform_name placement.Layout.transform));
    ]

let json_of_attachment (attachment : Layout.attachment) : Json.t =
  Json.Object
    [
      ("construct", Json.of_int attachment.Layout.attachment_construct);
      ("range", json_of_range attachment.Layout.attachment_range);
      ("inline", Json.of_int attachment.Layout.attachment_inline);
      ("block", Json.of_int attachment.Layout.attachment_block);
      ("advance", Json.of_int attachment.Layout.attachment_advance);
      ("size", json_of_size attachment.Layout.attachment_size);
      ( "writing_mode",
        Json.String (Model.writing_mode_name attachment.Layout.attachment_writing_mode) );
      ("transform", Json.String (Layout.transform_name attachment.Layout.attachment_transform));
      ( "symbol",
        match attachment.Layout.attachment_symbol with
        | Some scalar -> Json.String (Utf8.of_scalar scalar)
        | None -> Json.Null );
    ]

let json_of_line (line : Layout.line) : Json.t =
  Json.Object
    [
      ("range", json_of_range line.Layout.line_range);
      ("inline_origin", Json.of_int line.Layout.inline_origin);
      ("block_origin", Json.of_int line.Layout.block_origin);
      ("inline_extent", Json.of_int line.Layout.inline_extent);
      ("block_extent", Json.of_int line.Layout.block_extent);
      ("clusters", Json.Array (List.map json_of_placement line.Layout.clusters));
      ("attachments", Json.Array (List.map json_of_attachment line.Layout.attachments));
    ]

let json_of_diagnostic (diagnostic : Layout.diagnostic) : Json.t =
  Json.Object
    [
      ("code", Json.String diagnostic.Layout.code);
      ("severity", Json.String (Layout.severity_name diagnostic.Layout.severity));
      ( "range",
        match diagnostic.Layout.diagnostic_range with
        | Some range -> json_of_range range
        | None -> Json.Null );
      ("jlreq", Json.String diagnostic.Layout.jlreq);
    ]

(** A layout result as the [response] object.

    Order is fixed rather than incidental: the runner compares parsed values, so
    field order cannot change an answer, but a stable order makes a captured
    stdout diffable by eye during development. *)
let response ~(lines : Json.t list) ~(diagnostics : Json.t list) : Json.t =
  Json.Object [ ("lines", Json.Array lines); ("diagnostics", Json.Array diagnostics) ]

let json_of_layout (layout : Layout.t) : Json.t =
  response
    ~lines:(List.map json_of_line layout.Layout.lines)
    ~diagnostics:(List.map json_of_diagnostic layout.Layout.diagnostics)

(** A paragraph that produced no lines and no diagnostics. *)
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

(** Read one request, compose it, and answer it. *)
let answer (request : request) : Json.t =
  let paragraph, style = paragraph_of_json request.body in
  envelope_of_response ~id:request.id ~response:(json_of_layout (Pipeline.compose paragraph style))
