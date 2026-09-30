# Offset

A contour standing off the selected one at a given distance.

![A contour and its offset 6 mm inwards.](img/sketch-offset/)

## How to do it

Press **Offset**, type the **Distance** in the bar above and click the contour. The sign of the distance chooses the
side: with one sign the copy lies outside, with the other inside.

## The copy holds to its source

The copy is tied to the source contour: its segments run parallel to their own at the given distance, its arcs from
the same centres. Change a size of the source — the copy follows. The offset of a circle is a concentric circle with a
radius larger or smaller by the distance.

## Where it is used

- **A wall**: the outer contour exists, the inner one is an offset inwards by the thickness.
- **An allowance**: the outline of the part and the outline of the blank around it.
- **A slot along a path**: a centreline and two offsets at half the width.

For an even wall over a whole body, **Shell** in the part is better: it works on the body, not on a flat contour.

## If it did not work

- Inwards does not work — the distance is more than the contour holds: going inwards the sharp corners close in.
  Make the distance smaller.
- The copy did not follow the source — where the offset dropped or merged segments (a narrow ledge inside), the copy
  lies free. Make the distance smaller or split the contour into parts.
