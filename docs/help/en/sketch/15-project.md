# Projection

Click the edges of a body and they appear in the sketch as entities. The “Face outline” switch above
takes the whole outline of the picked face at once instead of edge by edge. The body may be this
part's own or another part of the assembly.

![The outline and the hole circle are taken from the body: real sketch entities, not a backdrop.](img/sketch-project.png)

## What it is for

So that a new sketch stands on geometry that already exists: a hole coaxial with a boss, a cut-out
along the edge of a flange, a groove along a rib. Tracing that by hand means typing the same numbers
a second time and losing the link to the original.

## A projection is driven

A projection is not redrawn: it **follows** the body. Change the part higher up the history and the
outline in the sketch moves with it, and so does everything built on it.

That is why a projection cannot be dragged by hand: it has no degrees of freedom of its own, the
source sets them.

## Another part's geometry

When one part's sketch stands on another part's geometry, an explicit **external reference** appears
between them. It shows in the properties of the part and can be broken — [more](assembly/06-external-refs).

This is top-down design: the bracket's dimensions come from the housing instead of being retyped
into it. Move the source part in the assembly and the outline follows.
