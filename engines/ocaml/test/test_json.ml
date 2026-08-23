(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** {!Jlreq_proto.Json}: integers only, and null is not absence.

    The two properties this file exists to pin are the two that would otherwise
    fail eighty-nine cases at once with a diff that reads like an arithmetic bug. *)

open Jlreq_proto.Json

let run () =
  (* Integers, and nothing that is not one. *)
  Check.ok "a positive integer" (parse "1" = Int 1L);
  Check.ok "a negative integer" (parse "-1" = Int (-1L));
  Check.ok "zero" (parse "0" = Int 0L);
  Check.ok "i32::MIN" (parse "-2147483648" = Int (-2147483648L));
  Check.ok "i64::MAX" (parse "9223372036854775807" = Int Int64.max_int);
  Check.ok "i64::MIN" (parse "-9223372036854775808" = Int Int64.min_int);
  Check.raises "a decimal point" (fun () -> parse "1.0");
  Check.raises "an exponent" (fun () -> parse "1e3");
  Check.raises "a capital exponent" (fun () -> parse "1E3");
  Check.raises "a fractional exponent" (fun () -> parse "2500.25");
  Check.raises "a leading zero" (fun () -> parse "01");
  Check.raises "a negative leading zero" (fun () -> parse "-01");
  Check.raises "a bare minus" (fun () -> parse "-");
  Check.raises "an integer wider than 64 bits" (fun () -> parse "9223372036854775808");
  Check.ok "zero is not a leading zero" (parse "-0" = Int 0L);

  (* The printer never emits a float, because it cannot construct one. *)
  Check.equal_string "an integer prints bare" ~expected:"2500"
    ~actual:(to_string (Int 2500L));
  Check.equal_string "a negative integer prints bare" ~expected:"-2500"
    ~actual:(to_string (Int (-2500L)));
  Check.equal_string "i64::MIN prints exactly" ~expected:"-9223372036854775808"
    ~actual:(to_string (Int Int64.min_int));

  (* Null is a value; absence is not. *)
  let stated = parse "{\"symbol\":null}" and absent = parse "{}" in
  Check.ok "a stated null reads as Null" (member "symbol" stated = Some Null);
  Check.ok "an absent field reads as None" (member "symbol" absent = None);
  Check.equal_bool "a stated null is present" ~expected:true ~actual:(has "symbol" stated);
  Check.equal_bool "an absent field is absent" ~expected:false ~actual:(has "symbol" absent);
  Check.equal_string "a stated null survives a round trip" ~expected:"{\"symbol\":null}"
    ~actual:(to_string stated);

  (* Objects keep their order and refuse a repeated key. *)
  Check.ok "field order is the order written"
    (names (parse "{\"b\":1,\"a\":2}") = [ "b"; "a" ]);
  Check.raises "a repeated key" (fun () -> parse "{\"a\":1,\"a\":2}");
  Check.ok "the empty object" (parse "{}" = Object []);
  Check.ok "the empty array" (parse "[]" = Array []);
  Check.ok "member on a non-object" (member "a" (Int 1L) = None);

  (* Strings. *)
  Check.ok "a simple string" (parse "\"hi\"" = String "hi");
  Check.ok "an escaped quote" (parse "\"a\\\"b\"" = String "a\"b");
  Check.ok "an escaped solidus" (parse "\"a\\/b\"" = String "a/b");
  Check.ok "the short escapes" (parse "\"\\b\\f\\n\\r\\t\"" = String "\b\012\n\r\t");
  Check.ok "a BMP \\u escape" (parse "\"\\u3042\"" = String "\xe3\x81\x82");
  Check.ok "a surrogate pair" (parse "\"\\ud83d\\ude00\"" = String "\xf0\x9f\x98\x80");
  Check.raises "a lone high surrogate" (fun () -> parse "\"\\ud83d\"");
  Check.raises "a lone low surrogate" (fun () -> parse "\"\\ude00\"");
  Check.raises "an unknown escape" (fun () -> parse "\"\\x\"");
  Check.raises "a raw control character" (fun () -> parse "\"a\tb\"");
  Check.raises "an unterminated string" (fun () -> parse "\"a");
  Check.equal_string "a control character is escaped on the way out"
    ~expected:"\"a\\u0001b\"" ~actual:(to_string (String "a\001b"));
  Check.equal_string "non-ASCII is written as UTF-8, not escaped"
    ~expected:"\"\xe3\x81\x82\"" ~actual:(to_string (String "\xe3\x81\x82"));

  (* Whole messages. *)
  Check.raises "trailing content after the value" (fun () -> parse "{} {}");
  Check.raises "an unclosed object" (fun () -> parse "{\"a\":1");
  Check.raises "an unclosed array" (fun () -> parse "[1,2");
  Check.raises "a trailing comma in an array" (fun () -> parse "[1,]");
  Check.raises "a trailing comma in an object" (fun () -> parse "{\"a\":1,}");
  Check.raises "the empty message" (fun () -> parse "");
  Check.raises "a bare word" (fun () -> parse "nul");
  Check.ok "leading and trailing whitespace" (parse "  \n\t {\"a\":1} \r\n" = Object [ ("a", Int 1L) ]);
  Check.ok "true" (parse "true" = Bool true);
  Check.ok "false" (parse "false" = Bool false);
  Check.ok "null" (parse "null" = Null);

  (* A round trip of the shape the engine actually writes. *)
  let envelope =
    "{\"protocol\":\"jlreq.conformance/1\",\"spec\":\"jlreq-2020-08-11+unicode-17.0.0\",\"id\":\"quick-start/two-lines\",\"response\":{\"lines\":[],\"diagnostics\":[]}}"
  in
  Check.equal_string "a response envelope round-trips byte for byte" ~expected:envelope
    ~actual:(to_string (parse envelope));
  Check.ok "of_int agrees with the parser" (of_int 42 = parse "42")
