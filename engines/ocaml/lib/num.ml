(* SPDX-FileCopyrightText: 2026 jlreq contributors *)
(* SPDX-License-Identifier: MIT OR Apache-2.0 *)

(** Integer semantics.

    Every quantity this engine computes is an exact integer. Nothing here uses
    [float], and nothing anywhere else in the engine may: a single floating-point
    value in a response is a guaranteed conformance failure, because the runner
    compares parsed JSON structurally and [1] is not [1.0].

    The engine is written against a reference implementation whose arithmetic is
    Rust's, so the two layers Rust distinguishes are distinguished here too.

    - The {b i64 layer} is [Int64], not the native [int]. OCaml's [int] is 63 bits,
      so [max_int] sits at a different place than [i64::MAX] does; the line-breaking
      cost function uses [i64::MAX] as a live sentinel value, and a saturating add
      that stops one bit early stops at the wrong number. [Int64] is slower and
      boxed, and correctness wins.
    - The {b i32 layer} is the native [int] with an explicit clamp. Every number
      that crosses the protocol boundary is an [i32], and the schema enforces the
      range, so the clamp is where a computation that left the range is caught.
    - The {b usize layer} is the native [int] with a floor at zero. This is the
      easiest thing in the whole engine to get wrong: Rust's [saturating_sub] on a
      [usize] stops at [0], OCaml's [-] carries straight on into the negatives, and
      a negative byte offset is a wrong answer rather than a crash. Subtracting one
      offset from another goes through {!usub} and nothing else.

    Truncating division needs no conversion at all: Rust's [/] and [%] round toward
    zero, and so do OCaml's [/], [mod], [Int64.div] and [Int64.rem]. The functions
    {!div_trunc} and {!rem_trunc} exist to say so at the call site. *)

(* ----------------------------------------------------------------------------- *)
(* The i64 layer *)
(* ----------------------------------------------------------------------------- *)

(** [i64::MAX]. *)
let i64_max = Int64.max_int

(** [i64::MIN]. *)
let i64_min = Int64.min_int

(** Saturating addition, [i64::saturating_add].

    Overflow happened exactly when the two operands share a sign and the wrapped
    sum does not: the sign bit of [(a lxor s) land (b lxor s)] is that condition. *)
let sadd (a : int64) (b : int64) : int64 =
  let sum = Int64.add a b in
  if Int64.compare (Int64.logand (Int64.logxor a sum) (Int64.logxor b sum)) 0L < 0
  then if Int64.compare a 0L < 0 then i64_min else i64_max
  else sum

(** Saturating subtraction, [i64::saturating_sub].

    Overflow happened exactly when the operands differ in sign and the wrapped
    difference does not share [a]'s sign. *)
let ssub (a : int64) (b : int64) : int64 =
  let difference = Int64.sub a b in
  if
    Int64.compare
      (Int64.logand (Int64.logxor a b) (Int64.logxor a difference))
      0L
    < 0
  then if Int64.compare a 0L < 0 then i64_min else i64_max
  else difference

(** Saturating multiplication, [i64::saturating_mul].

    [Int64.mul] wraps, so the product is verified by dividing it back out. The two
    [min_int * -1] shapes are settled first: they are the one case where the check
    itself would divide [min_int] by [-1]. *)
let smul (a : int64) (b : int64) : int64 =
  if Int64.equal a 0L || Int64.equal b 0L then 0L
  else if
    (Int64.equal a i64_min && Int64.equal b (-1L))
    || (Int64.equal b i64_min && Int64.equal a (-1L))
  then i64_max
  else
    let product = Int64.mul a b in
    if Int64.equal (Int64.div product a) b then product
    else if Int64.compare a 0L < 0 <> (Int64.compare b 0L < 0) then i64_min
    else i64_max

(** Saturating negation, [i64::saturating_neg]: [i64::MIN] becomes [i64::MAX]. *)
let sneg (a : int64) : int64 = if Int64.equal a i64_min then i64_max else Int64.neg a

(** Saturating absolute value, [i64::saturating_abs]: [i64::MIN] becomes [i64::MAX]. *)
let sabs (a : int64) : int64 =
  if Int64.equal a i64_min then i64_max
  else if Int64.compare a 0L < 0 then Int64.neg a
  else a

(** Division truncated toward zero, which is what Rust's [/] does.

    [-7 / 2] is [-3] and not [-4]; the engine never wants a floor. Raises
    [Division_by_zero] on a zero divisor, matching the Rust panic. *)
let div_trunc (a : int64) (b : int64) : int64 = Int64.div a b

(** Remainder with the sign of the dividend, which is what Rust's [%] does.

    [-7 mod 2] is [-1] and not [1]. *)
let rem_trunc (a : int64) (b : int64) : int64 = Int64.rem a b

(** The cost the line breaker treats as unreachable.

    [i64::MAX / 4] rather than [i64::MAX], so that a handful of these can be added
    together without the sum saturating and losing the ordering between two
    equally impossible breaks. *)
let infinite_cost = Int64.div i64_max 4L

(* ----------------------------------------------------------------------------- *)
(* The i32 layer *)
(* ----------------------------------------------------------------------------- *)

(** [i32::MIN]. *)
let i32_min = -2147483648

(** [i32::MAX]. *)
let i32_max = 2147483647

let i32_min_64 = Int64.of_int i32_min
let i32_max_64 = Int64.of_int i32_max

(** An [int64] brought into [i32] range by saturation.

    This is the only way an [i64]-layer computation becomes a number the protocol
    can carry. *)
let clamp_i32 (value : int64) : int =
  if Int64.compare value i32_min_64 < 0 then i32_min
  else if Int64.compare value i32_max_64 > 0 then i32_max
  else Int64.to_int value

(** Saturating [i32] addition. Computed in [int64] first: the native [int] is wide
    enough that the intermediate sum cannot itself overflow. *)
let i32_add (a : int) (b : int) : int =
  clamp_i32 (Int64.add (Int64.of_int a) (Int64.of_int b))

(** Saturating [i32] subtraction. *)
let i32_sub (a : int) (b : int) : int =
  clamp_i32 (Int64.sub (Int64.of_int a) (Int64.of_int b))

(** Whether a native [int] would survive the protocol's [i32] range unchanged. *)
let is_i32 (value : int) : bool = value >= i32_min && value <= i32_max

(* ----------------------------------------------------------------------------- *)
(* The usize layer *)
(* ----------------------------------------------------------------------------- *)

(** Saturating subtraction on a byte offset: the result is never negative.

    Rust's [usize::saturating_sub] stops at zero. OCaml's [-] does not. Every
    difference of two offsets in this engine goes through here. *)
let usub (a : int) (b : int) : int = if a > b then a - b else 0

(** Saturating addition on a byte offset.

    OCaml's native [int] is 63 bits, so on the sizes this protocol carries the
    guard never fires; it is here so that the intent reads the same as {!usub}'s. *)
let uadd (a : int) (b : int) : int =
  let sum = a + b in
  if sum < a then max_int else sum
