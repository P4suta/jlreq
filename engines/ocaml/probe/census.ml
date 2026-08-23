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

(** One construct over a half-open range of {i piece} indices. The census names the
    clusters it means and this is where they become the byte offsets the protocol
    carries. *)
type span = {
  span_kind : string;
  span_first : int;
  span_last : int;  (** One past the last piece the construct covers. *)
}

let tate_chu_yoko (first : int) (last : int) : span =
  { span_kind = "tate-chu-yoko"; span_first = first; span_last = last }

(** A request whose clusters are [pieces], one em each unless a piece states its own.

    Everything a census does not vary is fixed here: a full-em frame and one cluster
    per piece whose advance is its em. *)
let request ~(pieces : piece list) ~(line_extent : int) ~(breaks : breaks)
    ~(alignment : string) ?(writing_mode = "horizontal-tb") ?(spans : span list = [])
    ~(style : Jlreq_proto.Json.t) () : Jlreq_proto.Json.t =
  let source = String.concat "" (List.map (fun piece -> piece.piece_text) pieces) in
  let starts = Array.make (List.length pieces + 1) 0 in
  let clusters, boundaries, _ =
    List.fold_left
      (fun (clusters, boundaries, start) piece ->
        let index = List.length clusters in
        starts.(index) <- start;
        let stop = start + String.length piece.piece_text in
        starts.(index + 1) <- stop;
        let advance = Option.value ~default:em piece.piece_advance in
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
        let boundaries = if start = 0 then boundaries else start :: boundaries in
        (cluster :: clusters, boundaries, stop))
      ([], [], 0) pieces
  in
  let boundaries = List.rev boundaries in
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
                 Jlreq_proto.Json.Object
                   [
                     ("kind", Jlreq_proto.Json.String span.span_kind);
                     ( "range",
                       Jlreq_proto.Json.Array
                         [
                           Jlreq_proto.Json.of_int starts.(span.span_first);
                           Jlreq_proto.Json.of_int starts.(span.span_last);
                         ] );
                   ])
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
  Jlreq_proto.Json.Object
    ([
       ("source", Jlreq_proto.Json.String source);
       ("size", size);
       ("frame", Jlreq_proto.Json.String "full-em");
       ("clusters", Jlreq_proto.Json.Array (List.rev clusters));
       ("line_extent", Jlreq_proto.Json.of_int line_extent);
     ]
    @ breaks @ constructs
    @ [
        ("alignment", Jlreq_proto.Json.String alignment);
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
  print_string "# class\tkey\tcharacter\tclasses-listing-the-key\n";
  List.iter
    (fun row ->
      Printf.printf "%s\t%s\t%s\t%d\n" row.rep_label row.rep_key row.rep_text row.rep_ambiguity)
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
