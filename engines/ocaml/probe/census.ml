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
    just census spacing     # 2116 requests, both engines, one diff
    just census break
    just census-classes     # the representative code point chosen for each class
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
    where their census belongs.

    {1 Adding the reduction and expansion censuses}

    {!kinds} is the whole registry: a census is a name, a sentence, and a function
    that emits requests. M2's reduction census and M3's expansion census are one
    entry each -- the same pairs at a line extent that forces the paragraph to give
    back or take up space -- and no other part of this file has to know.

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

(** A request whose clusters are [texts], one em each.

    [allow_breaks] marks every internal cluster boundary [allowed]; the schema
    refuses offset zero and the end of the source, so the boundaries are exactly
    the starts of the second and later clusters. *)
let request ~(texts : string list) ~(line_extent : int) ~(allow_breaks : bool)
    ~(style : Jlreq_proto.Json.t) : Jlreq_proto.Json.t =
  let source = String.concat "" texts in
  let clusters, boundaries, _ =
    List.fold_left
      (fun (clusters, boundaries, start) text ->
        let stop = start + String.length text in
        let cluster =
          Jlreq_proto.Json.Object
            [
              ("range", Jlreq_proto.Json.Array [ Jlreq_proto.Json.of_int start; Jlreq_proto.Json.of_int stop ]);
              ("advance", Jlreq_proto.Json.of_int em);
            ]
        in
        let boundaries = if start = 0 then boundaries else start :: boundaries in
        (cluster :: clusters, boundaries, stop))
      ([], [], 0) texts
  in
  let breaks =
    if not allow_breaks then []
    else
      [
        ( "breaks",
          Jlreq_proto.Json.Array
            (List.rev_map
               (fun offset ->
                 Jlreq_proto.Json.Object
                   [
                     ("offset", Jlreq_proto.Json.of_int offset);
                     ("kind", Jlreq_proto.Json.String "allowed");
                   ])
               boundaries) );
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
    @ breaks
    @ [
        ("alignment", Jlreq_proto.Json.String "start");
        ("writing_mode", Jlreq_proto.Json.String "horizontal-tb");
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
            (request ~texts ~line_extent:wide_extent ~allow_breaks:false ~style:(style [])))
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
               ~texts:[ before.rep_text; after.rep_text ]
               ~line_extent:em ~allow_breaks:true
               ~style:(style [ ("kinsoku.level", level) ])))
        kinsoku_levels)

type kind = {
  kind_name : string;
  kind_summary : string;
  kind_emit : (string -> Jlreq_proto.Json.t -> unit) -> unit;
}

(** The registry. M2's reduction census and M3's expansion census are one entry
    each; nothing else in this file changes when they arrive. *)
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
