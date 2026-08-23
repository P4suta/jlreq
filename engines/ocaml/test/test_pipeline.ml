(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** {!Jlreq.Pipeline}: the cost function's boundaries, the validation boundary,
    and one paragraph composed end to end.

    The cost constants are checked here because they are the part of the engine
    with no external source: they are not in JLReq, not in [spec/], and not in any
    design note. A conformance case that breaks in the wrong place would say only
    that the paragraph came out wrong; these say which constant moved. *)

open Jlreq

let em = { Model.inline = 1000; Model.block = 1000 }

let cluster ?size ?frame ?role first last advance =
  {
    Model.first;
    Model.last;
    Model.advance;
    Model.size_override = size;
    Model.frame_override = frame;
    Model.role;
  }

let text ?(size = em) ?(frame = Model.Full_em) source clusters =
  { Model.source; Model.size; Model.frame; Model.clusters = Array.of_list clusters }

let compose_json (envelope : string) : string =
  match Jlreq_proto.Protocol.request_of_line envelope with
  | Some request -> Jlreq_proto.Json.to_string (Jlreq_proto.Protocol.answer request)
  | None -> "the line was blank"

let envelope (id : string) (request : string) : string =
  Printf.sprintf "{\"protocol\":%S,\"spec\":%S,\"id\":%S,\"request\":%s}"
    Jlreq_proto.Protocol.protocol Jlreq_proto.Protocol.spec id request

let run () =
  (* The badness of one line, by leftover measure. *)
  let badness ?(is_last = false) ?(preference = "least-adjustment") delta =
    Pipeline.line_badness (Int64.of_int delta) is_last preference
  in
  Check.equal_int64 "a line that fits exactly costs nothing" ~expected:0L ~actual:(badness 0);
  Check.equal_int64 "a line short by one" ~expected:1L ~actual:(badness 1);
  Check.equal_int64 "a line short by a thousand" ~expected:1_000_000L ~actual:(badness 1000);
  Check.equal_int64 "even texture doubles it" ~expected:2_000_000L
    ~actual:(badness ~preference:"even-texture" 1000);
  Check.equal_int64 "the last line is charged a hundredth" ~expected:10_000L
    ~actual:(badness ~is_last:true 1000);
  Check.equal_int64 "and a last line short by nine is charged nothing" ~expected:0L
    ~actual:(badness ~is_last:true 9);
  Check.equal_int64 "an overrun of one" ~expected:10_001_000L ~actual:(badness (-1));
  Check.equal_int64 "an overrun of a thousand" ~expected:1_010_000_000L
    ~actual:(badness (-1000));
  Check.ok "an overrun costs a thousand times the shortfall it mirrors"
    (Int64.equal (badness (-1000))
       (Num.sadd (Num.smul (badness 1000) 1_000L) 10_000_000L));
  Check.ok "and the surcharge puts the smallest overrun above a wide shortfall"
    (Int64.compare (badness (-1)) (badness 3000) > 0);
  (* The magnitude is capped, so two impossibly wide lines still compare equal
     rather than overflowing into each other. *)
  Check.equal_int64 "the magnitude caps at a million"
    ~expected:(badness 1_000_000) ~actual:(badness 2_000_000);
  Check.equal_int64 "an overrun caps the same way"
    ~expected:(badness (-1_000_000)) ~actual:(badness (-9_000_000));
  Check.equal_int64 "the cap squared" ~expected:1_000_000_000_000L ~actual:(badness 1_000_000);
  (* The unreachable cost leaves room for several of itself to be added up. *)
  Check.ok "the infinite cost is a quarter of the range"
    (Int64.equal Num.infinite_cost (Int64.div Int64.max_int 4L));
  Check.ok "and four of them do not saturate"
    (Int64.compare (Num.sadd (Num.sadd Num.infinite_cost Num.infinite_cost)
                      (Num.sadd Num.infinite_cost Num.infinite_cost))
       Int64.max_int
    < 0);

  (* The validation boundary. *)
  let two = text "\xe6\x97\xa5\xe6\x9c\xac" [ cluster 0 3 1000; cluster 3 6 1000 ] in
  Check.returns "a well formed paragraph" (fun () ->
      Paragraph.build ~text:two ~line_extent:1000 ());
  Check.raises "clusters that leave a byte uncovered" (fun () ->
      Paragraph.build ~text:(text "\xe6\x97\xa5\xe6\x9c\xac" [ cluster 0 3 1000 ])
        ~line_extent:1000 ());
  Check.raises "a cluster boundary inside a scalar" (fun () ->
      Paragraph.build
        ~text:(text "\xe6\x97\xa5\xe6\x9c\xac" [ cluster 0 2 1000; cluster 2 6 1000 ])
        ~line_extent:1000 ());
  Check.raises "a negative advance" (fun () ->
      Paragraph.build ~text:(text "A" [ cluster 0 1 (-1) ]) ~line_extent:1000 ());
  Check.raises "a size of nothing" (fun () ->
      Paragraph.build ~text:(text ~size:{ Model.inline = 0; Model.block = 1000 } "A"
                               [ cluster 0 1 1000 ])
        ~line_extent:1000 ());
  Check.raises "a measure of nothing" (fun () -> Paragraph.build ~text:two ~line_extent:0 ());
  Check.raises "a break inside a scalar" (fun () ->
      Paragraph.build ~text:two ~line_extent:1000
        ~breaks:[ { Paragraph.offset = 1; Paragraph.kind = Paragraph.Allowed } ]
        ());
  Check.raises "two breaks at one offset" (fun () ->
      Paragraph.build ~text:two ~line_extent:1000
        ~breaks:
          [
            { Paragraph.offset = 3; Paragraph.kind = Paragraph.Allowed };
            { Paragraph.offset = 3; Paragraph.kind = Paragraph.Mandatory };
          ]
        ());
  Check.raises "a full-em cluster holding two ideographs" (fun () ->
      Paragraph.build ~text:(text "\xe6\x97\xa5\xe6\x9c\xac" [ cluster 0 6 1000 ])
        ~line_extent:1000 ());
  Check.returns "a proportional cluster holding a ligature" (fun () ->
      Paragraph.build
        ~text:(text ~frame:Model.Proportional "fi" [ cluster 0 2 800 ])
        ~line_extent:1000 ());

  (* The end of the source is a break whether or not the caller says so. *)
  let paragraph =
    Paragraph.build ~text:two ~line_extent:1000
      ~breaks:[ { Paragraph.offset = 3; Paragraph.kind = Paragraph.Allowed } ]
      ()
  in
  Check.equal_int "the terminal break is added" ~expected:2
    ~actual:(Array.length paragraph.Paragraph.breaks);
  Check.ok "and it is mandatory"
    (Paragraph.is_mandatory paragraph.Paragraph.breaks.(1));

  (* Composed: two full-em clusters, a one-em measure, one break between them. *)
  let layout = Pipeline.compose paragraph (Style.default ()) in
  Check.equal_int "two lines" ~expected:2 ~actual:(List.length layout.Layout.lines);
  Check.equal_int "no diagnostics" ~expected:0 ~actual:(List.length layout.Layout.diagnostics);
  (match layout.Layout.lines with
  | [ first; second ] ->
    Check.equal_int "the first line starts at the origin" ~expected:0
      ~actual:first.Layout.inline_origin;
    Check.equal_int "and is one em wide" ~expected:1000 ~actual:first.Layout.inline_extent;
    Check.equal_int "the second line is one em down" ~expected:1000
      ~actual:second.Layout.block_origin;
    Check.equal_int "and holds one placement" ~expected:1
      ~actual:(List.length second.Layout.clusters)
  | _ -> Check.ok "two lines" false);

  (* The same paragraph over the wire, byte for byte. *)
  Check.equal_string "quick-start/two-lines"
    ~expected:
      "{\"protocol\":\"jlreq.conformance/1\",\"spec\":\"jlreq-2020-08-11+unicode-17.0.0\",\"id\":\"quick-start/two-lines\",\"response\":{\"lines\":[{\"range\":[0,3],\"inline_origin\":0,\"block_origin\":0,\"inline_extent\":1000,\"block_extent\":1000,\"clusters\":[{\"origin\":{\"cluster\":0},\"range\":[0,3],\"inline\":0,\"block\":0,\"advance\":1000,\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"writing_mode\":\"horizontal-tb\",\"transform\":\"identity\"}],\"attachments\":[]},{\"range\":[3,6],\"inline_origin\":0,\"block_origin\":1000,\"inline_extent\":1000,\"block_extent\":1000,\"clusters\":[{\"origin\":{\"cluster\":1},\"range\":[3,6],\"inline\":0,\"block\":1000,\"advance\":1000,\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"writing_mode\":\"horizontal-tb\",\"transform\":\"identity\"}],\"attachments\":[]}],\"diagnostics\":[]}}"
    ~actual:
      (compose_json
         (envelope "quick-start/two-lines"
            "{\"source\":\"\xe6\x97\xa5\xe6\x9c\xac\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,3],\"advance\":1000},{\"range\":[3,6],\"advance\":1000}],\"line_extent\":1000,\"breaks\":[{\"offset\":3,\"kind\":\"allowed\"}],\"writing_mode\":\"horizontal-tb\",\"style\":\"jlreq-2020\"}"));

  (* End alignment moves the origin and leaves the extent alone (§3.5.3). *)
  Check.equal_string "3.5.3/end-alignment-shifts-origin-without-changing-extent"
    ~expected:
      "{\"protocol\":\"jlreq.conformance/1\",\"spec\":\"jlreq-2020-08-11+unicode-17.0.0\",\"id\":\"end\",\"response\":{\"lines\":[{\"range\":[0,6],\"inline_origin\":3000,\"block_origin\":0,\"inline_extent\":2000,\"block_extent\":1000,\"clusters\":[{\"origin\":{\"cluster\":0},\"range\":[0,3],\"inline\":3000,\"block\":0,\"advance\":1000,\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"writing_mode\":\"horizontal-tb\",\"transform\":\"identity\"},{\"origin\":{\"cluster\":1},\"range\":[3,6],\"inline\":4000,\"block\":0,\"advance\":1000,\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"writing_mode\":\"horizontal-tb\",\"transform\":\"identity\"}],\"attachments\":[]}],\"diagnostics\":[]}}"
    ~actual:
      (compose_json
         (envelope "end"
            "{\"source\":\"\xe6\x97\xa5\xe6\x9c\xac\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,3],\"advance\":1000},{\"range\":[3,6],\"advance\":1000}],\"line_extent\":5000,\"alignment\":\"end\",\"style\":\"jlreq-2020\"}"));

  (* A request the protocol does not carry is an error, not a default. *)
  Check.raises "an unknown request field" (fun () ->
      compose_json
        (envelope "x"
           "{\"source\":\"A\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,1],\"advance\":1000}],\"line_extent\":1000,\"leading\":1000}"));
  Check.raises "an unknown cluster role" (fun () ->
      compose_json
        (envelope "x"
           "{\"source\":\"A\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,1],\"advance\":1000,\"role\":\"headword\"}],\"line_extent\":1000}"));
  Check.raises "an unknown style setting" (fun () ->
      compose_json
        (envelope "x"
           "{\"source\":\"A\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,1],\"advance\":1000}],\"line_extent\":1000,\"style\":{\"profile\":\"jlreq-2020\",\"spacing.leading\":\"loose\"}}"));
  Check.returns "and a request with every optional field left out" (fun () ->
      compose_json
        (envelope "x"
           "{\"source\":\"A\",\"size\":{\"inline\":1000,\"block\":1000},\"frame\":\"full-em\",\"clusters\":[{\"range\":[0,1],\"advance\":1000}],\"line_extent\":1000}"))
