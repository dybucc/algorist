# `grapht`

This library implements functionality to make producing test graphs
programmatically more amenable. It is based off of the work on the Stanford
GraphBase by Donald E. Knuth, borrowing the same ideas for the underlying
algorithms, but ultimately implementing a different public API.

One major difference includes providing a backend-agnostic set of routines for
implementing the same functionality as the original GraphBase. This means any
implementor of the `GraphBackend` trait and the extension traits required for
iteration over vertices and arc insertion can benefit from the all of the
generative routines. By default, the library ships with an implementation of the
same graph backend as in the original GraphBase, with slight modifications to
make it more Rusty/safe.

It's still very much a work in progress, though.
