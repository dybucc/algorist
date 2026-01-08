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
  "where" m "is even and" R_0, R_1, dots.c, R_54 "may contain both even and odd numbers".
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
stateful array. I belive it resets it to the element right before the last and not to the element
before the last proper because the routine itself returns the last element in the stateful array.
And then because this function is really only used inside the `gb_next_rand` macro, it's expected to
keep a coherent sequence of values, such that so long as we've not hit the sentinel, we return the
dereferenced `gb_fptr`, otherwise calling `gb_flip_cycle()` and getting after its call the value at
the very end of the array, while resetting back (as DEK says) `gb_fptr` for the next call to the
macro to start anew.

The `gb_next_rand` macro simply computes the actual formula in @random-engine-formula with discrete
values for the terms $R_(n - 55), R_(n - 24)$. This, though, is not a built--in modulo operation
with the chosen $m$ ($2^(31)$), but rather a (possibly) more optimized version using a bit--wise
#smallcaps[And] that relies on the machine using 2C bit representation its integer primitive types.

Note that, according to the docs, the `gb_flip_cycle()` routine is to be thought as reflecting the
sequence of values, in the sense that they are now considered in reverse order to their initial
orderings. Still, a point is made about this not affecting the degree of perceived randomness on the
returned sequence throughout calls to the `gb_next_rand` macro and subsequent "flipped cycles" upon
hitting the sentinel value in the array with the `gb_fptr`.

The initialization routine `gb_init_rand()` follows a process akin to the one detailed in the
article mentioned by DEK (the same one as the one initially mentioned in these notes,) except that
apparently the article bases its generator off on the assumption that only the low--order bits of
the initial values (those variables allocated at the start of the routine) are the ones with
pseudodeterministic significance. Then for the initial number sequence dispersion, it makes use of
coprime numbers 21 and 55 because further increments expressed in terms of a modulo such as
$21 mod 55$ allow for the iteration--based values in use with the initialization of the seed to be
numbers part of the Fibonacci sequence. The reason why this is any relevant for the purposes of a
random number generator are left to another article DEK quotes by the name (quite possibly a
recurring publication) of _Sorting and Searching_.

The reason why the resulting C programs from running `ctangle` on the CWEB sources make abundant use
of the `#line` directive is due to the fact literate programming, as conceived by DEK in WEB, may
clip parts of a given routine or general language construct, for the sake of documenting an isolated
"region" of it. This in turn forces the `ctangle` to parse and force a restructuring that may not be
desired when debugging and using compiler--defined symbols when the C source file changes the
ordering of such lines to the one expected by a compiler toolchain. This in turn implies DEK expects
CWEB programs to be perused in their `.w` forms, and not as standalone C programs, including
debugging.

Back to the initialization routine, this process mostly consists of three separate steps:
#l-enum[Assigning to each value of the statful array a different "random" value][computing the next
  set of values that will be assigned to such elements of the array, and]["warming up" the values
  finally set in the array by calling for 275 steps of the array value--reflecting routine within
  the cycling function.]
The reason behind the warmup cycles being run after the routine showcased in DEK's own volume 2 of
_The Art of Computer Programming_ is due to the fact that the least 10 significant bits (the
low--order bits we spoke of before) present a fairly predictable pattern no matter which first
random number we ought compute. Of course, the pattern is only obvious when purposefully considering
the bits of the numbers, even if small fluctuations may happen between the 9th bit and the 1st bit.
The quick cycling (array member reflection) of added to the initialization routine for this pattern
to quickly disperse, as otherwise the first few hundred runs would very much follow step no matter
the execution conditions.

Beyond this, there's nothing else to the generator routines. The only other function present in the
public interface of the library is one for computing a uniform, _bounded_ distribution of integers.
As per #author(<skiena-2020>), the function $R_n$ already produces such bounded distribution, where
the range is denoted as $[0, m)$, so it's quite possible the function presented in DEK's generator
is not a linear congruential engine, but can be coerced into the ranges of one. The reason why the
routine is provided instead of simply bounding the generated number by a modulo opeartion is
attributed to the fact that such operation would yield values smaller than or equal to $m/2$, on
$2/3$ of most execution runs. This routine should apparently (for no further elaboration on the
reason why is given) try to clamp the genearted value down to the specified range, while not
consuming any more than 2 more random number generations (through the `gb_next_rand` macro.)

This seems to work especially well for values of $m$ larger than $2^(16)$, where the trend for
smaller values is seen more often. Because the random numbers generated within the hot loop of the
routine are compared with the largest representable 32--bit _unsigned_ integer modulo $m$, this
should technically perform a form of await operation that would evaluate the generated random number
prior to itself being returned modulo $m$. This may sound like more than two runs (and thus more
than two macro invocations to `gb_next_rand`) would be required, but of note is that the generated
number is itself not yet bound to a range smaller than the provided $m$, even if the heuristic used
to compute a number anew is based on a value bound within $m$ (more specifically, $2^31 mod m$.) It
is only once the computed value yields a (possibly) larger value than those generated under $m/2$
that the routine returns the "clamped" value $n mod m$ (I use $n$ instead of the variable name `r`
in the source code, for the purpose of generality.)

*Pending: getting information on some omitted technicalities on the methods used for both engine
initialization and number generation (though these should be found in DEK's own _TAoCP_).*

=== Graph routines (`gb_graph.w`)

The file speaks of safety in terms of undefining a `min` macro that seems to exist in certain system
headers, but it uses considerably unsafe practices for pointer arithmetic when performing traversals
in a loop.

All declarations until linea 110 of the generated C code are part of the type system used for graph
abstractions. It also seems to include a `verbose` and `panic_code` variables to consider for
#l-enum[possible additional verbosity in the output of graph routines (akin to current--day
  `--verbose` CLI options), and][a custom, `errno`--like, global return code living as a static
  uninitialized variable for the purposes of reporting a wide assortment of failure cases],
respectively.

The error codes returned use a sligthly primitive approach based on the same `E*` error codes
present in #smallcaps[UNIX]--based platforms, but really only using generic names for, say, a syntax
error, where there may be different variations in the core source, but such subtleties are left to
the engineer to find out by perusing the implementation in search of the offset added to the generic
error code. As a general rule, DEK allows for the following ranges to delimit the set of values that
may be stored in the `panic_code` global.

/ Code $-1$: \
  Some memory limit was hit during some prior operation. This is not further explained, and I don't
  know how would a single error buffer hold an indication of a "previous" error if determining that
  the error itself took place before some other error already requires having had another error, and
  thus having had to discard the "previous" error to use this integer buffer for the new error code.

/ Range 1--9: \
  Some memory limit has been hit, quite possibly due to heap or stack allocation limits, though I
  doubt the program uses the `alloca` syscall.

/ Range 10--19 (10 with an additional offset 1--9): \
  This denotes some error took place at the "start" of one of the data files sourcing the
  information that gets parsed onto the graphs. Note that we speak of a _10 with an additional
  offset_ because the next item denotes variations on the errors when the error has taken place at
  some point in the second middle of the `.dat` file at fault.

/ Range 11--19 (11 with an additional offset 1--8): \
  Much in the same way as the above range, it reports some error has taken place at some point in
  the "lower end" of the file (the second half of it.)

/ Range 20--29: \
  There was a syntax error while reading a `.dat` file. I assume this error implies the above two
  ranges relate to memory corruption errors in the #smallcaps[I/O] routines, and not so much in the
  file parsing procedure.

/ Range 30--31: \
  There was an error related to the set of parameters passed to some kernel routine. This range is
  reserved for fairly "well--behaved" but either way wrong values that may still make some sense in
  terms of the operation performed by the routine at hand.

/ Range 40--49: \
  Ibid., except the passed values are completely wrong and it could very well be that the
  overarching logic involved in the call to the reporting subroutine is also wrong. Basically,
  you're _stupid_ (in the words of DEK himself.)

/ Range 50--59: \
  The graph parameter is `NULL`, which is no good but especially so in this codebase, where
  according to the `README`, DEK _assumed_ that this symbol was defined universally as `0` and thus
  is used for expansions where one would naturally expect the number 0 used. *Pay attention to this
  item, both because in Rust this could very well be a no--op, and because the code could be quite
  wrong in its use of `NULL`.*

/ Range 60--89: \
  A parameter is technically correct, but it likely breaks some invariant the routine must upkeep as
  part of its pre/post--conditions (maybe.)

/ Range 90--onwards: \
  This is not meant to happen, but in the name of safety (the irony,) DEK still considers these edge
  cases, and binds values to be interpreted as execution paths ending at an `unreachable!()` macro
  in Rust.

The docs move on now to the DSs in use for graph abstractions. Because of the presence of multiple
types of graphs across the actual library routines in GraphBase, graphs use an overarching structure
`Graph`, which itself is accompanied by types for edges and vertices. The edges use an `Arc` type,
while the vertices use a `Vertex` type.

Each graph uses an array (though at this point it's unspecified whether it is stack or heap
allocated; The base assumption should be that of a contiguous memory allocation) to store `Vertex`
structures. Then it uses a linked list (this one must be heap--allocated, for Rust's sake) to keep a
record of the edges, such that each element of the linked list is an `Arc` structure.

By the sound of it, this seems to be using an adjacency list, where the array of `Vertex`s possibly
has that each of those structures contains a field with a pointer to the linked list of adjacent
edges as `Arc`.

Another core tenant of the architectural design is that each one of the `Arc`, `Vertex` and `Graph`
types is provided with an assortment of additional fields of a union type `util`. This union
contains one of #l-enum[a pointer to another `Graph`][a pointer to another `Vertex`][a pointer to
  another `Arc`][a string literal as a pre--ANSI C `char *`, or][a 32--bit signed integer]. This
union is said to be provided in the spirit of allowing flexibility for the algorithms using these
set of DSs. *This could quite possibly benefit from a complete refactor in Rust to extend said
flexibility far beyond the one achieved by a few union fields in a structure*.

The set of fields in the `Vertex` structure is fairly straightforward. It associates a string
literal with the `Vertex` in question, as well as a standard C implementation of a singly linked
list for the adjacency list, such that only a pointer to an edge `Arc` is really held per vertex.
Then an invariant holds such that if the pointer is non-`NULL`, one may assuredly access the `Arc`
and check for the null--ness of a `next` field giving step to the next edge _with respect to the
original `Vertex` from which the pointer to `Arc` originated_. Each vertex is also outfit with 6
`util` unions, which are said to be useful for some such information as vertex degrees or additional
relationships to other elements of a given graph.

The set of fields in `Arc` structures hold information about the vertex that some other vertex is
connected to (implying here that such relationship only really holds true when sourcing an `Arc`
from the adjacency list of a `Vertex`, where this latter vertex makes up the other end of the edge,)
the next `Arc` connecting the source `Vertex` (in the same context as the above note) in such
vertex's adjacency list, and a length associated with the edge (possibly for weighted graphs or
otherwise embedded/isomorphic graphs/needs for graphs.) Unlike `Vertex`, it holds a smaller number
of `util` unions, namely 2, for purposes of diversifying use in routines external to GraphBase's
kernel functions (this may imply that `Arc`s are expected to hold far less meaning, and thus
extensions to the core functionality should focus on `Vertex` structures.)

This design for vertices and edges does force the same set of restrictions on graph representations
as basic adjacency lists presented in @ds-handbook-graphs. A more optimized graph backend set of DSs
is definetely a better idea, and will likely motivate the implementation of the Engine API as a Rust
trait--based implementation of this and other routines.

The docs move on now to explaining the memory allocation strategy followed to try and make the
system efficient. This is quite possibly going to require both considerable refactoring in Rust and
benchmarks to test that whatever efficency was attained with DEK's methods continues being
_attainable_ with Rust methods.

The way memory allocations and memory resources are freed is by means of an abstraction layer over
basic memory handling library functions of whatever pre--ANSI C the author used. The entrypoint and
main resource handle in all routines involved is the so--called `Area` type(def). This could be said
to be modeled after the `allocator` API in the corresponding module of Rust's `std`, only instead of
being a complete memory allocator, it serves as a bridge to mediate between the default system
allocator and whatever memory allocation requests are performed in the graph routines.

The `Area` type is, at its core, a singleton array of pointers to other `Area`s (to allow for direct
dereferencing through array--to--pointer decay without expecting the user to provide the explicit
address--of operator in parameters to memory allocation request routines.) When rid of this level of
indirection, the underlying structure reveals to be a `char *` (this is from back when `char *`s
were used instead of `void *`s because this latter type wasn't valid, "generic" C code,) denoting
the address at which this block of "memory area" starts. This is also accompanied by another field
`next` pointing to another `Area`. I don't completely understand whether this field acts as a form
of linked list connection to the next (or previous?) block of allocated memory. The documentation
mentions it's a pointer to the start of the previously allocated block, but also implies that a
block is made out of multiple allocations acting as the pointees of differing sources.

Maybe `Area`s are really only an abstraction over a zeroed memory block, that is not itself meant to
be used as memory for an object, but rather as pre--allocated resources to make requests on. Maybe.

The basic use of `Area`s follows that the user should consider in which segment of the program the
resource handle is stored at, and if it is outside the statically initialized data segment, so
either in _bss_ or in stack space (I don't think it a good idea to have it in the free store) then
it should proceed to call the `gb_init` macro to initialize the array to `NULL` (this, for one, is a
correct use of the symbol.) Each `Area` represents a fixed--size heap allocation, and each call to
the `gb_alloc()` routine with a block size and an `Area` passed as parameters is expected to either
#l-enum[allocate on a `NULL` memory area][extend a non--`NULL` memory area, or][return `NULL` (as a
  status code, so quite wrongly assuming it expands to `0` on all platforms) if the `Area` cannot
  allocate any further].

Note the memory allocation limits set on this routine are not given by system limits (reacting to an
error to C allocation routines,) but rather by a hardcoded magic number `0xffff00` (i.e.
$approx 16 upright(M)$ bytes.) If such an error takes place, another global (static) status code
variable, `gb_trouble_code`, is set to a non--zero value such that if performing multiple
allocations in a row, an error on any one of these may be detected by means of a conditional check
on said status code.

A convenience macro, `gb_typed_alloc`, is provided to allocate the required amount for a given type,
through a quick `sizeof` on said type and a multiplication by the requested amount of objects $n$ of
that type. The resulting value is passed as part of an allocation request to `gb_alloc()`
representing the number of bytes. Because this last routine requires an `Area` to be passed as well,
it both allocates the required heap memory on said area, and returns a pointer that is cast to a
pointer to the type passed as part of the `gb_typed_alloc` macro.

The `gb_alloc()` routine, at its core, only really `calloc()`s $2^8$ items, each made out of `n`
bytes passed as parameter (having applied the ceiling function to those `n` bytes such that they are
a multiple of the platform's pointer size, `char *` back when the program was written instead of
`void *`, but 8 bytes either way (#smallcaps[LP] or #smallcaps[LLP] models would do just fine, and
the program likely predates the time when proposals over these two "ended" other memory models.))
The specific size of each of the elements to be allocated is not completely clear to me just yet;
DEK computes $n / m + (2m) / m + (m - 1) / m$, which should theoretically resolve to
$approx (n - 1) / m^2 + 1 / m + 3$. But this meaning of this is lost on me right now. Maybe it only
serves as a range restriction as per the notice in the documentation, which comments on old--style C
having a hard limit on the byte size passed as the first parameter to `calloc()`.

If the allocation is sucessful, the returned region of heap memory goes through three main
"manipulation" steps:

+ The address lying `n` bytes (post--ceiling function) forward from the returned `calloc`ation is
  cast into the element of an `Area` (the underlying `struct area_pointers *`) while a separate
  temporary `Area` is dereferenced (making use of array--to--pointer decay to reach directly for the
  first element, again a `struct area_pointers *`) to have assigned to its `first` field the
  starting address of the original `calloc`ation,

#bibliography("bib.yml")
