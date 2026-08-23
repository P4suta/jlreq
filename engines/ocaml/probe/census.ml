(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** Synthetic request suites that isolate one mechanism at a time.

    The built-in conformance suite has eighty-nine cases and each of them exercises
    a dozen mechanisms at once: a case that disagrees says the paragraph came out
    wrong, not which of classification, spacing, breakability, reduction and
    geometry is the one that is wrong. A census inverts that. It walks the
    character classes pairwise and emits one minimal request per pair, so a
    disagreement names a coordinate of a published matrix -- [cl-01+cl-02] -- and
    the fix is a cell rather than a paragraph.

    {v
    just census spacing        # 2116 requests, both engines, one diff
    just census tate-chu-yoko
    just census-classes        # the representative code point chosen for each class
    v}

    The suites are development assets and are never added to [suite.ndjson]: the
    built-in suite is a curated statement about the specification, and five hundred
    machine-generated pairs are a debugging instrument. Nothing here is committed
    either; the generated NDJSON lands under [target/], which is ignored.

    {1 What a pair request looks like}

    Every cluster is one em wide in a one-em full-em frame, so the only thing that
    varies between two requests of the same census is which characters are in them.
    A spacing request gives the line more room than the paragraph can use, so no
    break and no adjustment can happen and the inline positions are the spacing
    matrix read back out. A break request gives the line room for exactly one
    cluster and marks every internal boundary [allowed], so the answer is one line
    or two and that is Table 2 read back out.

    Twenty-three of the thirty classes get a representative. cl-17 and cl-18 are
    left out because no matrix axis carries them, and cl-20 through cl-23 and cl-30
    because Appendix A lists no code point in them: they are what a character
    becomes inside a construct, and the milestones that build those constructs are
    where their census belongs. cl-30 has one now -- a character standing inside a
    tate-chu-yoko run is cl-30 whatever it is, so the {!tate_chu_yoko_census} reaches
    that row and column by building the run rather than by naming a key.

    {1 Adding a census}

    {!kinds} is the whole registry: a census is a name, a sentence, and a function
    that emits requests. Each one is a single entry -- the reduction and expansion
    censuses walk the same pairs at a line extent that forces the paragraph to give
    back or take up space, and the vertical and tate-chu-yoko censuses walk them in
    the other writing mode and beside a construct -- and no other part of this file
    has to know.

    This is a development probe. It is outside the engine, the runner never starts
    it, and it may read files at run time, which the engine itself may not
    (docs/design/conformance.md, "The runner contract"). *)

let program = "census"

exception Fault of string

let fault format = Printf.ksprintf (fun message -> raise (Fault message)) format

(* ----------------------------------------------------------------------------- *)
(* Representative code points *)
(* ----------------------------------------------------------------------------- *)

(** How many of Appendix A's classes list a key.

    A key listed by one class classifies to that class whatever the context; a key
    listed by several is resolved by [classification.ambiguous_context] and by the
    frame it arrives in, which is a mechanism of its own and not the one a spacing
    census is trying to measure. The count is the first thing a representative is
    chosen by, so the census asks the least ambiguous question it can. *)
let classes_listing : (string, int) Hashtbl.t =
  let seen = Hashtbl.create 4096 and count = Hashtbl.create 2048 in
  List.iter
    (fun (row : Jlreq.Tables.listing) ->
      let pair = Printf.sprintf "%d\t%s" row.Jlreq.Tables.listing_class row.Jlreq.Tables.listing_key_text in
      if not (Hashtbl.mem seen pair) then begin
        Hashtbl.add seen pair ();
        let key = row.Jlreq.Tables.listing_key_text in
        Hashtbl.replace count key (1 + Option.value ~default:0 (Hashtbl.find_opt count key))
      end)
    Jlreq.Tables.appendix_a;
  count

type representative = {
  rep_class : Jlreq.Tables.klass;
  rep_label : string;  (** [cl-01]. *)
  rep_key : string;  (** The key as Appendix A spells it, e.g. [3008]. *)
  rep_text : string;  (** The key as UTF-8. *)
  rep_ambiguity : int;  (** How many classes list {!rep_key}. *)
}

(** The representative of one class, or [None] where Appendix A lists nothing.

    Five classes list nothing: cl-20 through cl-23 and cl-30 are the classes a
    character acquires by standing inside a construct -- a reference mark, an
    ornamented complex, a ruby complex, a tate-chu-yoko run -- and no code point
    is in them on its own. They have no pair census until the milestones that
    build the constructs that create them.

    Among the rows a class does list, the order of preference is: fewest classes
    listing the key, then a single-scalar key over a sequence, then an empty
    Remarks cell over a qualified one, then Appendix A's own document order. Each
    step removes a way for the census to be measuring something other than the
    adjacency of the two classes it names. The Remarks cell is read in Japanese
    because that is the transcription this engine reads throughout. *)
let representative_of_class (value : Jlreq.Tables.klass) : representative option =
  let score index (row : Jlreq.Tables.listing) =
    let key = row.Jlreq.Tables.listing_key_text in
    ( Option.value ~default:1 (Hashtbl.find_opt classes_listing key),
      Array.length row.Jlreq.Tables.listing_key,
      (if String.equal row.Jlreq.Tables.remark_ja "" then 0 else 1),
      index )
  in
  let best = ref None in
  List.iteri
    (fun index (row : Jlreq.Tables.listing) ->
      if row.Jlreq.Tables.listing_class = value then begin
        let candidate = score index row in
        match !best with
        | Some (chosen, _) when compare candidate chosen >= 0 -> ()
        | _ -> best := Some (candidate, row)
      end)
    Jlreq.Tables.appendix_a;
  Option.map
    (fun ((ambiguity, _, _, _), (row : Jlreq.Tables.listing)) ->
      let buffer = Buffer.create 8 in
      Array.iter (fun code -> Jlreq.Utf8.encode buffer code) row.Jlreq.Tables.listing_key;
      {
        rep_class = value;
        rep_label = Jlreq.Tables.row_label value;
        rep_key = row.Jlreq.Tables.listing_key_text;
        rep_text = Buffer.contents buffer;
        rep_ambiguity = ambiguity;
      })
    !best

(** One representative per class the census can address, in class order.

    cl-17 and cl-18 are left out: §3.9.2 lists them, but no matrix axis carries
    them, so an adjacency census has no cell to compare a pair against. *)
let representatives : representative list =
  let out = ref [] in
  for value = 30 downto 1 do
    if Jlreq.Tables.has_adjacency value then
      match representative_of_class value with Some row -> out := row :: !out | None -> ()
  done;
  !out

(** The neutral cluster a variant pads with.

    cl-19's representative, so the padding is an ideograph -- the class ordinary
    running text is mostly made of. Table 1 states [(line-head, cl-19)],
    [(cl-19, line-end)] and [(cl-19, cl-19)] blank, and Table 2 states
    [(cl-19, cl-19)] blank, so a padded variant adds no spacing of its own at
    either line edge and prohibits no break: what changes between [pair] and
    [interior] is the pair's distance from the edges and nothing else. *)
let filler : string =
  match representative_of_class 19 with
  | Some row -> row.rep_text
  | None -> fault "Appendix A lists no cl-19 key to pad a census request with"

(* ----------------------------------------------------------------------------- *)
(* Requests *)
(* ----------------------------------------------------------------------------- *)

(** The em every census cluster is, in the caller's unit.

    One thousand rather than seven hundred and twenty: the unit the protocol
    carries is the caller's and has nothing to do with the 1/720 em the matrices
    are transcribed in, and a round number makes a wrong inline position legible at
    a glance. *)
let em = 1000

(** A line no spacing census can fill.

    Four clusters is the widest variant and there are five places spacing can go --
    the line head, three cluster boundaries, the line end -- none of which any of
    the six matrices puts more than one em at. Nine ems is therefore the very most a
    variant can want, and sixteen is well clear of it: nothing breaks, nothing is
    reduced, nothing is expanded, and the inline positions are the spacing matrix
    and nothing else. *)
let wide_extent = 16 * em

let size =
  Jlreq_proto.Json.Object
    [ ("inline", Jlreq_proto.Json.of_int em); ("block", Jlreq_proto.Json.of_int em) ]

(** §3.3.3: a ruby character is half the size of its base character. Every reading a
    census attaches is at that size, because it is the one size §3.3 states, and a
    variant that wants a second size states it on one cluster rather than on the
    whole reading. *)
let ruby_em = em / 2

let ruby_size =
  Jlreq_proto.Json.Object
    [ ("inline", Jlreq_proto.Json.of_int ruby_em); ("block", Jlreq_proto.Json.of_int ruby_em) ]

(* ----------------------------------------------------------------------------- *)
(* Style answers that do not contradict each other *)
(* ----------------------------------------------------------------------------- *)

(** One of the twenty-two places JLReq permits more than one answer.

    [spec/derived/questions.tsv] carries, beside each question's permitted answers
    and each profile's answer to it, an [excludes] column: the pairs of answers the
    specification refuses to hold at once. There is one such pair today, and it is
    exactly the one a break census walks into. §C.3's fourth level, [very-strict],
    excludes [kinsoku.grouped_numeral_before_western = breakable] and
    [kinsoku.relaxation_mechanism = reclassify], and both of those are what the
    [jlreq-2020] profile answers -- so a request that states nothing but
    [kinsoku.level: very-strict] is a contradiction and an engine is right to refuse
    it rather than answer it.

    Reading the column rather than writing the two settings down here is not
    ceremony: the census that arrives at M2 will state a reduction table and the one
    at M3 an expansion ceiling, and whatever those exclude will already be handled. *)
type question = {
  question_name : string;
  question_permits : string list;
  question_default : string;  (** The [jlreq-2020] profile's answer. *)
  question_excludes : (string * string * string) list;
      (** [(this answer, that question, the answer it excludes)]. *)
}

let split_words (separator : char) (text : string) : string list =
  List.filter (fun piece -> piece <> "") (String.split_on_char separator text)

let questions : question list =
  let file = Jlreq.Tables.questions in
  let name_column = Jlreq.Tsv.column file "question"
  and permits_column = Jlreq.Tsv.column file "permits"
  and default_column = Jlreq.Tsv.column file "jlreq"
  and excludes_column = Jlreq.Tsv.column file "excludes" in
  List.map
    (fun row ->
      {
        question_name = Jlreq.Tsv.field row name_column;
        question_permits = split_words ' ' (Jlreq.Tsv.field row permits_column);
        question_default = Jlreq.Tsv.field row default_column;
        question_excludes =
          List.map
            (fun rule ->
              match String.split_on_char '|' rule with
              | answer :: other :: forbidden :: _ -> (answer, other, forbidden)
              | _ ->
                fault "the Style questions state the exclusion `%s`, which is not `answer|question|answer|rule`"
                  rule)
            (split_words ';' (Jlreq.Tsv.field row excludes_column));
      })
    file.Jlreq.Tsv.rows

let question_named (name : string) : question =
  match List.find_opt (fun entry -> String.equal entry.question_name name) questions with
  | Some entry -> entry
  | None -> fault "`%s` is not one of the Style questions" name

(** What [settings] answers [name] with: the stated answer, or the profile's. *)
let answer_of (settings : (string * string) list) (name : string) : string =
  match List.assoc_opt name settings with
  | Some answer -> answer
  | None -> (question_named name).question_default

(** [stated] with every answer it forces stated explicitly.

    Each round finds a question whose current answer is one an already-answered
    question excludes, and states the other permitted answer instead. Each round
    answers at least one more question than the last, so the loop is bounded by the
    number of questions; a census that states both halves of an excluded pair itself
    is a fault, because there is no answer that would satisfy it. *)
let resolve (stated : (string * string) list) : (string * string) list =
  let rec settle settings fuel =
    if fuel <= 0 then fault "the Style exclusions do not settle";
    let forced =
      List.concat_map
        (fun entry ->
          let answer = answer_of settings entry.question_name in
          List.filter_map
            (fun (when_answer, other, forbidden) ->
              if
                (not (String.equal when_answer answer))
                || not (String.equal (answer_of settings other) forbidden)
              then None
              else if List.mem_assoc other settings then
                fault "a census states %s=%s and %s=%s, which the specification excludes"
                  entry.question_name answer other forbidden
              else
                match
                  List.find_opt
                    (fun permitted -> not (String.equal permitted forbidden))
                    (question_named other).question_permits
                with
                | Some replacement -> Some (other, replacement)
                | None -> fault "%s permits only the answer %s excludes" other entry.question_name)
            entry.question_excludes)
        questions
    in
    if forced = [] then settings else settle (settings @ forced) (fuel - 1)
  in
  settle stated (List.length questions + 1)

(** The style every census states, so that a difference is never a difference about
    which profile the two engines defaulted to. *)
let style (settings : (string * string) list) : Jlreq_proto.Json.t =
  Jlreq_proto.Json.Object
    (("profile", Jlreq_proto.Json.String "jlreq-2020")
    :: List.map (fun (name, value) -> (name, Jlreq_proto.Json.String value)) (resolve settings))

type piece = {
  piece_text : string;
  piece_size : (int * int) option;
      (** An inline and a block em of its own, where a census varies one. [None] is
          the paragraph's, which is what every cluster of the class-pair censuses
          is. *)
  piece_advance : int option;  (** [None] is the paragraph's em. *)
  piece_frame : string option;  (** [None] is the paragraph's full-em frame. *)
  piece_role : string option;  (** What the caller says the occurrence is used as. *)
}

let plain (text : string) : piece =
  { piece_text = text; piece_size = None; piece_advance = None; piece_frame = None; piece_role = None }

(** The same character at an inline em of its own, which is the only way to see
    which of a boundary's two characters an amount was measured from. *)
let sized (text : string) (inline : int) : piece =
  { (plain text) with piece_size = Some (inline, em); piece_advance = Some inline }

(** The same character with a role the caller declares.

    Several rules read the role rather than the code point -- §3.1.3's decimal point
    and digit group separator are vertical-only, §B.2 note 12's unit symbol is not --
    and a census that states one on every class in turn is how the scope of such a
    rule becomes visible. *)
let roled (text : string) (role : string) : piece = { (plain text) with piece_role = Some role }

(** The same character in a frame of its own.

    §3.9.2 reads the frame: the same code point is a Western character when it
    arrives proportional and an ideographic one when it does not, and §3.2.4 turns
    that into an orientation -- a fixed-width Western character is quasi-Japanese and
    stands upright where a proportional one is rotated. *)
let framed (text : string) (frame : string) (advance : int) : piece =
  { (plain text) with piece_frame = Some frame; piece_advance = Some advance }

(** One member of a tate-chu-yoko run: a proportional character with an advance and
    a block em of its own, so that a census can vary the run's width across the line
    and its height along it independently. *)
let member (text : string) ~(advance : int) ~(block : int) : piece =
  {
    piece_text = text;
    piece_size = Some (em, block);
    piece_advance = Some advance;
    piece_frame = Some "proportional";
    piece_role = None;
  }

(** Which break opportunities a request states.

    The schema refuses offset zero and the end of the source, so the boundaries a
    request can name are exactly the starts of the second and later clusters. *)
type breaks =
  | No_break  (** Nothing but the paragraph end, which is a break by definition. *)
  | Every_boundary  (** Every internal cluster boundary, [allowed]. *)
  | Boundaries_before of int list
      (** [allowed] at the start of each named cluster, and nowhere else.

          A census that puts a construct on the line cannot offer every boundary: a
          tate-chu-yoko run is indivisible, and a request that asks for a break
          inside one is refused rather than answered, which would end the census
          rather than measure anything. This states the boundaries that exist. *)
  | Mandatory_after of int
      (** One [mandatory] break after the nth cluster, so that what precedes it is a
          line and is not the last one. *)

(** The reading a ruby construct carries, in {i piece} indices on both sides.

    A run names a half-open range of the line's own pieces and a half-open range of
    the reading's, which is exactly what the protocol's [runs] array names in bytes.
    Stating it in pieces is what lets a census vary the association -- one ruby
    character over one base, five over two, three runs over three bases -- without
    counting a byte. *)
type ruby = {
  ruby_kind : string;  (** [mono], [group] or [jukugo]. *)
  ruby_reading : piece list;
  ruby_runs : (int * int * int * int) list;
      (** The base's first and last piece index, then the reading's. *)
}

(** One of §3.6.2's four tab stops, as the protocol spells it. A [character] stop
    carries the one scalar it aligns on and the other three carry none. *)
type tab_stop = {
  stop_position : int;
  stop_alignment : string;  (** [start], [center], [end] or [character]. *)
  stop_character : string option;
}

let stop ?(alignment = "start") ?character (position : int) : tab_stop =
  { stop_position = position; stop_alignment = alignment; stop_character = character }

(** One construct over a half-open range of {i piece} indices. The census names the
    clusters it means and this is where they become the byte offsets the protocol
    carries. *)
type span = {
  span_kind : string;
  span_first : int;
  span_last : int;  (** One past the last piece the construct covers. *)
  span_ruby : ruby option;  (** The reading a ruby construct carries. *)
  span_mark : string option;  (** The repeated symbol an emphasis run carries. *)
  span_annotation : piece list option;
      (** The text a script or a reference-mark construct carries beside its base. *)
  span_columns : (int * int) option;  (** A furawake's column count and line gap. *)
  span_cells : int option;  (** A jidori's declared length, in full-em cells. *)
}

let span_default =
  {
    span_kind = "tate-chu-yoko";
    span_first = 0;
    span_last = 0;
    span_ruby = None;
    span_mark = None;
    span_annotation = None;
    span_columns = None;
    span_cells = None;
  }

let tate_chu_yoko (first : int) (last : int) : span =
  { span_default with span_kind = "tate-chu-yoko"; span_first = first; span_last = last }

(** The three things a shaped text is on the wire: its source, its clusters, and the
    byte offset each piece starts at. The paragraph's own text and a ruby reading are
    built the same way, so they are built by the same function. *)
let shaped ?(default_advance = em) (pieces : piece list) :
    string * Jlreq_proto.Json.t list * int array =
  let source = String.concat "" (List.map (fun piece -> piece.piece_text) pieces) in
  let starts = Array.make (List.length pieces + 1) 0 in
  let clusters, _ =
    List.fold_left
      (fun (clusters, start) piece ->
        let index = List.length clusters in
        starts.(index) <- start;
        let stop = start + String.length piece.piece_text in
        starts.(index + 1) <- stop;
        let advance = Option.value ~default:default_advance piece.piece_advance in
        let own_size =
          match piece.piece_size with
          | None -> []
          | Some (inline, block) ->
            [
              ( "size",
                Jlreq_proto.Json.Object
                  [
                    ("inline", Jlreq_proto.Json.of_int inline);
                    ("block", Jlreq_proto.Json.of_int block);
                  ] );
            ]
        in
        let own_frame =
          match piece.piece_frame with
          | None -> []
          | Some frame -> [ ("frame", Jlreq_proto.Json.String frame) ]
        in
        let own_role =
          match piece.piece_role with
          | None -> []
          | Some role -> [ ("role", Jlreq_proto.Json.String role) ]
        in
        let cluster =
          Jlreq_proto.Json.Object
            ([
               ("range", Jlreq_proto.Json.Array [ Jlreq_proto.Json.of_int start; Jlreq_proto.Json.of_int stop ]);
               ("advance", Jlreq_proto.Json.of_int advance);
             ]
            @ own_size @ own_frame @ own_role)
        in
        (cluster :: clusters, stop))
      ([], 0) pieces
  in
  (source, List.rev clusters, starts)

(** A request whose clusters are [pieces], one em each unless a piece states its own.

    Everything a census does not vary is fixed here: a full-em frame and one cluster
    per piece whose advance is its em. *)
let request ~(pieces : piece list) ~(line_extent : int) ~(breaks : breaks)
    ~(alignment : string) ?(writing_mode = "horizontal-tb") ?(spans : span list = [])
    ?(first_line_indent = 0) ?(tab_stops : tab_stop list = []) ?(state_alignment = true)
    ?(widow : int option) ~(style : Jlreq_proto.Json.t) () : Jlreq_proto.Json.t =
  let source, clusters, starts = shaped pieces in
  let boundaries =
    List.filteri (fun index _ -> index > 0 && index < List.length pieces)
      (Array.to_list (Array.map (fun offset -> offset) starts))
  in
  let constructs =
    if spans = [] then []
    else
      [
        ( "constructs",
          Jlreq_proto.Json.Array
            (List.map
               (fun span ->
                 if span.span_first < 0 || span.span_last > List.length pieces then
                   fault "a census puts a %s over pieces %d..%d of %d" span.span_kind
                     span.span_first span.span_last (List.length pieces);
                 let reading =
                   match span.span_ruby with
                   | None -> []
                   | Some ruby ->
                     let ruby_source, ruby_clusters, ruby_starts =
                       shaped ~default_advance:(em / 2) ruby.ruby_reading
                     in
                     [
                       ("ruby_kind", Jlreq_proto.Json.String ruby.ruby_kind);
                       ( "annotation",
                         Jlreq_proto.Json.Object
                           [
                             ("source", Jlreq_proto.Json.String ruby_source);
                             ("size", ruby_size);
                             ("frame", Jlreq_proto.Json.String "full-em");
                             ("clusters", Jlreq_proto.Json.Array ruby_clusters);
                           ] );
                       ( "runs",
                         Jlreq_proto.Json.Array
                           (List.map
                              (fun (base_first, base_last, ann_first, ann_last) ->
                                Jlreq_proto.Json.Object
                                  [
                                    ( "base",
                                      Jlreq_proto.Json.Array
                                        [
                                          Jlreq_proto.Json.of_int starts.(base_first);
                                          Jlreq_proto.Json.of_int starts.(base_last);
                                        ] );
                                    ( "annotation",
                                      Jlreq_proto.Json.Array
                                        [
                                          Jlreq_proto.Json.of_int ruby_starts.(ann_first);
                                          Jlreq_proto.Json.of_int ruby_starts.(ann_last);
                                        ] );
                                  ])
                              ruby.ruby_runs) );
                     ]
                 in
                 let mark =
                   match span.span_mark with
                   | None -> []
                   | Some mark -> [ ("mark", Jlreq_proto.Json.String mark) ]
                 in
                 let annotation =
                   match span.span_annotation with
                   | None -> []
                   | Some pieces ->
                     let text, annotation_clusters, _ =
                       shaped ~default_advance:ruby_em pieces
                     in
                     [
                       ( "annotation",
                         Jlreq_proto.Json.Object
                           [
                             ("source", Jlreq_proto.Json.String text);
                             ("size", ruby_size);
                             ("frame", Jlreq_proto.Json.String "full-em");
                             ("clusters", Jlreq_proto.Json.Array annotation_clusters);
                           ] );
                     ]
                 in
                 let columns =
                   match span.span_columns with
                   | None -> []
                   | Some (count, gap) ->
                     [
                       ("columns", Jlreq_proto.Json.of_int count);
                       ("line_gap", Jlreq_proto.Json.of_int gap);
                     ]
                 in
                 let cells =
                   match span.span_cells with
                   | None -> []
                   | Some cells -> [ ("cells", Jlreq_proto.Json.of_int cells) ]
                 in
                 Jlreq_proto.Json.Object
                   ([
                      ("kind", Jlreq_proto.Json.String span.span_kind);
                      ( "range",
                        Jlreq_proto.Json.Array
                          [
                            Jlreq_proto.Json.of_int starts.(span.span_first);
                            Jlreq_proto.Json.of_int starts.(span.span_last);
                          ] );
                    ]
                   @ reading @ mark @ annotation @ columns @ cells))
               spans) );
      ]
  in
  let stated =
    match breaks with
    | No_break -> []
    | Every_boundary -> List.map (fun offset -> (offset, "allowed")) boundaries
    | Boundaries_before indices ->
      List.map
        (fun index ->
          if index < 1 || index >= List.length pieces then
            fault "a census offers a break before piece %d of %d" index (List.length pieces);
          (starts.(index), "allowed"))
        indices
    | Mandatory_after count -> (
      match List.nth_opt boundaries (count - 1) with
      | Some offset -> [ (offset, "mandatory") ]
      | None -> fault "a census asks for a break after cluster %d of %d" count (List.length pieces))
  in
  let breaks =
    if stated = [] then []
    else
      [
        ( "breaks",
          Jlreq_proto.Json.Array
            (List.map
               (fun (offset, kind) ->
                 Jlreq_proto.Json.Object
                   [
                     ("offset", Jlreq_proto.Json.of_int offset);
                     ("kind", Jlreq_proto.Json.String kind);
                   ])
               stated) );
      ]
  in
  let stops =
    if tab_stops = [] then []
    else
      [
        ( "tab_stops",
          Jlreq_proto.Json.Array
            (List.map
               (fun stop ->
                 Jlreq_proto.Json.Object
                   ([
                      ("position", Jlreq_proto.Json.of_int stop.stop_position);
                      ("alignment", Jlreq_proto.Json.String stop.stop_alignment);
                    ]
                   @
                   match stop.stop_character with
                   | None -> []
                   | Some character -> [ ("character", Jlreq_proto.Json.String character) ]))
               tab_stops) );
      ]
  in
  Jlreq_proto.Json.Object
    ([
       ("source", Jlreq_proto.Json.String source);
       ("size", size);
       ("frame", Jlreq_proto.Json.String "full-em");
       ("clusters", Jlreq_proto.Json.Array clusters);
       ("line_extent", Jlreq_proto.Json.of_int line_extent);
     ]
    @ breaks @ constructs @ stops
    @ (if first_line_indent = 0 then []
       else [ ("first_line_indent", Jlreq_proto.Json.of_int first_line_indent) ])
    @ (if state_alignment then [ ("alignment", Jlreq_proto.Json.String alignment) ] else [])
    @ (match widow with
      | None -> []
      | Some minimum -> [ ("widow_minimum_clusters", Jlreq_proto.Json.of_int minimum) ])
    @ [
        ("writing_mode", Jlreq_proto.Json.String writing_mode);
        ("style", style);
      ])

let envelope ~(id : string) ~(request : Jlreq_proto.Json.t) : Jlreq_proto.Json.t =
  Jlreq_proto.Json.Object
    [
      ("protocol", Jlreq_proto.Json.String Jlreq_proto.Protocol.protocol);
      ("spec", Jlreq_proto.Json.String Jlreq_proto.Protocol.spec);
      ("id", Jlreq_proto.Json.String id);
      ("request", request);
    ]

(** Every ordered pair of representatives, in class order. A matrix is not
    symmetric -- [(cl-01, cl-02)] and [(cl-02, cl-01)] are different cells -- so
    the census walks ordered pairs and not combinations. *)
let each_pair (f : representative -> representative -> unit) : unit =
  List.iter (fun before -> List.iter (fun after -> f before after) representatives) representatives

(* ----------------------------------------------------------------------------- *)
(* The censuses *)
(* ----------------------------------------------------------------------------- *)

(** Where the pair sits relative to the line edges.

    A bare pair on a line of its own answers three questions at once: what Table 1
    puts at the line head before the first character, what it puts between the two,
    and what it puts at the line end after the second. The padded variants separate
    them, so a difference that shows up in [interior] is about the pair's own cell,
    and one that shows up in [head] but not in [interior] is about the matrix's
    [line-head] row. *)
let spacing_variants (before : string) (after : string) : (string * string list) list =
  [
    ("pair", [ before; after ]);
    ("head", [ before; after; filler ]);
    ("end", [ filler; before; after ]);
    ("interior", [ filler; before; after; filler ]);
  ]

let spacing_census (emit : string -> Jlreq_proto.Json.t -> unit) : unit =
  each_pair (fun before after ->
      List.iter
        (fun (variant, texts) ->
          emit
            (Printf.sprintf "census/spacing/%s+%s/%s" before.rep_label after.rep_label variant)
            (request ~pieces:(List.map plain texts) ~line_extent:wide_extent ~breaks:No_break
               ~alignment:"start" ~style:(style []) ()))
        (spacing_variants before.rep_text after.rep_text))

(** The four levels §C.3 grades the prohibitions by. Table 2's [not 3,4] cells are
    only prohibitions at two of them, so a break census that fixed the level would
    read two of the table's four columns and call it the table. *)
let kinsoku_levels = [ "very-loose"; "loose"; "strict"; "very-strict" ]

let break_census (emit : string -> Jlreq_proto.Json.t -> unit) : unit =
  each_pair (fun before after ->
      List.iter
        (fun level ->
          emit
            (Printf.sprintf "census/break/%s+%s/%s" before.rep_label after.rep_label level)
            (request
               ~pieces:(List.map plain [ before.rep_text; after.rep_text ])
               ~line_extent:em ~breaks:Every_boundary ~alignment:"start"
               ~style:(style [ ("kinsoku.level", level) ]) ()))
        kinsoku_levels)

(** The pair between two ideographs, on a line exactly as wide as the four ems it
    holds.

    Nothing can break -- the request states no opportunity, so the paragraph is one
    line -- and every unit of space any of the five Table 1 coordinates puts on that
    line has to be given back by §3.8.3's ladder or reported as an overrun. A pair
    whose own cell is solid fits exactly and is the control. *)
let reduction_interior (before : string) (after : string) : piece list =
  List.map plain [ filler; before; after; filler ]

(** The same pair with the line ending on it, which is the only way to put the
    pair's own trailing member against Table 3, 4 or 5's [line-end] column and to
    reach §3.8.2's hanging punctuation. *)
let reduction_tail (before : string) (after : string) : piece list =
  List.map plain [ filler; before; after ]

let reduction_variants =
  [
    ("table-3", reduction_interior, 4 * em, [ ("adjustment.reduction_table", "table-3") ]);
    ("table-4", reduction_interior, 4 * em, [ ("adjustment.reduction_table", "table-4") ]);
    ("table-5", reduction_interior, 4 * em, [ ("adjustment.reduction_table", "table-5") ]);
    ("trailing", reduction_interior, 4 * em, [ ("adjustment.remainder", "trailing") ]);
    ("line-end", reduction_tail, 3 * em, []);
    ("hanging", reduction_tail, 3 * em, [ ("adjustment.hanging_punctuation", "hanging") ]);
  ]

let reduction_census (emit : string -> Jlreq_proto.Json.t -> unit) : unit =
  each_pair (fun before after ->
      List.iter
        (fun (variant, shape, line_extent, settings) ->
          emit
            (Printf.sprintf "census/reduction/%s+%s/%s" before.rep_label after.rep_label variant)
            (request
               ~pieces:(shape before.rep_text after.rep_text)
               ~line_extent ~breaks:No_break ~alignment:"start" ~style:(style settings) ()))
        reduction_variants)

(** The pair between two ideographs on a justified line with a second line after
    it.

    §3.8.4 opens a justified line that is not the last one, so the request states a
    mandatory break after the fourth cluster: the four ems before it are a line
    with room left over, and the fifth cluster is the last line, which is meant to
    be short and is left alone. *)
let expansion_pieces (before : string) (after : string) : piece list =
  List.map plain [ filler; before; after; filler; filler ]

(** The same line with the trailing member of the pair set at half the em.

    Table 6 names a class pair and no neighbor (ADR 0021), so an engine has to
    decide which character's em a quarter of an em is a quarter of. On a line whose
    every cluster is the same size that decision is invisible; here it is not. *)
let expansion_mixed (before : string) (after : string) : piece list =
  [ plain filler; plain before; sized after (em / 2); plain filler; plain filler ]

(** Two measures, because §3.8.4's ladder has two regimes. Three quarters of an em
    of room over three boundaries sits inside the ceilings the first three stages
    state, and two ems does not: the second measure is the only way to reach step
    (d), which opens every stage's own boundaries past their ceilings until the line
    is full. *)
let expansion_variants =
  [
    ("stage", expansion_pieces, (4 * em) + (em * 3 / 4), []);
    ("residual", expansion_pieces, 6 * em, []);
    ( "third-em",
      expansion_pieces,
      (4 * em) + (em * 3 / 4),
      [ ("adjustment.japanese_latin_expansion_ceiling", "third-em") ] );
    ( "rigid",
      expansion_pieces,
      (4 * em) + (em * 3 / 4),
      [ ("adjustment.japanese_latin_expansion_ceiling", "rigid") ] );
    ("table-5", expansion_pieces, (4 * em) + (em * 3 / 4), [ ("adjustment.reduction_table", "table-5") ]);
    ("mixed-em", expansion_mixed, (4 * em) + (em * 3 / 4), []);
  ]

let expansion_census (emit : string -> Jlreq_proto.Json.t -> unit) : unit =
  each_pair (fun before after ->
      List.iter
        (fun (variant, shape, line_extent, settings) ->
          emit
            (Printf.sprintf "census/expansion/%s+%s/%s" before.rep_label after.rep_label variant)
            (request
               ~pieces:(shape before.rep_text after.rep_text)
               ~line_extent ~breaks:(Mandatory_after 4) ~alignment:"justify"
               ~style:(style settings) ()))
        expansion_variants)

(* ----------------------------------------------------------------------------- *)
(* Vertical composition (§3.1.3, §3.2.3, §3.2.4, §3.2.6) *)
(* ----------------------------------------------------------------------------- *)

(** The pair between two ideographs in a vertical line wide enough to hold it all.

    Vertical composition is not a different set of tables; it is the same ones asked
    in a context that changes three answers. §3.9.2 reads the frame, so a Western
    code point classifies differently upright than rotated; §3.1.3 hands two marks a
    vertical-only exception; and §3.2's orientation gives each placement its own
    writing mode and transform, which are two response fields the horizontal
    censuses never see anything but one value in. *)
let vertical_interior (before : piece) (after : piece) : piece list =
  [ plain filler; before; after; plain filler ]

(** The variants, each of which changes exactly one thing about the pair.

    [upright] is the control: full-em clusters in a vertical line, which is the
    spacing census's [interior] variant asked again in the other writing mode.
    [rotated] sets both members proportional, which §3.2.6 turns a quarter turn
    clockwise and §3.9.2 may reclassify on the way. [quasi-japanese] sets them in a
    half-em frame instead, which §3.2.4 reads as Japanese and leaves standing up --
    the same code point, the same advance, a different answer to both fields. The
    two role variants state §3.1.3's roles on every class in turn, because the
    section names two marks and the engines have to agree on what the role does to
    the twenty-one classes it does not name. *)
let vertical_variants =
  [
    ("upright", fun (before : string) (after : string) -> vertical_interior (plain before) (plain after));
    ("rotated", fun before after ->
      vertical_interior (framed before "proportional" (em / 2)) (framed after "proportional" (em / 2)));
    ("quasi-japanese", fun before after ->
      vertical_interior (framed before "half-em" (em / 2)) (framed after "half-em" (em / 2)));
    ("decimal-point", fun before after ->
      vertical_interior (roled before "decimal-point") (plain after));
    ("digit-group", fun before after ->
      vertical_interior (roled before "digit-group-separator") (plain after));
  ]

let vertical_census (emit : string -> Jlreq_proto.Json.t -> unit) : unit =
  each_pair (fun before after ->
      List.iter
        (fun (variant, shape) ->
          emit
            (Printf.sprintf "census/vertical/%s+%s/%s" before.rep_label after.rep_label variant)
            (request
               ~pieces:(shape before.rep_text after.rep_text)
               ~line_extent:wide_extent ~breaks:No_break ~alignment:"start"
               ~writing_mode:"vertical-rl" ~style:(style []) ());
          (* The same pair on a line with room for one cluster and every boundary
             offered, which is the break census asked in vertical composition: the
             classes the frame and §3.1.3 change are the ones Table 2 is read at. *)
          emit
            (Printf.sprintf "census/vertical/%s+%s/%s-break" before.rep_label after.rep_label
               variant)
            (request
               ~pieces:(shape before.rep_text after.rep_text)
               ~line_extent:em ~breaks:Every_boundary ~alignment:"start"
               ~writing_mode:"vertical-rl" ~style:(style []) ()))
        vertical_variants)

(* ----------------------------------------------------------------------------- *)
(* Tate-chu-yoko (§3.2.5, §C.2 note 13, §E.2 note 12) *)
(* ----------------------------------------------------------------------------- *)

(** A run of [count] members, each a European numeral.

    The advances are deliberately unequal and deliberately not divisible by two, so
    that a run's total width is odd for the odd member counts: §3.2.5 centers the
    whole string on the line and says nothing about which way half of an odd number
    rounds, which is invisible on any run whose members are all the same round
    width. The block ems differ too, because what the run takes up {i along} the line
    is a height and the members do not have to share one. *)
let run_members (count : int) : piece list =
  let widths = [| 300; 433; 500; 267 |] and blocks = [| 1000; 1200; 900; 1000 |] in
  List.init count (fun index ->
      member
        (String.make 1 (Char.chr (Char.code '1' + index)))
        ~advance:widths.(index mod Array.length widths)
        ~block:blocks.(index mod Array.length blocks))

(** How far along the line a run of [count] members reaches: its tallest member's
    block em, which is what the vertical line sees of a horizontal string. Kept here
    so that a census can state a measure that is exactly full. *)
let run_extent (count : int) : int =
  let blocks = [| 1000; 1200; 900; 1000 |] in
  let widest = ref 0 in
  List.iteri
    (fun index () ->
      let block = blocks.(index mod Array.length blocks) in
      if block > !widest then widest := block)
    (List.init count (fun _ -> ()));
  !widest

(** One run between the pair, which is the shape every cl-30 coordinate of Tables 1
    through 6 is reachable by: the pair's leading member is what stands before the
    run and its trailing member is what stands after it. *)
let tate_chu_yoko_one (count : int) (before : string) (after : string) :
    piece list * span list =
  ([ plain before ] @ run_members count @ [ plain after ], [ tate_chu_yoko 1 (1 + count) ])

(** Two runs back to back, which is the only shape that reaches the [(cl-30, cl-30)]
    coordinate at all -- and the only one that tells §C.2 note 13 and §E.2 note 12
    apart from the blank and the quarter em their tables state there, because the
    same coordinate {i inside} one run is what the two notes withdraw. *)
let tate_chu_yoko_two (before : string) (after : string) : piece list * span list =
  ( [ plain before ] @ run_members 2 @ run_members 2 @ [ plain after ],
    [ tate_chu_yoko 1 3; tate_chu_yoko 3 5 ] )

(** The same two runs with a fifth cluster after a mandatory break, so the line that
    holds them is justified and is not the paragraph's last. *)
let tate_chu_yoko_justified (before : string) (after : string) : piece list * span list =
  let pieces, spans = tate_chu_yoko_two before after in
  (pieces @ [ plain filler ], spans)

(** The same justified line with the character after the second run at half the em.

    Table 6 names a class pair and no neighbor (ADR 0021), so a boundary beside a run
    has to be measured from one of the two ems -- the run's member or the character
    it stands against -- and on a line whose every cluster is the same size that
    choice cannot be seen. Three boundaries share the line's shortfall in proportion
    to those ems, so here it can. *)
let tate_chu_yoko_mixed (before : string) (after : string) : piece list * span list =
  let pieces, spans = tate_chu_yoko_two before after in
  match List.rev pieces with
  | _ :: leading -> (List.rev (sized after (em / 2) :: leading) @ [ plain filler ], spans)
  | [] -> fault "the tate-chu-yoko census built an empty line"

type tate_chu_yoko_variant = {
  tcy_name : string;
  tcy_shape : string -> string -> piece list * span list;
  tcy_extent : int;
  tcy_breaks : breaks;
  tcy_alignment : string;
  tcy_settings : (string * string) list;
}

(** A run's own width along the line is its tallest member's block em, so a line
    that holds [before], one run and [after] is two ems plus that: stating the
    measure exactly forces every unit Table 1 puts around the run to come back out
    through §3.8.3, and stating it three quarters of an em over forces §3.8.4 to put
    some in. *)
let tate_chu_yoko_variants =
  [
    {
      tcy_name = "solid";
      tcy_shape = tate_chu_yoko_one 2;
      tcy_extent = wide_extent;
      tcy_breaks = No_break;
      tcy_alignment = "start";
      tcy_settings = [];
    };
    {
      tcy_name = "single";
      tcy_shape = tate_chu_yoko_one 1;
      tcy_extent = wide_extent;
      tcy_breaks = No_break;
      tcy_alignment = "start";
      tcy_settings = [];
    };
    {
      tcy_name = "odd";
      tcy_shape = tate_chu_yoko_one 3;
      tcy_extent = wide_extent;
      tcy_breaks = No_break;
      tcy_alignment = "start";
      tcy_settings = [];
    };
    {
      tcy_name = "adjacent";
      tcy_shape = tate_chu_yoko_two;
      tcy_extent = wide_extent;
      tcy_breaks = No_break;
      tcy_alignment = "start";
      tcy_settings = [];
    };
    {
      tcy_name = "reduce";
      tcy_shape = tate_chu_yoko_one 2;
      tcy_extent = (2 * em) + run_extent 2;
      tcy_breaks = No_break;
      tcy_alignment = "start";
      tcy_settings = [];
    };
    {
      tcy_name = "justify";
      tcy_shape = tate_chu_yoko_justified;
      tcy_extent = (2 * em) + (2 * run_extent 2) + (em * 3 / 4);
      tcy_breaks = Mandatory_after 6;
      tcy_alignment = "justify";
      tcy_settings = [];
    };
    {
      tcy_name = "justify-mixed-em";
      tcy_shape = tate_chu_yoko_mixed;
      tcy_extent = (2 * em) + (2 * run_extent 2) + (em * 3 / 4);
      tcy_breaks = Mandatory_after 6;
      tcy_alignment = "justify";
      tcy_settings = [];
    };
    {
      tcy_name = "break";
      tcy_shape = tate_chu_yoko_two;
      tcy_extent = em;
      tcy_breaks = Boundaries_before [ 1; 3; 5 ];
      tcy_alignment = "start";
      tcy_settings = [];
    };
    {
      tcy_name = "break-very-loose";
      tcy_shape = tate_chu_yoko_two;
      tcy_extent = em;
      tcy_breaks = Boundaries_before [ 1; 3; 5 ];
      tcy_alignment = "start";
      tcy_settings = [ ("kinsoku.level", "very-loose") ];
    };
  ]

let tate_chu_yoko_census (emit : string -> Jlreq_proto.Json.t -> unit) : unit =
  each_pair (fun before after ->
      List.iter
        (fun variant ->
          let pieces, spans = variant.tcy_shape before.rep_text after.rep_text in
          emit
            (Printf.sprintf "census/tate-chu-yoko/%s+%s/%s" before.rep_label after.rep_label
               variant.tcy_name)
            (request ~pieces ~line_extent:variant.tcy_extent ~breaks:variant.tcy_breaks
               ~alignment:variant.tcy_alignment ~writing_mode:"vertical-rl" ~spans
               ~style:(style variant.tcy_settings) ()))
        tate_chu_yoko_variants)

(* ----------------------------------------------------------------------------- *)
(* Ruby *)
(* ----------------------------------------------------------------------------- *)

(** The base characters a reading is attached to.

    Ideographic characters, because §3.3 attaches ruby to 漢字等 -- though what they
    hold does not matter to the geometry: §3.9.2 gives every character a construct
    covers the construct's own class, cl-22 or cl-23, and the pair under test is the
    two characters {i beside} the construct rather than the ones inside it. *)
let ruby_bases (count : int) : piece list =
  let letters = [| "日"; "本"; "語"; "文" |] in
  List.init count (fun index -> plain letters.(index mod Array.length letters))

(** [count] ruby characters, each a hiragana at §3.3.3's half size. *)
let ruby_reading (count : int) : piece list =
  let letters = [| "に"; "ほ"; "ん"; "ご"; "か"; "な"; "じ"; "ま" |] in
  List.init count (fun index ->
      {
        (plain letters.(index mod Array.length letters)) with
        piece_size = Some (ruby_em, ruby_em);
        piece_advance = Some ruby_em;
      })

(** The same reading at an em of its own.

    Two things ride on the ruby character's own em and are invisible at half the base
    character's. One is arithmetic: §3.3.8's "plus 1/2 a ruby character size" and the
    proportional shares §3.3.6 and §F.3 divide a surplus into both round, and an em
    that halves and thirds badly is where a rounding shows. The other is §F's own
    geometry: a reading whose characters divide the base character's em exactly tiles
    the base characters it is set over, and every one of §F.2's own questions -- which
    base character a reading is allowed to reach into, and how far -- is answered by
    the tiling rather than by the rule when it does. *)
let ruby_reading_at (size : int) (count : int) : piece list =
  List.map
    (fun piece -> { piece with piece_size = Some (size, size); piece_advance = Some size })
    (ruby_reading count)

(** The same reading with unequal advances.

    §3.3.6's ratios are stated over "the number of ruby characters" and §F.3's over
    "the number of ruby characters (or the length of ruby characters when set solid)",
    which are the same number on a reading whose characters are all one width and two
    different numbers on one whose are not. *)
let ruby_reading_uneven (count : int) : piece list =
  let widths = [| 300; 500; 700; 400 |] in
  List.mapi
    (fun index piece ->
      { piece with piece_advance = Some widths.(index mod Array.length widths) })
    (ruby_reading count)

(** The same reading with its last character at an em of its own.

    §3.3.8 measures every overhang in "the full-width size of a ruby character", and
    a reading whose characters are all one size cannot say {i which} ruby character
    that is -- the one at the end doing the overhanging, or the reading's own stated
    size. The block em travels with it, because that is what the line has to make
    room for across itself and §3.3.4 says nothing about a reading of two heights. *)
let ruby_reading_mixed (count : int) : piece list =
  match List.rev (ruby_reading count) with
  | last :: leading ->
    List.rev ({ last with piece_size = Some (ruby_em * 7 / 5, ruby_em * 7 / 5) } :: leading)
  | [] -> fault "a ruby census asked for a reading of no characters"

(** A ruby construct over the pieces from [first], one run per entry of [runs]: how
    many base pieces that run covers and how many ruby characters it carries. *)
let ruby_over ~(kind : string) ~(first : int) ~(runs : (int * int) list) ~(mixed : bool)
    ~(uneven : bool) ~(size : int) : span =
  let annotations = List.fold_left (fun sum (_, count) -> sum + count) 0 runs in
  let reading =
    if uneven then ruby_reading_uneven annotations
    else if mixed then ruby_reading_mixed annotations
    else if size > 0 then ruby_reading_at size annotations
    else ruby_reading annotations
  in
  let base = ref first and annotation = ref 0 and entries = ref [] in
  List.iter
    (fun (bases, count) ->
      entries := (!base, !base + bases, !annotation, !annotation + count) :: !entries;
      base := !base + bases;
      annotation := !annotation + count)
    runs;
  {
    span_default with
    span_kind = "ruby";
    span_first = first;
    span_last = !base;
    span_ruby = Some { ruby_kind = kind; ruby_reading = reading; ruby_runs = List.rev !entries };
  }

type ruby_variant = {
  rb_name : string;
  rb_kind : string;
  rb_runs : (int * int) list;
  rb_pair : bool;  (** A second construct of the same shape right behind the first. *)
  rb_mixed : bool;
  rb_uneven : bool;  (** The reading's characters are of unequal advance. *)
  rb_size : int;  (** The reading's own em, or [0] for half the base character's. *)
  rb_narrow : bool;  (** The pair under test is set at half the em. *)
  rb_head : bool;  (** The construct starts the line: nothing but the indent before it. *)
  rb_tail : bool;  (** One more cluster after a mandatory break, so the line is not the last. *)
  rb_extent : int -> int;  (** The measure, from the number of clusters before the tail. *)
  rb_breaks : int -> breaks;
  rb_alignment : string;
  rb_writing_mode : string;
  rb_indent : int;
  rb_settings : (string * string) list;
}

let ruby_default =
  {
    rb_name = "";
    rb_kind = "mono";
    rb_runs = [ (1, 2) ];
    rb_pair = false;
    rb_mixed = false;
    rb_uneven = false;
    rb_size = 0;
    rb_narrow = false;
    rb_head = false;
    rb_tail = false;
    rb_extent = (fun _ -> wide_extent);
    rb_breaks = (fun _ -> No_break);
    rb_alignment = "start";
    rb_writing_mode = "horizontal-tb";
    rb_indent = 0;
    rb_settings = [];
  }

(** A line that holds one or two ruby constructs, with the pair under test on either
    side of them: [before] is what the reading may reach back over and [after] is what
    it may reach forward over, which is the whole of §3.3.8's own question. *)
let ruby_shape (variant : ruby_variant) (before : string) (after : string) :
    piece list * span list * int =
  let bases = List.fold_left (fun sum (count, _) -> sum + count) 0 variant.rb_runs in
  let edge text = if variant.rb_narrow then sized text (em / 2) else plain text in
  let head = if variant.rb_head then [] else [ edge before ] in
  let first = List.length head in
  let copies = if variant.rb_pair then 2 else 1 in
  let spans =
    List.init copies (fun copy ->
        ruby_over ~kind:variant.rb_kind
          ~first:(first + (copy * bases))
          ~runs:variant.rb_runs ~mixed:variant.rb_mixed ~uneven:variant.rb_uneven
          ~size:variant.rb_size)
  in
  let body = head @ ruby_bases (bases * copies) @ [ edge after ] in
  let pieces = body @ if variant.rb_tail then [ plain filler ] else [] in
  (pieces, spans, List.length body)

let ruby_variants =
  [
    { ruby_default with rb_name = "mono-solid" };
    { ruby_default with rb_name = "mono-short"; rb_runs = [ (1, 1) ] };
    { ruby_default with rb_name = "mono-long"; rb_runs = [ (1, 3) ] };
    {
      ruby_default with
      rb_name = "mono-long-katatsuki";
      rb_runs = [ (1, 3) ];
      rb_settings = [ ("ruby.alignment", "katatsuki") ];
    };
    { ruby_default with rb_name = "mono-adjacent"; rb_runs = [ (1, 3); (1, 3) ] };
    { ruby_default with rb_name = "mono-mixed"; rb_runs = [ (1, 3) ]; rb_mixed = true };
    { ruby_default with rb_name = "group-jis-short"; rb_kind = "group"; rb_runs = [ (2, 3) ] };
    {
      ruby_default with
      rb_name = "group-flush-short";
      rb_kind = "group";
      rb_runs = [ (2, 3) ];
      rb_settings = [ ("ruby.group_distribution", "flush") ];
    };
    { ruby_default with rb_name = "group-jis-long"; rb_kind = "group"; rb_runs = [ (2, 6) ] };
    {
      ruby_default with
      rb_name = "group-flush-long";
      rb_kind = "group";
      rb_runs = [ (2, 6) ];
      rb_settings = [ ("ruby.group_distribution", "flush") ];
    };
    { ruby_default with rb_name = "group-jis-single"; rb_kind = "group"; rb_runs = [ (2, 1) ] };
    {
      ruby_default with
      rb_name = "group-flush-single";
      rb_kind = "group";
      rb_runs = [ (2, 1) ];
      rb_settings = [ ("ruby.group_distribution", "flush") ];
    };
    { ruby_default with rb_name = "jukugo-short"; rb_kind = "jukugo"; rb_runs = [ (1, 1); (1, 2) ] };
    {
      ruby_default with
      rb_name = "jukugo-group";
      rb_kind = "jukugo";
      rb_runs = [ (1, 3); (1, 2) ];
      rb_settings = [ ("ruby.jukugo_layout", "group") ];
    };
    {
      ruby_default with
      rb_name = "jukugo-group-flush";
      rb_kind = "jukugo";
      rb_runs = [ (1, 3); (1, 2) ];
      rb_settings = [ ("ruby.jukugo_layout", "group"); ("ruby.group_distribution", "flush") ];
    };
    {
      ruby_default with
      rb_name = "jukugo-phonetic";
      rb_kind = "jukugo";
      rb_runs = [ (1, 3); (1, 2) ];
      rb_settings = [ ("ruby.jukugo_layout", "phonetic") ];
    };
    {
      ruby_default with
      rb_name = "jukugo-phonetic-three";
      rb_kind = "jukugo";
      rb_runs = [ (1, 3); (1, 4); (1, 2) ];
      rb_settings = [ ("ruby.jukugo_layout", "phonetic") ];
    };
    {
      ruby_default with
      rb_name = "overhang-jis";
      rb_runs = [ (1, 3) ];
      rb_settings = [ ("ruby.overhang_kana", "jis") ];
    };
    {
      ruby_default with
      rb_name = "overhang-any";
      rb_runs = [ (1, 3) ];
      rb_settings = [ ("ruby.overhang_kana", "any") ];
    };
    {
      ruby_default with
      rb_name = "overhang-none";
      rb_runs = [ (1, 3) ];
      rb_settings = [ ("ruby.overhang_kana", "none") ];
    };
    { ruby_default with rb_name = "indent-permitted"; rb_runs = [ (1, 3) ]; rb_head = true; rb_indent = em };
    {
      ruby_default with
      rb_name = "indent-prohibited";
      rb_runs = [ (1, 3) ];
      rb_head = true;
      rb_indent = em;
      rb_settings = [ ("ruby.overhang_indent", "prohibited") ];
    };
    {
      ruby_default with
      rb_name = "vertical-mono-long";
      rb_runs = [ (1, 3) ];
      rb_writing_mode = "vertical-rl";
    };
    {
      ruby_default with
      rb_name = "vertical-mono-katatsuki";
      rb_runs = [ (1, 3) ];
      rb_writing_mode = "vertical-rl";
      rb_settings = [ ("ruby.alignment", "katatsuki") ];
    };
    {
      ruby_default with
      rb_name = "vertical-group-long";
      rb_kind = "group";
      rb_runs = [ (2, 6) ];
      rb_writing_mode = "vertical-rl";
    };
    {
      ruby_default with
      rb_name = "vertical-jukugo-phonetic";
      rb_kind = "jukugo";
      rb_runs = [ (1, 3); (1, 2) ];
      rb_writing_mode = "vertical-rl";
      rb_settings = [ ("ruby.jukugo_layout", "phonetic"); ("ruby.alignment", "katatsuki") ];
    };
    {
      ruby_default with
      rb_name = "justify-mono";
      rb_runs = [ (1, 2); (1, 2) ];
      rb_tail = true;
      rb_alignment = "justify";
      rb_extent = (fun clusters -> (clusters * em) + (2 * em));
      rb_breaks = (fun clusters -> Mandatory_after clusters);
    };
    {
      ruby_default with
      rb_name = "justify-group";
      rb_kind = "group";
      rb_runs = [ (2, 3) ];
      rb_tail = true;
      rb_alignment = "justify";
      rb_extent = (fun clusters -> (clusters * em) + (2 * em));
      rb_breaks = (fun clusters -> Mandatory_after clusters);
    };
    {
      ruby_default with
      rb_name = "justify-jukugo";
      rb_kind = "jukugo";
      rb_runs = [ (1, 1); (1, 2) ];
      rb_tail = true;
      rb_alignment = "justify";
      rb_extent = (fun clusters -> (clusters * em) + (2 * em));
      rb_breaks = (fun clusters -> Mandatory_after clusters);
    };
    {
      ruby_default with
      rb_name = "justify-two-constructs";
      rb_kind = "group";
      rb_runs = [ (1, 2) ];
      rb_pair = true;
      rb_tail = true;
      rb_alignment = "justify";
      rb_extent = (fun clusters -> (clusters * em) + (2 * em));
      rb_breaks = (fun clusters -> Mandatory_after clusters);
    };
    {
      ruby_default with
      rb_name = "break";
      rb_kind = "mono";
      rb_runs = [ (1, 2); (1, 2) ];
      rb_extent = (fun _ -> em);
      rb_breaks = (fun _ -> Every_boundary);
    };
    {
      ruby_default with
      rb_name = "break-jukugo";
      rb_kind = "jukugo";
      rb_runs = [ (1, 2); (1, 2) ];
      rb_extent = (fun _ -> em);
      rb_breaks = (fun _ -> Every_boundary);
    };
    {
      ruby_default with
      (* Every break variant gives each construct one base character, because a
         boundary inside one base character group is not an opportunity (§C.2 note 7)
         and a request that states one is refused rather than answered -- which would
         end the census rather than measure anything, exactly as it would for a
         tate-chu-yoko run. *)
      rb_name = "break-group";
      rb_kind = "group";
      rb_runs = [ (1, 2) ];
      rb_pair = true;
      rb_extent = (fun _ -> em);
      rb_breaks = (fun _ -> Every_boundary);
    };
    {
      ruby_default with
      rb_name = "reduce";
      rb_kind = "group";
      rb_runs = [ (2, 3) ];
      rb_extent = (fun clusters -> (clusters * em) - (em / 2));
    };
    { ruby_default with rb_name = "mono-very-long"; rb_runs = [ (1, 5) ] };
    { ruby_default with rb_name = "mono-huge"; rb_runs = [ (1, 8) ] };
    { ruby_default with rb_name = "mono-uneven"; rb_runs = [ (1, 4) ]; rb_uneven = true };
    {
      ruby_default with
      rb_name = "group-uneven";
      rb_kind = "group";
      rb_runs = [ (2, 5) ];
      rb_uneven = true;
    };
    { ruby_default with rb_name = "group-one-base-jis"; rb_kind = "group"; rb_runs = [ (1, 3) ] };
    {
      ruby_default with
      rb_name = "group-one-base-flush";
      rb_kind = "group";
      rb_runs = [ (1, 3) ];
      rb_settings = [ ("ruby.group_distribution", "flush") ];
    };
    { ruby_default with rb_name = "narrow-neighbors"; rb_runs = [ (1, 5) ]; rb_narrow = true };
    {
      ruby_default with
      rb_name = "narrow-neighbors-any";
      rb_runs = [ (1, 5) ];
      rb_narrow = true;
      rb_settings = [ ("ruby.overhang_kana", "any") ];
    };
    {
      ruby_default with
      rb_name = "jukugo-phonetic-four";
      rb_kind = "jukugo";
      rb_runs = [ (1, 3); (1, 4); (1, 3); (1, 2) ];
      rb_settings = [ ("ruby.jukugo_layout", "phonetic") ];
    };
    {
      ruby_default with
      rb_name = "jukugo-phonetic-narrow";
      rb_kind = "jukugo";
      rb_runs = [ (1, 3); (1, 2) ];
      rb_narrow = true;
      rb_settings = [ ("ruby.jukugo_layout", "phonetic") ];
    };
    {
      ruby_default with
      rb_name = "jukugo-phonetic-uneven";
      rb_kind = "jukugo";
      rb_runs = [ (1, 3); (1, 4) ];
      rb_uneven = true;
      rb_settings = [ ("ruby.jukugo_layout", "phonetic") ];
    };
    {
      ruby_default with
      rb_name = "vertical-two-lines";
      rb_runs = [ (1, 3) ];
      rb_pair = true;
      rb_writing_mode = "vertical-rl";
      rb_extent = (fun _ -> 2 * em);
      rb_breaks = (fun _ -> Mandatory_after 2);
    };
    {
      ruby_default with
      rb_name = "horizontal-two-lines";
      rb_runs = [ (1, 3) ];
      rb_pair = true;
      rb_extent = (fun _ -> 2 * em);
      rb_breaks = (fun _ -> Mandatory_after 2);
    };
    { ruby_default with rb_name = "centered"; rb_runs = [ (1, 3) ]; rb_alignment = "center";
      rb_extent = (fun clusters -> (clusters * em) + (2 * em)) };
    { ruby_default with rb_name = "end-aligned"; rb_runs = [ (1, 3) ]; rb_alignment = "end";
      rb_extent = (fun clusters -> (clusters * em) + (2 * em)) };
    { ruby_default with rb_name = "mono-odd-em"; rb_runs = [ (1, 5) ]; rb_size = 333 };
    {
      ruby_default with
      rb_name = "group-odd-em";
      rb_kind = "group";
      rb_runs = [ (2, 5) ];
      rb_size = 333;
    };
    {
      ruby_default with
      rb_name = "jukugo-phonetic-odd-em";
      rb_kind = "jukugo";
      rb_runs = [ (1, 3); (1, 4) ];
      rb_size = 333;
      rb_settings = [ ("ruby.jukugo_layout", "phonetic") ];
    };
    {
      ruby_default with
      rb_name = "remainder-trailing-mono";
      rb_runs = [ (1, 5) ];
      rb_size = 333;
      rb_settings = [ ("adjustment.remainder", "trailing") ];
    };
    {
      ruby_default with
      rb_name = "remainder-trailing-group";
      rb_kind = "group";
      rb_runs = [ (2, 5) ];
      rb_settings = [ ("adjustment.remainder", "trailing") ];
    };
    {
      ruby_default with
      rb_name = "remainder-trailing-group-short";
      rb_kind = "group";
      rb_runs = [ (3, 4) ];
      rb_settings = [ ("adjustment.remainder", "trailing") ];
    };
    {
      ruby_default with
      rb_name = "remainder-trailing-jukugo";
      rb_kind = "jukugo";
      rb_runs = [ (1, 3); (1, 4); (1, 3) ];
      rb_settings = [ ("ruby.jukugo_layout", "phonetic"); ("adjustment.remainder", "trailing") ];
    };
    {
      ruby_default with
      rb_name = "group-short-three-base";
      rb_kind = "group";
      rb_runs = [ (3, 4) ];
    };
  ]
  @ List.map
      (fun (name, runs) ->
        {
          ruby_default with
          rb_name = "phonetic-" ^ name;
          rb_kind = "jukugo";
          rb_runs = runs;
          rb_size = 2 * em / 5;
          rb_settings = [ ("ruby.jukugo_layout", "phonetic") ];
        })
      [
        ("short-long", [ (1, 1); (1, 4) ]);
        ("short-longer", [ (1, 1); (1, 5) ]);
        ("long-short", [ (1, 4); (1, 1) ]);
        ("longer-short", [ (1, 5); (1, 1) ]);
        ("three-short", [ (1, 3); (1, 1) ]);
        ("short-three", [ (1, 1); (1, 3) ]);
        ("both-long", [ (1, 3); (1, 4) ]);
        ("three-one-four", [ (1, 3); (1, 1); (1, 4) ]);
        ("one-five-one", [ (1, 1); (1, 5); (1, 1) ]);
        ("three-three-three", [ (1, 3); (1, 3); (1, 3) ]);
      ]
  @ [
      {
        ruby_default with
        rb_name = "phonetic-mixed";
        rb_kind = "jukugo";
        rb_runs = [ (1, 3); (1, 4) ];
        rb_mixed = true;
        rb_settings = [ ("ruby.jukugo_layout", "phonetic") ];
      };
      {
        ruby_default with
        rb_name = "phonetic-vertical-narrow";
        rb_kind = "jukugo";
        rb_runs = [ (1, 1); (1, 5) ];
        rb_size = 2 * em / 5;
        rb_writing_mode = "vertical-rl";
        rb_settings = [ ("ruby.jukugo_layout", "phonetic"); ("ruby.alignment", "katatsuki") ];
      };
      {
        ruby_default with
        rb_name = "phonetic-remainder-trailing";
        rb_kind = "jukugo";
        rb_runs = [ (1, 3); (1, 1); (1, 4) ];
        rb_size = 2 * em / 5;
        rb_settings =
          [ ("ruby.jukugo_layout", "phonetic"); ("adjustment.remainder", "trailing") ];
      };
    ]

let ruby_census (emit : string -> Jlreq_proto.Json.t -> unit) : unit =
  each_pair (fun before after ->
      List.iter
        (fun variant ->
          let pieces, spans, clusters = ruby_shape variant before.rep_text after.rep_text in
          emit
            (Printf.sprintf "census/ruby/%s+%s/%s" before.rep_label after.rep_label variant.rb_name)
            (request ~pieces
               ~line_extent:(variant.rb_extent clusters)
               ~breaks:(variant.rb_breaks clusters) ~alignment:variant.rb_alignment
               ~writing_mode:variant.rb_writing_mode ~spans
               ~first_line_indent:variant.rb_indent
               ~style:(style variant.rb_settings) ()))
        ruby_variants)

(* ----------------------------------------------------------------------------- *)
(* The remaining constructs *)
(* ----------------------------------------------------------------------------- *)

(** Four classes are what a character {i becomes} inside one of the structures §3.3.9,
    §3.7.1 and §3.4 build, so Appendix A lists no code point in them and no census
    that picks a representative out of Appendix A can reach their rows and columns.
    This one builds the structure instead, the way the tate-chu-yoko census reaches
    cl-30 and the ruby census reaches cl-22 and cl-23.

    - cl-21, the ornamented character complex: a base character carrying emphasis dots
      (§3.3.9) or a superscript (§3.7.1).
    - cl-20, a reference mark's own characters.
    - cl-28 and cl-29, the brackets §3.4.2 wraps a warichu in, which are those classes
      because of the role the caller gives the occurrence and for no other reason.

    It reaches the geometry with them. §3.3.9's mark is half its base character and
    centered on it; §3.7.1's annotation is centered on the whole complex; §3.4.2
    divides a note into two lines as near the same length as it can and centers them
    across the main line; §3.7.2 does the same with as many columns as the caller
    declared and the gap the caller stated; and §3.7.3 spreads a run of text over a
    declared number of full-em cells. Every one of those is asked here against each of
    the twenty-three classes in turn, on a line too wide to adjust, on one that has to
    give space back, and on a justified one that has room to spare. *)

let warichu_bracket (klass : int) : piece =
  match representative_of_class klass with
  | Some row -> { (plain row.rep_text) with piece_role = Some "warichu-bracket" }
  | None -> fault "Appendix A lists no cl-%02d key to wrap a warichu census in" klass

let warichu_open () : piece = warichu_bracket 1
let warichu_close () : piece = warichu_bracket 2

(** The note a warichu census sets: half-em characters, which is what §3.4.2's own
    "around six point size" comes to against a ten point kihon-hanmen. *)
let note (count : int) : piece list =
  let letters = [| "あ"; "い"; "う"; "え"; "お"; "か" |] in
  List.init count (fun index ->
      {
        (plain letters.(index mod Array.length letters)) with
        piece_size = Some (em / 2, em / 2);
        piece_advance = Some (em / 2);
      })

type construct_variant = {
  cs_name : string;
  cs_shape : string -> string -> piece list * span list;
      (** The pair under test, and what stands between the two of them. *)
  cs_extent : int;
  cs_breaks : breaks;
  cs_alignment : string;
  cs_writing_mode : string;
  cs_settings : (string * string) list;
}

let construct_default =
  {
    cs_name = "";
    cs_shape = (fun _ _ -> ([], []));
    cs_extent = wide_extent;
    cs_breaks = No_break;
    cs_alignment = "start";
    cs_writing_mode = "horizontal-tb";
    cs_settings = [];
  }

(** [before] [body] [after], with [spans] stated over the body's own piece indices.

    [tail] adds one more ordinary character past the pair, so that a variant can put a
    mandatory break after the pair and have the line under test not be the paragraph's
    last one -- which is what §3.5.3 makes the difference between a justified line and
    a flush one. *)
let around ?(tail = false) (body : piece list) (spans : span list) (before : string)
    (after : string) : piece list * span list =
  ((plain before :: (body @ [ plain after ])) @ (if tail then [ plain filler ] else []), spans)

let emphasis ?(mark = "\xe2\x80\xa2") (first : int) (last : int) : span =
  { span_default with span_kind = "emphasis-dots"; span_first = first; span_last = last;
    span_mark = Some mark }

let ornament (kind : string) (first : int) (last : int) (annotation : piece list) : span =
  { span_default with span_kind = kind; span_first = first; span_last = last;
    span_annotation = Some annotation }

let furawake (first : int) (last : int) ~(columns : int) ~(gap : int) : span =
  { span_default with span_kind = "furawake"; span_first = first; span_last = last;
    span_columns = Some (columns, gap) }

let jidori (first : int) (last : int) ~(cells : int) : span =
  { span_default with span_kind = "jidori"; span_first = first; span_last = last;
    span_cells = Some cells }

let construct_variants : construct_variant list =
  let two_bases = ruby_bases 2 in
  let three_bases = ruby_bases 3 in
  let emphasized ?tail mark body =
    around ?tail body [ emphasis ~mark 1 (1 + List.length body) ]
  in
  let ornamented ?tail kind reading body =
    around ?tail body [ ornament kind 1 (1 + List.length body) (ruby_reading reading) ]
  in
  let bracketed ?tail body =
    let inner = List.length body in
    around ?tail ((warichu_open () :: body) @ [ warichu_close () ])
      [ { span_default with span_kind = "warichu"; span_first = 1; span_last = 3 + inner } ]
  in
  let bare body =
    around body [ { span_default with span_kind = "warichu"; span_first = 1;
                    span_last = 1 + List.length body } ]
  in
  let columned body columns gap =
    around body [ furawake 1 (1 + List.length body) ~columns ~gap ]
  in
  let celled body cells = around body [ jidori 1 (1 + List.length body) ~cells ] in
  [
    (* §3.3.9. The mark is half its base character, so a base of a second size is the
       only way to see that the half is taken per character and not once per run. *)
    { construct_default with cs_name = "emphasis"; cs_shape = emphasized "\xe2\x80\xa2" two_bases };
    {
      construct_default with
      cs_name = "emphasis-mixed-size";
      cs_shape =
        emphasized "\xe2\x80\xa2"
          [ List.nth two_bases 0; { (List.nth two_bases 1) with piece_size = Some (em * 3 / 5, em * 3 / 5);
                                    piece_advance = Some (em * 3 / 5) } ];
    };
    {
      construct_default with
      cs_name = "emphasis-vertical";
      cs_shape = emphasized "\xef\xb9\x85" two_bases;
      cs_writing_mode = "vertical-rl";
    };
    {
      construct_default with
      cs_name = "emphasis-justified";
      cs_shape = emphasized ~tail:true "\xe2\x80\xa2" three_bases;
      cs_extent = 6 * em;
      cs_breaks = Mandatory_after 5;
      cs_alignment = "justify";
    };
    {
      construct_default with
      cs_name = "emphasis-reduced";
      cs_shape = emphasized "\xe2\x80\xa2" three_bases;
      cs_extent = 4 * em;
    };
    {
      construct_default with
      cs_name = "emphasis-broken";
      cs_shape = emphasized "\xe2\x80\xa2" two_bases;
      cs_extent = 2 * em;
      cs_breaks = Every_boundary;
    };
    (* §3.7.1. The whole complex is one thing for breaking and for expansion alike,
       so a run of three states two boundaries that neither may touch. *)
    { construct_default with cs_name = "script"; cs_shape = ornamented "script" 1 two_bases };
    {
      construct_default with
      cs_name = "script-long";
      cs_shape = ornamented "script" 4 two_bases;
    };
    {
      construct_default with
      cs_name = "script-vertical";
      cs_shape = ornamented "script" 2 two_bases;
      cs_writing_mode = "vertical-rl";
    };
    {
      construct_default with
      cs_name = "script-justified";
      cs_shape = ornamented ~tail:true "script" 1 three_bases;
      cs_extent = 6 * em;
      cs_breaks = Mandatory_after 5;
      cs_alignment = "justify";
    };
    {
      construct_default with
      cs_name = "script-reduced";
      cs_shape = ornamented "script" 1 three_bases;
      cs_extent = 4 * em;
    };
    {
      construct_default with
      cs_name = "reference-mark";
      cs_shape = ornamented "reference-mark" 1 two_bases;
    };
    {
      construct_default with
      cs_name = "reference-mark-justified";
      cs_shape = ornamented ~tail:true "reference-mark" 2 three_bases;
      cs_extent = 6 * em;
      cs_breaks = Mandatory_after 5;
      cs_alignment = "justify";
    };
    (* §3.4. Four half-em characters divide evenly and five do not, which is where
       "the length of the second line should not be longer than the length of the
       first" stops being satisfiable and starts being a preference. *)
    { construct_default with cs_name = "warichu"; cs_shape = bracketed (note 4) };
    { construct_default with cs_name = "warichu-odd"; cs_shape = bracketed (note 5) };
    {
      construct_default with
      cs_name = "warichu-declared-split";
      cs_shape = bracketed (note 4);
      cs_breaks = Boundaries_before [ 4 ];
    };
    { construct_default with cs_name = "warichu-bare"; cs_shape = bare (note 4) };
    {
      construct_default with
      cs_name = "warichu-vertical";
      cs_shape = bracketed (note 4);
      cs_writing_mode = "vertical-rl";
    };
    {
      construct_default with
      cs_name = "warichu-reduced";
      cs_shape = bracketed (note 4);
      cs_extent = 3 * em;
    };
    {
      construct_default with
      cs_name = "warichu-justified";
      cs_shape = bracketed ~tail:true (note 4);
      cs_extent = 7 * em;
      cs_breaks = Mandatory_after 8;
      cs_alignment = "justify";
    };
    {
      construct_default with
      cs_name = "warichu-straddled";
      cs_shape = bracketed (note 6);
      cs_extent = 3 * em;
      cs_breaks = Boundaries_before [ 3; 4; 5; 6 ];
    };
    (* §3.7.2. The block is centered across the line and its own height is the line's,
       which a gap and an odd column count make visible at once. *)
    {
      construct_default with
      cs_name = "furawake-two";
      cs_shape = (fun before after -> columned three_bases 2 (em / 5) before after);
      cs_breaks = Boundaries_before [ 2 ];
    };
    {
      construct_default with
      cs_name = "furawake-three";
      cs_shape = (fun before after -> columned three_bases 3 (em / 5) before after);
      cs_breaks = Boundaries_before [ 2; 3 ];
    };
    {
      construct_default with
      cs_name = "furawake-vertical";
      cs_shape = (fun before after -> columned three_bases 2 (em / 5) before after);
      cs_breaks = Boundaries_before [ 2 ];
      cs_writing_mode = "vertical-rl";
    };
    {
      construct_default with
      cs_name = "furawake-solid";
      cs_shape = (fun before after -> columned three_bases 2 0 before after);
      cs_breaks = Boundaries_before [ 3 ];
    };
    (* §3.7.3. Two characters in four cells, three in five: the first divides its
       surplus over one boundary and the second over two, and a boundary the
       specification calls unbreakable takes none of it. *)
    { construct_default with cs_name = "jidori-two-in-four";
      cs_shape = (fun before after -> celled two_bases 4 before after) };
    { construct_default with cs_name = "jidori-three-in-five";
      cs_shape = (fun before after -> celled three_bases 5 before after) };
    {
      construct_default with
      cs_name = "jidori-remainder-trailing";
      cs_shape = (fun before after -> celled three_bases 5 before after);
      cs_settings = [ ("adjustment.remainder", "trailing") ];
    };
    { construct_default with cs_name = "jidori-one-in-three";
      cs_shape = (fun before after -> celled (ruby_bases 1) 3 before after) };
    {
      construct_default with
      cs_name = "jidori-vertical";
      cs_shape = (fun before after -> celled three_bases 5 before after);
      cs_writing_mode = "vertical-rl";
    };
  ]

let constructs_census (emit : string -> Jlreq_proto.Json.t -> unit) : unit =
  each_pair (fun before after ->
      List.iter
        (fun variant ->
          let pieces, spans = variant.cs_shape before.rep_text after.rep_text in
          emit
            (Printf.sprintf "census/constructs/%s+%s/%s" before.rep_label after.rep_label
               variant.cs_name)
            (request ~pieces ~line_extent:variant.cs_extent ~breaks:variant.cs_breaks
               ~alignment:variant.cs_alignment ~writing_mode:variant.cs_writing_mode ~spans
               ~style:(style variant.cs_settings) ()))
        construct_variants)

(* ----------------------------------------------------------------------------- *)
(* §3.6's tab setting *)
(* ----------------------------------------------------------------------------- *)

(** A tab sign standing between the pair under test.

    Two things meet here that nothing else in this file can put together. §3.6.3
    aligns the text {i after} a tab sign to a stop, so the pair's spacing is set by
    Table 1 and then moved bodily along the line, and a census that never varies
    where the stop is never finds out whether the two are being computed in the
    right order. And §3.6.3's last case -- "if there is no tab position
    corresponding to the target string, the string should be set from the tab
    position of the next line" -- turns a tab sign into a place the line may end,
    which no break the caller states can reach and no other census exercises: what
    ends the line is the arithmetic of the stops, so it moves with the pair.

    Every variant states at least as many stops as it holds tab signs, and every
    stop is inside the measure, because a request that breaks either rule is
    refused by both engines and a refused request measures nothing. *)
type tab_variant = {
  tb_name : string;
  tb_pieces : string -> string -> piece list;
  tb_spans : span list;  (** Constructs over the variant's own piece indices. *)
  tb_stops : string -> string -> tab_stop list;
  tb_extent : int;
  tb_breaks : breaks;
  tb_alignment : string;
  tb_state_alignment : bool;
  tb_writing_mode : string;
  tb_indent : int;
  tb_settings : (string * string) list;
}

let tab_default =
  {
    tb_name = "";
    tb_pieces = (fun _ _ -> []);
    tb_spans = [];
    tb_stops = (fun _ _ -> []);
    tb_extent = wide_extent;
    tb_breaks = No_break;
    tb_alignment = "start";
    tb_state_alignment = true;
    tb_writing_mode = "horizontal-tb";
    tb_indent = 0;
    tb_settings = [];
  }

let tab : piece = plain "\t"

let tab_variants : tab_variant list =
  (* The pair with a tab sign between its two halves: whatever Table 1 puts at that
     boundary is displaced by the stop, and whatever it puts at the two outer
     boundaries is not. *)
  let split before after = [ plain before; tab; plain after ] in
  let split_padded before after = [ plain filler; plain before; tab; plain after; plain filler ] in
  let pair_then_tab before after = [ plain before; plain after; tab; plain filler ] in
  let tab_then_pair before after = [ tab; plain before; plain after ] in
  let two_tabs before after = [ plain before; tab; plain after; tab; plain filler ] in
  let trailing_tab before after = [ plain before; plain after; tab ] in
  [
    (* Found stops, one per §3.6.2 alignment. The stop is past where the text before
       the sign ends, so the sign moves the rest of the line forward. *)
    { tab_default with tb_name = "start-found"; tb_pieces = split; tb_stops = (fun _ _ -> [ stop (4 * em) ]) };
    {
      tab_default with
      tb_name = "center-found";
      tb_pieces = split;
      tb_stops = (fun _ _ -> [ stop ~alignment:"center" (4 * em) ]);
    };
    {
      tab_default with
      tb_name = "end-found";
      tb_pieces = split;
      tb_stops = (fun _ _ -> [ stop ~alignment:"end" (4 * em) ]);
    };
    (* §3.6.2's fourth kind aligns on a character of the target string. The pair's
       second half is that character, so the stop lands on a coordinate that moves
       with the pair rather than on a fixed one. *)
    {
      tab_default with
      tb_name = "character-found";
      tb_pieces = (fun before after -> [ plain before; tab; plain after; plain filler ]);
      tb_stops = (fun _ _ -> [ stop ~alignment:"character" ~character:filler (4 * em) ]);
    };
    {
      tab_default with
      tb_name = "character-is-the-pair";
      tb_pieces = (fun before after -> [ plain filler; tab; plain before; plain after ]);
      tb_stops = (fun _ after -> [ stop ~alignment:"character" ~character:after (4 * em) ]);
    };
    (* A stop the line has already passed. §3.6.3's fourth case sends the sign and
       everything after it to the next line, whatever Table 2 says about the
       boundary the line now ends at. *)
    { tab_default with tb_name = "exhausted"; tb_pieces = split; tb_stops = (fun _ _ -> [ stop (em / 2) ]) };
    {
      tab_default with
      tb_name = "exhausted-padded";
      tb_pieces = split_padded;
      tb_stops = (fun _ _ -> [ stop (em / 2) ]);
    };
    {
      tab_default with
      tb_name = "exhausted-center";
      tb_pieces = split;
      tb_stops = (fun _ _ -> [ stop ~alignment:"center" (em / 2) ]);
    };
    {
      tab_default with
      tb_name = "exhausted-end";
      tb_pieces = split;
      tb_stops = (fun _ _ -> [ stop ~alignment:"end" (em / 2) ]);
    };
    {
      tab_default with
      tb_name = "exhausted-character";
      tb_pieces = (fun before after -> [ plain before; tab; plain after; plain filler ]);
      tb_stops = (fun _ _ -> [ stop ~alignment:"character" ~character:filler (em / 2) ]);
    };
    (* The sign at the line head has nowhere earlier to go, so a stop it has already
       passed is the one case §3.6.3's fourth sentence cannot answer. *)
    {
      tab_default with
      tb_name = "head-tab-found";
      tb_pieces = tab_then_pair;
      tb_stops = (fun _ _ -> [ stop (2 * em) ]);
    };
    {
      tab_default with
      tb_name = "head-tab-exhausted-by-indent";
      tb_pieces = tab_then_pair;
      tb_stops = (fun _ _ -> [ stop em ]);
      tb_indent = 3 * em;
    };
    (* Nothing follows the sign, so the stop aligns an empty string. *)
    { tab_default with tb_name = "trailing-tab"; tb_pieces = trailing_tab;
      tb_stops = (fun _ _ -> [ stop (4 * em) ]) };
    {
      tab_default with
      tb_name = "trailing-tab-exhausted";
      tb_pieces = trailing_tab;
      tb_stops = (fun _ _ -> [ stop (em / 2) ]);
    };
    (* The pair itself is the target string, so Table 1's cell between the two is
       computed after the stop has moved them both. *)
    { tab_default with tb_name = "pair-after-tab"; tb_pieces = tab_then_pair;
      tb_stops = (fun _ _ -> [ stop (3 * em) ]) };
    { tab_default with tb_name = "pair-before-tab"; tb_pieces = pair_then_tab;
      tb_stops = (fun _ _ -> [ stop (5 * em) ]) };
    (* Two signs. The second one's stop is the second in the list, and a line that
       ends at the second sign starts the list again -- which is the only way to see
       that the stops are consumed per line and not per paragraph. *)
    {
      tab_default with
      tb_name = "two-tabs-both-found";
      tb_pieces = two_tabs;
      tb_stops = (fun _ _ -> [ stop (3 * em); stop (6 * em) ]);
    };
    {
      tab_default with
      tb_name = "two-tabs-second-exhausted";
      tb_pieces = two_tabs;
      tb_stops = (fun _ _ -> [ stop (3 * em); stop (3 * em / 2) ]);
    };
    {
      tab_default with
      tb_name = "two-tabs-both-exhausted";
      tb_pieces = two_tabs;
      tb_stops = (fun _ _ -> [ stop (em / 2); stop (em / 4) ]);
    };
    (* Stops written down out of order. Which stop a sign takes is a question about
       the line, and the line knows only where they are. *)
    {
      tab_default with
      tb_name = "two-tabs-descending-stops";
      tb_pieces = two_tabs;
      tb_stops = (fun _ _ -> [ stop (6 * em); stop (3 * em) ]);
    };
    {
      tab_default with
      tb_name = "surplus-stops";
      tb_pieces = split;
      tb_stops = (fun _ _ -> [ stop (2 * em); stop (4 * em); stop (6 * em); stop (7 * em) ]);
    };
    (* The measure is what makes a stop reachable, so a tight one changes which of
       §3.6.3's four cases the same stop falls under. *)
    { tab_default with tb_name = "tight-found"; tb_pieces = split; tb_stops = (fun _ _ -> [ stop (2 * em) ]);
      tb_extent = 5 * em };
    { tab_default with tb_name = "tight-exhausted"; tb_pieces = split_padded;
      tb_stops = (fun _ _ -> [ stop (3 * em / 2) ]); tb_extent = 4 * em };
    { tab_default with tb_name = "narrow-overruns"; tb_pieces = split;
      tb_stops = (fun _ _ -> [ stop (3 * em / 2) ]); tb_extent = 2 * em };
    (* A caller who states breaks of their own states none at the sign: §3.6.3's cut
       has to be found whether or not it is also an opportunity the caller named. *)
    { tab_default with tb_name = "stated-breaks-found"; tb_pieces = split_padded;
      tb_stops = (fun _ _ -> [ stop (4 * em) ]); tb_breaks = Every_boundary; tb_extent = 6 * em };
    { tab_default with tb_name = "stated-breaks-exhausted"; tb_pieces = split_padded;
      tb_stops = (fun _ _ -> [ stop (em / 2) ]); tb_breaks = Every_boundary; tb_extent = 6 * em };
    { tab_default with tb_name = "mandatory-break-before"; tb_pieces = split_padded;
      tb_stops = (fun _ _ -> [ stop (em / 2) ]); tb_breaks = Mandatory_after 1 };
    (* §3.8's adjustment and §3.6's stops on the same line: the space a justified
       line takes up is measured after the sign has moved the text along it. *)
    { tab_default with tb_name = "justified-found"; tb_pieces = split_padded;
      tb_stops = (fun _ _ -> [ stop (2 * em) ]); tb_extent = 7 * em; tb_breaks = Mandatory_after 4;
      tb_alignment = "justify" };
    { tab_default with tb_name = "justified-exhausted"; tb_pieces = split_padded;
      tb_stops = (fun _ _ -> [ stop (em / 2) ]); tb_extent = 7 * em; tb_alignment = "justify" };
    { tab_default with tb_name = "centered"; tb_pieces = split; tb_stops = (fun _ _ -> [ stop (3 * em) ]);
      tb_extent = 8 * em; tb_alignment = "center" };
    { tab_default with tb_name = "end-aligned"; tb_pieces = split; tb_stops = (fun _ _ -> [ stop (3 * em) ]);
      tb_extent = 8 * em; tb_alignment = "end" };
    (* A request that states no alignment at all. What such a caller gets is written
       down nowhere, and it is not the same answer as `start`. *)
    { tab_default with tb_name = "alignment-omitted"; tb_pieces = split_padded;
      tb_stops = (fun _ _ -> [ stop (2 * em) ]); tb_extent = 7 * em; tb_state_alignment = false;
      tb_breaks = Mandatory_after 4 };
    { tab_default with tb_name = "alignment-omitted-exhausted"; tb_pieces = split_padded;
      tb_stops = (fun _ _ -> [ stop (em / 2) ]); tb_extent = 7 * em; tb_state_alignment = false };
    (* §3.6.2 names its stops for horizontal composition and §3.6.1's figure sets
       them along the line, so the same four answers have to come back in a vertical
       paragraph, where the line runs the other way. *)
    { tab_default with tb_name = "vertical-found"; tb_pieces = split;
      tb_stops = (fun _ _ -> [ stop (4 * em) ]); tb_writing_mode = "vertical-rl" };
    { tab_default with tb_name = "vertical-exhausted"; tb_pieces = split;
      tb_stops = (fun _ _ -> [ stop (em / 2) ]); tb_writing_mode = "vertical-rl" };
    { tab_default with tb_name = "vertical-two-tabs"; tb_pieces = two_tabs;
      tb_stops = (fun _ _ -> [ stop (3 * em); stop (3 * em / 2) ]); tb_writing_mode = "vertical-rl" };
    (* §3.5.2's indent moves the line head, and a stop is a position in the line and
       not a distance from the text. *)
    { tab_default with tb_name = "indented-found"; tb_pieces = split;
      tb_stops = (fun _ _ -> [ stop (5 * em) ]); tb_indent = 2 * em };
    { tab_default with tb_name = "indented-exhausted"; tb_pieces = split;
      tb_stops = (fun _ _ -> [ stop (3 * em / 2) ]); tb_indent = 2 * em };
    (* Where the line has to give space back, §3.8.3's ladder and §3.6.3's stop are
       both trying to decide where the same character sits. *)
    { tab_default with tb_name = "reduced"; tb_pieces = split_padded;
      tb_stops = (fun _ _ -> [ stop (3 * em) ]); tb_extent = 4 * em };
    { tab_default with tb_name = "reduced-trailing-remainder"; tb_pieces = split_padded;
      tb_stops = (fun _ _ -> [ stop (3 * em) ]); tb_extent = 4 * em;
      tb_settings = [ ("adjustment.remainder", "trailing") ] };
    (* A sign inside a construct. §3.6.3's cut is a line boundary rather than a break
       opportunity, so what decides whether it is available is not a rule about
       characters but whether there is a boundary there at all -- and inside one
       object there is not. The stop is one the line has gone past in each of these,
       which is the only case where the difference shows. *)
    { tab_default with tb_name = "inside-tate-chu-yoko"; tb_pieces = split;
      tb_spans = [ tate_chu_yoko 0 3 ]; tb_stops = (fun _ _ -> [ stop (em / 2) ]);
      tb_extent = 3 * em; tb_writing_mode = "vertical-rl" };
    { tab_default with tb_name = "inside-tate-chu-yoko-found"; tb_pieces = split;
      tb_spans = [ tate_chu_yoko 0 3 ]; tb_stops = (fun _ _ -> [ stop (3 * em) ]);
      tb_extent = 6 * em; tb_writing_mode = "vertical-rl" };
    { tab_default with tb_name = "inside-emphasis"; tb_pieces = split;
      tb_spans = [ emphasis 0 3 ]; tb_stops = (fun _ _ -> [ stop (em / 2) ]);
      tb_extent = 3 * em };
    { tab_default with tb_name = "inside-script"; tb_pieces = split;
      tb_spans = [ ornament "script" 0 3 (ruby_reading 1) ];
      tb_stops = (fun _ _ -> [ stop (em / 2) ]); tb_extent = 3 * em };
    { tab_default with tb_name = "inside-jidori"; tb_pieces = split;
      tb_spans = [ jidori 0 3 ~cells:5 ]; tb_stops = (fun _ _ -> [ stop (em / 2) ]);
      tb_extent = 6 * em };
    (* A construct that ends exactly at the sign: the sign is beside the construct
       rather than in it, so it is a sign of the line and §3.6.3's cut is available. *)
    { tab_default with tb_name = "construct-ends-at-the-sign"; tb_pieces = split;
      tb_spans = [ tate_chu_yoko 0 1 ]; tb_stops = (fun _ _ -> [ stop (em / 2) ]);
      tb_extent = 3 * em; tb_writing_mode = "vertical-rl" };
  ]

let tabs_census (emit : string -> Jlreq_proto.Json.t -> unit) : unit =
  each_pair (fun before after ->
      List.iter
        (fun variant ->
          emit
            (Printf.sprintf "census/tabs/%s+%s/%s" before.rep_label after.rep_label variant.tb_name)
            (request
               ~pieces:(variant.tb_pieces before.rep_text after.rep_text)
               ~line_extent:variant.tb_extent ~breaks:variant.tb_breaks
               ~alignment:variant.tb_alignment ~state_alignment:variant.tb_state_alignment
               ~writing_mode:variant.tb_writing_mode ~first_line_indent:variant.tb_indent
               ~spans:variant.tb_spans ~tab_stops:(variant.tb_stops before.rep_text after.rep_text)
               ~style:(style variant.tb_settings) ()))
        tab_variants)

(* ----------------------------------------------------------------------------- *)
(* §3.5.4's paragraph end, and §3.5.3's four answers *)
(* ----------------------------------------------------------------------------- *)

(** A paragraph whose last line is short enough to be a widow, walked past every
    class pair.

    §3.5.4 is one sentence -- "avoid that the last line of a paragraph contains less
    than a given number of characters" -- and it says nothing about what avoiding it
    costs, which line gives the character up, or what the line left behind then looks
    like. The last of those is the one a class pair can answer: a paragraph that
    breaks a line early to keep the widow away has left that line short, and §3.8.1
    then adjusts it back out to the measure at whatever boundaries Table 6 offers.
    The pair {i is} those boundaries, so the same widow decision comes out differently
    at every one of the 529 cells.

    The alignment axis is here for the same reason. §3.5.3's four answers and the
    request that states none are five different things to do with a line that did not
    fill the measure, and which one an unstated alignment means is written down
    nowhere at all. *)
type widow_variant = {
  wd_name : string;
  wd_pieces : string -> string -> piece list;
  wd_extent : int;
  wd_minimum : int option;
  wd_breaks : breaks;
  wd_alignment : string;
  wd_state_alignment : bool;
  wd_writing_mode : string;
  wd_indent : int;
  wd_settings : (string * string) list;
}

let widow_default =
  {
    wd_name = "";
    wd_pieces = (fun _ _ -> []);
    wd_extent = 4 * em;
    wd_minimum = None;
    wd_breaks = Every_boundary;
    wd_alignment = "start";
    wd_state_alignment = true;
    wd_writing_mode = "horizontal-tb";
    wd_indent = 0;
    wd_settings = [];
  }

let widow_variants : widow_variant list =
  (* Five clusters in a four-em measure: greedy filling leaves one on the last line,
     and keeping two there costs the first line one cluster. The pair stands at the
     boundary the first line has to open up. *)
  let five before after = [ plain before; plain after; plain filler; plain filler; plain filler ] in
  let five_pair_last before after =
    [ plain filler; plain filler; plain filler; plain before; plain after ]
  in
  let three before after = [ plain before; plain after; plain filler ] in
  [
    (* The same paragraph with and without the constraint: what §3.5.4 changed. *)
    { widow_default with wd_name = "unconstrained"; wd_pieces = five };
    { widow_default with wd_name = "minimum-one"; wd_pieces = five; wd_minimum = Some 1 };
    { widow_default with wd_name = "minimum-two"; wd_pieces = five; wd_minimum = Some 2 };
    { widow_default with wd_name = "minimum-three"; wd_pieces = five; wd_minimum = Some 3 };
    (* More than the paragraph holds, so the constraint cannot be met and whatever
       the engine reports it reports on every line it could have chosen. *)
    { widow_default with wd_name = "minimum-unreachable"; wd_pieces = five; wd_minimum = Some 5 };
    { widow_default with wd_name = "minimum-past-the-text"; wd_pieces = three;
      wd_minimum = Some 4; wd_extent = 2 * em };
    (* The pair on the last line rather than the first: the clusters the constraint
       counts are the pair itself. *)
    { widow_default with wd_name = "pair-on-last-line"; wd_pieces = five_pair_last;
      wd_minimum = Some 2 };
    { widow_default with wd_name = "pair-on-last-line-three"; wd_pieces = five_pair_last;
      wd_minimum = Some 3 };
    (* Nowhere to move the break to. *)
    { widow_default with wd_name = "no-opportunity"; wd_pieces = five; wd_minimum = Some 2;
      wd_breaks = No_break };
    { widow_default with wd_name = "one-opportunity"; wd_pieces = five; wd_minimum = Some 2;
      wd_breaks = Boundaries_before [ 4 ] };
    { widow_default with wd_name = "opportunity-before-the-pair"; wd_pieces = five;
      wd_minimum = Some 2; wd_breaks = Boundaries_before [ 1; 3 ] };
    (* §3.5.3's four answers to a line that did not fill the measure, and the request
       that gives none. *)
    { widow_default with wd_name = "justified"; wd_pieces = five; wd_minimum = Some 2;
      wd_alignment = "justify" };
    { widow_default with wd_name = "centered"; wd_pieces = five; wd_minimum = Some 2;
      wd_alignment = "center" };
    { widow_default with wd_name = "end-aligned"; wd_pieces = five; wd_minimum = Some 2;
      wd_alignment = "end" };
    { widow_default with wd_name = "alignment-omitted"; wd_pieces = five; wd_minimum = Some 2;
      wd_state_alignment = false };
    { widow_default with wd_name = "alignment-omitted-unconstrained"; wd_pieces = five;
      wd_state_alignment = false };
    { widow_default with wd_name = "alignment-omitted-short-last-line"; wd_pieces = three;
      wd_extent = 2 * em; wd_state_alignment = false };
    (* The style's own preferences decide between two break sets that the widow rule
       has made equally attractive. *)
    { widow_default with wd_name = "even-texture"; wd_pieces = five; wd_minimum = Some 2;
      wd_settings = [ ("adjustment.preference", "even-texture") ] };
    { widow_default with wd_name = "trailing-remainder"; wd_pieces = five; wd_minimum = Some 2;
      wd_settings = [ ("adjustment.remainder", "trailing") ] };
    (* §3.5.1's indent takes a cluster off the first line, which moves the widow
       decision one boundary along. *)
    { widow_default with wd_name = "indented"; wd_pieces = five; wd_minimum = Some 2;
      wd_indent = em };
    { widow_default with wd_name = "indented-omitted-alignment"; wd_pieces = five;
      wd_minimum = Some 2; wd_indent = em; wd_state_alignment = false };
    (* The other writing mode. *)
    { widow_default with wd_name = "vertical"; wd_pieces = five; wd_minimum = Some 2;
      wd_writing_mode = "vertical-rl" };
    { widow_default with wd_name = "vertical-omitted-alignment"; wd_pieces = five;
      wd_minimum = Some 2; wd_writing_mode = "vertical-rl"; wd_state_alignment = false };
    (* A wider measure, where three lines rather than two are in play. *)
    { widow_default with wd_name = "narrow-three-lines"; wd_pieces = five; wd_minimum = Some 2;
      wd_extent = 2 * em };
    { widow_default with wd_name = "narrow-three-lines-omitted"; wd_pieces = five;
      wd_minimum = Some 2; wd_extent = 2 * em; wd_state_alignment = false };
  ]

let widow_census (emit : string -> Jlreq_proto.Json.t -> unit) : unit =
  each_pair (fun before after ->
      List.iter
        (fun variant ->
          emit
            (Printf.sprintf "census/widow/%s+%s/%s" before.rep_label after.rep_label variant.wd_name)
            (request
               ~pieces:(variant.wd_pieces before.rep_text after.rep_text)
               ~line_extent:variant.wd_extent ~breaks:variant.wd_breaks
               ~alignment:variant.wd_alignment ~state_alignment:variant.wd_state_alignment
               ~writing_mode:variant.wd_writing_mode ~first_line_indent:variant.wd_indent
               ?widow:variant.wd_minimum ~style:(style variant.wd_settings) ()))
        widow_variants)

type kind = {
  kind_name : string;
  kind_summary : string;
  kind_emit : (string -> Jlreq_proto.Json.t -> unit) -> unit;
}

(** The registry. A census is a name, a sentence and a function that emits
    requests; nothing else in this file knows how many there are. *)
let kinds =
  [
    {
      kind_name = "spacing";
      kind_summary = "every ordered class pair on a line too wide to break or adjust (Table 1)";
      kind_emit = spacing_census;
    };
    {
      kind_name = "break";
      kind_summary = "every ordered class pair on a one-cluster line, at all four levels (Table 2)";
      kind_emit = break_census;
    };
    {
      kind_name = "reduction";
      kind_summary = "every ordered class pair on a line too narrow for its own spacing (Tables 3-5)";
      kind_emit = reduction_census;
    };
    {
      kind_name = "expansion";
      kind_summary = "every ordered class pair on a justified line with room left over (Table 6)";
      kind_emit = expansion_census;
    };
    {
      kind_name = "vertical";
      kind_summary = "every ordered class pair in a vertical line, upright, rotated and roled";
      kind_emit = vertical_census;
    };
    {
      kind_name = "tate-chu-yoko";
      kind_summary = "every ordered class pair beside a tate-chu-yoko run, and two runs beside each other";
      kind_emit = tate_chu_yoko_census;
    };
    {
      kind_name = "ruby";
      kind_summary = "every ordered class pair beside a ruby construct, in all three kinds";
      kind_emit = ruby_census;
    };
    {
      kind_name = "constructs";
      kind_summary =
        "every ordered class pair beside emphasis dots, a superscript, a reference mark, a \
         warichu, a furawake and a jidori";
      kind_emit = constructs_census;
    };
    {
      kind_name = "tabs";
      kind_summary =
        "every ordered class pair across a tab sign, at stops the line reaches and stops it \
         has passed (§3.6)";
      kind_emit = tabs_census;
    };
    {
      kind_name = "widow";
      kind_summary =
        "every ordered class pair on a paragraph whose last line is a widow, at every \
         alignment and at none (§3.5.3, §3.5.4)";
      kind_emit = widow_census;
    };
  ]

(* ----------------------------------------------------------------------------- *)
(* Reading a response stream back *)
(* ----------------------------------------------------------------------------- *)

(** The same value with every object's keys in name order.

    Two engines write the same answer with different key orders -- the Rust side's
    [serde_json] sorts them, this side writes [lines] before [diagnostics] -- and
    key order is not part of an answer: the runner compares parsed values. A
    textual [diff] of two raw response streams would therefore report every line as
    different. Canonicalizing both sides first is what makes [diff] mean what it
    looks like it means. *)
let rec canonical (value : Jlreq_proto.Json.t) : Jlreq_proto.Json.t =
  match value with
  | Jlreq_proto.Json.Object fields ->
    Jlreq_proto.Json.Object
      (List.sort
         (fun (left, _) (right, _) -> String.compare left right)
         (List.map (fun (name, item) -> (name, canonical item)) fields))
  | Jlreq_proto.Json.Array items -> Jlreq_proto.Json.Array (List.map canonical items)
  | other -> other

let normalize () : unit =
  let number = ref 0 in
  let rec loop () =
    match input_line stdin with
    | exception End_of_file -> ()
    | line ->
      incr number;
      if String.trim line <> "" then begin
        match Jlreq_proto.Json.parse line with
        | value ->
          print_string (Jlreq_proto.Json.to_string (canonical value));
          print_char '\n'
        | exception Jlreq_proto.Json.Invalid message -> fault "line %d: %s" !number message
      end;
      loop ()
  in
  loop ()

(* ----------------------------------------------------------------------------- *)
(* The command line *)
(* ----------------------------------------------------------------------------- *)

(** The chosen representatives, as a TSV a reviewer can read next to Appendix A. *)
let print_classes () : unit =
  print_string "# class\tkey\tcharacter\tclasses-listing-the-key\tclassified-as\n";
  List.iter
    (fun row ->
      (* The class the census {i addresses} and the class §3.9.2 gives the occurrence
         are not always the same: a key several classes list, or one whose Appendix A
         listing states an advance the census's own full-em frame contradicts,
         resolves somewhere else. A census that could not say where would be
         measuring a coordinate it cannot name. *)
      let resolved =
        Jlreq.Spec.class_of ~piece:row.rep_text ~frame:Jlreq.Model.Full_em ~role:None
          ~writing_mode:Jlreq.Model.Horizontal_tb ~unlisted_is_ideographic:false
          ~highest_ambiguous_class:false ~grouped_numeral_requires_role:false
      in
      Printf.printf "%s\t%s\t%s\t%d\t%s\n" row.rep_label row.rep_key row.rep_text
        row.rep_ambiguity (Jlreq.Tables.row_label resolved))
    representatives;
  for value = 1 to 30 do
    if Jlreq.Tables.has_adjacency value && not (List.exists (fun row -> row.rep_class = value) representatives)
    then
      Printf.printf "# %s\tno Appendix A listing: reachable only inside a construct\n"
        (Jlreq.Tables.row_label value)
  done

let usage () =
  let buffer = Buffer.create 512 in
  Buffer.add_string buffer
    "usage: census generate <kind>   one NDJSON request envelope per line, on stdout\n";
  Buffer.add_string buffer
    "       census classes           the representative code point chosen for each class, as TSV\n";
  Buffer.add_string buffer
    "       census normalize         a response stream on stdin, with every object's keys sorted,\n\
    \                                so that `diff` means what it looks like it means\n";
  Buffer.add_string buffer "\nkinds:\n";
  List.iter
    (fun kind ->
      Buffer.add_string buffer (Printf.sprintf "  %-10s %s\n" kind.kind_name kind.kind_summary))
    kinds;
  Buffer.contents buffer

let run (arguments : string list) : unit =
  match arguments with
  | [ "classes" ] -> print_classes ()
  | [ "normalize" ] -> normalize ()
  | [ "generate"; name ] -> (
    match List.find_opt (fun kind -> String.equal kind.kind_name name) kinds with
    | None -> fault "`%s` is not a census\n%s" name (usage ())
    | Some kind ->
      kind.kind_emit (fun id request ->
          print_string (Jlreq_proto.Json.to_string (envelope ~id ~request));
          print_char '\n'))
  | _ -> fault "%s" (usage ())

let () =
  set_binary_mode_in stdin true;
  set_binary_mode_out stdout true;
  (* The representatives are read out of the embedded tables, so a build that
     pasted the wrong file would generate a plausible-looking wrong census. The
     engine's own startup census is the check for that, and it costs nothing here. *)
  let status =
    try
      Jlreq.Tables.self_check ();
      run (List.tl (Array.to_list Sys.argv));
      0
    with
    | Fault message ->
      prerr_endline (program ^ ": " ^ message);
      2
    | Jlreq.Tables.Invalid message ->
      prerr_endline (program ^ ": specification tables: " ^ message);
      2
    | Jlreq.Tsv.Invalid message ->
      prerr_endline (program ^ ": specification tables: " ^ message);
      2
    | Jlreq_proto.Json.Invalid message ->
      prerr_endline (program ^ ": " ^ message);
      2
  in
  flush stdout;
  exit status
