#import "@local/typst-template:0.31.0": *

#show: template.with(
  title: [Notes on the Stanford GraphBase],
  authorship: (
    (
      name: "Adam Martinez",
      email: "staying@never.land",
      affiliation: "University of Life",
    ),
  ),
)

= Project code organization

The main project has apparently been improved by a contributor other than DEK, including patches for
what seems like compatibility with post--ANSI C code, as well as better pointer handling practices.
The changes beyond those in specific folders like `AMIGA` or `MSVC` (i.e. those included as patches
in the project's root directory,) have not yet been inspected. Implementing the changes included in
those files will not be done, as apparently the core logic was sound enough to even be implemented
in both the Boost Graph Library, and in a subproject of it.

As per the project's `README`, all logic is implemented in terms of the _kernel routines_, so called
because they implement the graph DSs in use as well as some routines for efficiently handling both
random number generation and linked list traversal (maybe because adjacency lists are implemented in
terms of linked lists?)

The actual logic for graph generation is stored in all `gb_*` files other than
#l-enum[`flip`][`graph`][`io`, and][`sort`], except for #l-enum(numbering: "(a)")[`dijk`][`save`,
  and][`types`]. I believe the main approach to this should be an inspection of the kernel routines,
followed by an in--depth study of the generative routines. Hopefully, this can yield some
conclusions as to whether a generic interface over the graph kernel routines can be implemented,
such that a different "backend" can be interchangebly used with the same generative routines.

== Kernel code files

=== Random number generation (`gb_flip.w`)

The interface to the code is fairly simple, and is apparently based off of a publication of the name
_Seminumerical Algorithms_. I may require this article if the engine proves too complex for me to
implement without further external assitance.

In and of itself, this part of the program offers a function with which to initialize the random
number generator, and a macro with which to produce a random number. Both of these are very much
transparent in the way the perform their internal operations, as the initial routine expects an
explicit seed with which (for now, I believe) the program "picks" a point in its deterministic
sequence to start off producing values. Beyond this, the macro to be called makes explicit the fact
that the genearated numbers follow as part of, upon initialization, a predetermined series.

#let period = 85 - 30

According to the file, the period of the numbers is of $2^(85) - 2^(30) = 2^#period$. According to
Skiena's book, the cycling of numbers that rely on $2^32$ calls of a linear congruential engine is
worrying. Whether the algorithm used in this file is a linear congruential engine, and whether
$2^#period$ calls may be performed by today's computers in little more than $2^32$ calls is
something I am not knowledgeable of.

Further inspection of the file reveals that this seems very much like an instance of a linear
congruential engine, where the value of a random number $n$ is determined as the function $R_n$,
such that

$
  R_n = (R_(n - 55) - R_(n - 24)) mod m, \
  "where" m "is even and" R_0, R_1, dots.c, approx R_54 "may contain both even and odd numbers".
$ <random-engine-formula>

This looks a lot like the computation resolved in the example in @skiena-2020[Sec. 16.7, p. 487]. It
computes the value of the $n$th random number from some other $n - 55$th and $n - 24$th random
numbers. This algorithm is also noted to consider $m$ as taking on the largest value with which to
bound the number the recurrence relation in the modulo's lhs resolves to, by taking on the $2^31$
negative numbers available in almost all modern computers for 32--bit signed integers.

In terms of the effectiveness of such a random number generator, DEK notes that the chosen offsets,
namely 24 and 55, should prove to be good enough for "most" applications. This likely doesn't cover
cryptographic applications, but neither do I belive the Stanford GraphBase to require of
fine--grained pseudorandom number generation. The point being, *the period of this generator should
be at least $2^(55) - 1$.*

The C code generated after running `ctangle` is full of `#line` directives I'm going to ignore for
the sake of sanity. Maybe there's more information on this in @knuth-graphbase @knuth-literate. The
code then seems fairly straightforward.

The "public" interface in the header file exposes both the macro for advancing the random number
sequence, as well as three more routines.

/ `gb_flip_cycle()`: \
  Yet unbeknownst to me.
/ `gb_init_rand()`: \
  To initialize the random number engine with a given seed.
/ `gb_unif_rand()`: \
  Yet unbeknownst to me.

Beyond this, the state of the whole shebang is kept through two globals; #l-enum[an array `A`
  holding $n + 1, "where" n = 55$ elements, and][a pointer to that array `gb_fptr`]. The array is
set to hold one more than the largest offset used in @random-engine-formula, such that the first
value is used as a sentinel (more on this once I get to know the code better.) The pointer doesn't
exist for any purpose other than allowing the user to run the macro exposed in the header file
(which otherwise would not have any means of reaching for the array holding the core state.)

Of note is that the documentation means a limitation with the `gb_next_rand` macro: There seems to
be a test, referred to as the _birthday spacings test_, that fails to prove this to be a decent
enough random number generator. The solution proposed by DEK is to modify the definition of the
macro such that instead of performing as follows,

```
#define gb_next_rand() (*gb_fptr >= 0 ? *gb_fptr-- : gb_flip_cycle())
```

it performs two cycle flipping computations in a row before continuing execution.

```
#define gb_next_rand() (*gb_fptr >= 0 ? *gb_fptr-- : (gb_flip_cycle(),  \
                                                     gb_flip_cycle()))
```

The modification would exploit the comma operator in C to allow for two consecutive runs of the
routine prior to continuing with the flow of execution at the macro invocation site; This is
possible thanks to the fact C evaluates as `void` the lhs of the comma operator and assures that the
rhs will only run after the lhs has finished execution, such that the `gb_flip_cycle()` returns the
last element of the stateful array after having run 110 steps of the recurrence defined in
@random-engine-formula. For future reference, I belive it is useful that we consider the operations
performed in the macro.

First, it considers whether the current value pointed to by `gb_fptr` is negative. This holds only
for the sentinel value in the array the pointer is aliasing, and thus serves as an indication that
it's time to run `gb_flip_cycle()` (though the workings of this are, yet again, unbeknownst to me at
this point.) If the value yielded happens to still be within the "acceptable" range, then the
underlying value is dereferenced again prior to performing pointer arithmetic by subtracting from
`gb_fptr`; Note how the decrement operator is used in its postfix form, such that only upon
returning the value does the pointer's address recede back by one position in the array.

The `gb_flip_cyle()` routine is said to perform 55 iterations of @random-engine-formula, aiming for
these to be as high--speed as possible by requesting register storage of the pointers it uses for
that. This function's body, though, is quite the sight for sore eyes; It keeps two pointers to the
array holding the $n$ random numbers (never acting on the sentinel value at index `1`,) and performs
_pointer address_ comparisons to consider whether the pointer at the end of each loop iteration has
hit the address of the last element in the array. The problem here is that the exit condition of the
loops depend on whether the pointer involved in each loop, respectively, has an address that is now
"beyond" the address range of the array (i.e. has an address that is numerically larger than that of
the last element of the array.) Technically, one can trust that C arrays will allocate contiguous
memory and thus an address that is numerically larger than the address of the last element in the
array would be outside the safe range in which to dereference the pointer so the check is certainly
not incorrect in its logic. But this is borderline unsafe in Rust.

Then it proceeds to "reset" the `gb_fptr` pointer by making it alias element at index `54` of the
stateful array. I belive it resets to the element right before the last and not to the element
before the last proper because the routine itself returns the last element in the stateful array.
And then because this function is really only used inside the `gb_next_rand` macro, it's expected to
keep a coherent sequence of values, such that so long as we've not hit the sentinel, we return the
dereferenced `gb_fptr`, otherwise calling `gb_flip_cycle()` and getting after its call the value at
the very end of the array, while resetting back (as DEK says) `gb_fptr` for the next call to the
macro to start anew.

#bibliography("bib.yml")
