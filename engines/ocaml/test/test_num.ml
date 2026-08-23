(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** {!Jlreq.Num}: saturation at the boundaries, and the sign of a division.

    These are the two places the OCaml engine can silently disagree with the Rust
    one. OCaml's native [int] saturates one bit early, so the i64 layer is
    [Int64]; and [usize] subtraction has no floor in OCaml, so it goes through
    [usub]. Both are checked at the exact boundary, because a check at a
    comfortable value proves nothing. *)

open Jlreq.Num

let max64 = Int64.max_int
let min64 = Int64.min_int

let run () =
  (* Saturating addition. *)
  Check.equal_int64 "sadd 2 + 3" ~expected:5L ~actual:(sadd 2L 3L);
  Check.equal_int64 "sadd MAX + 0" ~expected:max64 ~actual:(sadd max64 0L);
  Check.equal_int64 "sadd MAX + 1 saturates" ~expected:max64 ~actual:(sadd max64 1L);
  Check.equal_int64 "sadd MAX + MAX saturates" ~expected:max64 ~actual:(sadd max64 max64);
  Check.equal_int64 "sadd MIN + -1 saturates" ~expected:min64 ~actual:(sadd min64 (-1L));
  Check.equal_int64 "sadd MIN + MIN saturates" ~expected:min64 ~actual:(sadd min64 min64);
  Check.equal_int64 "sadd MIN + MAX is -1" ~expected:(-1L) ~actual:(sadd min64 max64);
  Check.equal_int64 "sadd MAX-1 + 1" ~expected:max64 ~actual:(sadd (Int64.sub max64 1L) 1L);

  (* Saturating subtraction. *)
  Check.equal_int64 "ssub 2 - 3" ~expected:(-1L) ~actual:(ssub 2L 3L);
  Check.equal_int64 "ssub MIN - 1 saturates" ~expected:min64 ~actual:(ssub min64 1L);
  Check.equal_int64 "ssub MAX - -1 saturates" ~expected:max64 ~actual:(ssub max64 (-1L));
  Check.equal_int64 "ssub 0 - MIN saturates" ~expected:max64 ~actual:(ssub 0L min64);
  Check.equal_int64 "ssub MIN - MIN is zero" ~expected:0L ~actual:(ssub min64 min64);
  Check.equal_int64 "ssub MAX - MAX is zero" ~expected:0L ~actual:(ssub max64 max64);

  (* Saturating multiplication. *)
  Check.equal_int64 "smul 6 * 7" ~expected:42L ~actual:(smul 6L 7L);
  Check.equal_int64 "smul -6 * 7" ~expected:(-42L) ~actual:(smul (-6L) 7L);
  Check.equal_int64 "smul anything * 0" ~expected:0L ~actual:(smul min64 0L);
  Check.equal_int64 "smul MAX * 2 saturates high" ~expected:max64 ~actual:(smul max64 2L);
  Check.equal_int64 "smul MAX * -2 saturates low" ~expected:min64 ~actual:(smul max64 (-2L));
  Check.equal_int64 "smul MIN * -1 saturates high" ~expected:max64 ~actual:(smul min64 (-1L));
  Check.equal_int64 "smul -1 * MIN saturates high" ~expected:max64 ~actual:(smul (-1L) min64);
  Check.equal_int64 "smul MIN * 2 saturates low" ~expected:min64 ~actual:(smul min64 2L);
  Check.equal_int64 "smul MIN * MIN saturates high" ~expected:max64 ~actual:(smul min64 min64);
  Check.equal_int64 "smul MAX * 1 is exact" ~expected:max64 ~actual:(smul max64 1L);
  Check.equal_int64 "smul MIN * 1 is exact" ~expected:min64 ~actual:(smul min64 1L);

  (* Saturating negation and absolute value. *)
  Check.equal_int64 "sneg 7" ~expected:(-7L) ~actual:(sneg 7L);
  Check.equal_int64 "sneg MIN saturates" ~expected:max64 ~actual:(sneg min64);
  Check.equal_int64 "sneg MAX" ~expected:(Int64.neg max64) ~actual:(sneg max64);
  Check.equal_int64 "sabs -7" ~expected:7L ~actual:(sabs (-7L));
  Check.equal_int64 "sabs 7" ~expected:7L ~actual:(sabs 7L);
  Check.equal_int64 "sabs MIN saturates" ~expected:max64 ~actual:(sabs min64);

  (* Division truncates toward zero, exactly as Rust's `/` and `%` do. *)
  Check.equal_int64 "-7 / 2 truncates toward zero" ~expected:(-3L)
    ~actual:(div_trunc (-7L) 2L);
  Check.equal_int64 "-7 % 2 keeps the dividend's sign" ~expected:(-1L)
    ~actual:(rem_trunc (-7L) 2L);
  Check.equal_int64 "7 / -2 truncates toward zero" ~expected:(-3L) ~actual:(div_trunc 7L (-2L));
  Check.equal_int64 "7 % -2 keeps the dividend's sign" ~expected:1L ~actual:(rem_trunc 7L (-2L));
  Check.equal_int64 "-7 / -2" ~expected:3L ~actual:(div_trunc (-7L) (-2L));
  Check.equal_int64 "-7 % -2" ~expected:(-1L) ~actual:(rem_trunc (-7L) (-2L));
  Check.equal_int64 "7 / 2" ~expected:3L ~actual:(div_trunc 7L 2L);
  Check.raises "division by zero is refused" (fun () -> div_trunc 1L 0L);

  (* The unreachable cost. Four of them still fit in an i64. *)
  Check.equal_int64 "infinite_cost is i64::MAX / 4" ~expected:(Int64.div max64 4L)
    ~actual:infinite_cost;
  Check.ok "four infinite costs do not saturate"
    (Int64.compare (sadd (sadd infinite_cost infinite_cost) (sadd infinite_cost infinite_cost))
       max64
    < 0);

  (* The i32 layer. *)
  Check.equal_int "i32_min" ~expected:(-2147483648) ~actual:i32_min;
  Check.equal_int "i32_max" ~expected:2147483647 ~actual:i32_max;
  Check.equal_int "clamp_i32 in range" ~expected:1000 ~actual:(clamp_i32 1000L);
  Check.equal_int "clamp_i32 at the ceiling" ~expected:i32_max
    ~actual:(clamp_i32 (Int64.of_int i32_max));
  Check.equal_int "clamp_i32 one above the ceiling" ~expected:i32_max
    ~actual:(clamp_i32 (Int64.of_int (i32_max + 1)));
  Check.equal_int "clamp_i32 at the floor" ~expected:i32_min
    ~actual:(clamp_i32 (Int64.of_int i32_min));
  Check.equal_int "clamp_i32 one below the floor" ~expected:i32_min
    ~actual:(clamp_i32 (Int64.of_int (i32_min - 1)));
  Check.equal_int "clamp_i32 of i64::MAX" ~expected:i32_max ~actual:(clamp_i32 max64);
  Check.equal_int "clamp_i32 of i64::MIN" ~expected:i32_min ~actual:(clamp_i32 min64);
  Check.equal_int "i32_add saturates high" ~expected:i32_max ~actual:(i32_add i32_max 1);
  Check.equal_int "i32_sub saturates low" ~expected:i32_min ~actual:(i32_sub i32_min 1);
  Check.equal_int "i32_add is exact in range" ~expected:3000 ~actual:(i32_add 1000 2000);
  Check.equal_int "i32_sub is exact in range" ~expected:(-1000) ~actual:(i32_sub 1000 2000);
  Check.equal_bool "i32_max is in range" ~expected:true ~actual:(is_i32 i32_max);
  Check.equal_bool "one past i32_max is not" ~expected:false ~actual:(is_i32 (i32_max + 1));

  (* The usize layer. This is the trap: OCaml's `-` goes negative, Rust's does not. *)
  Check.equal_int "usub 9 - 4" ~expected:5 ~actual:(usub 9 4);
  Check.equal_int "usub 4 - 9 stops at zero" ~expected:0 ~actual:(usub 4 9);
  Check.equal_int "usub 0 - 1 stops at zero" ~expected:0 ~actual:(usub 0 1);
  Check.equal_int "usub n - n" ~expected:0 ~actual:(usub 7 7);
  Check.equal_int "uadd" ~expected:13 ~actual:(uadd 6 7);
  Check.equal_int "uadd saturates" ~expected:max_int ~actual:(uadd max_int 1)
