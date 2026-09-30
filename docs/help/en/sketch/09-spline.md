# Spline

Key **N**. A smooth curve through points.

![A spline through four points: a smooth curve passing through them.](img/sketch-spline.png)

## How to draw it

Press **Spline** and click the knots in order; a **double-click** places the last one and ends the curve. The knots
can be dragged with the mouse later — the curve follows them.

## Mind the definition

Each knot has two degrees of freedom, and pinning a spline down entirely with dimensions is next to impossible. That
is fine for a styled shape and bad for a working outline: where a function sets the shape (a surround, aerodynamics,
ergonomics), a spline fits; where a size does (a fit, a hole, a joint), lines and arcs are better.

## If it did not work

A contour with a spline does not extrude — the ends of the spline do not meet their neighbours. Pin them with the
**Coincident** constraint to the ends of the neighbouring lines.
