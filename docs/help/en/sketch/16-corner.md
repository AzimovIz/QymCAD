# Corner fillet and chamfer

![A sharp corner and the same one rounded: the radius becomes a dimension, the tangencies become constraints.](img/sketch-corner/)

## How to do it

- **Fillet** (key **F**): click the corner of two lines or of a line and an arc, type the radius into the field at
  the corner, **Enter**.
- **Fillet by its chord or arc length**: on the bar above press **Chord** (the straight distance between the two
  ends of the arc) or **Arc length** instead of **Radius**, then click the corner, type the value, **Enter**.
- **Chamfer**: click the corner of two lines, type the size into the field at the corner, **Enter**.
- **Chamfer by two values**: on the bar above press **Two distances** (**Leg 1** and **Leg 2**) or **Leg and angle**
  (**Length** and the **Angle** between that line and the cut); **Symmetric** gives one size along both lines. The
  first value runs along the line you click nearer to: click the corner a little to the side of that line. Type the first
  value, **Tab** to the second, **Enter**.
- **The corner by its two lines**: with two lines that share a corner chosen, the button offers to take that corner
  off straight away. It works with nothing chosen as well — click the first line, then the one beside it, and the
  corner between them is offered at once.
- **Fillet every corner of the contour**: select the contour, press the button, type the **R of every corner**,
  **Enter**.

## How a corner is named

Every click in the sketch is a line or a point, and the window of the selection is **the last two clicks**: each new
one pushes the previous out of the window. So the second line of a pair becomes the first of the next pair, and three
lines in a row are two corners in a row.

What the window offers:

- **two lines** — the corner between them and no other: the pair is named, and there is nothing to ask;
- **a line and a point** — the point says where the corner is, the line says which of the corners there is meant:
  only the ones that line takes part in are offered, and the cursor standing in a sector says which of them;
- **a point alone** — every line that meets at that point is offered, and the cursor says which pair of them;
- a window that names nothing (two points, a line with a point it does not reach, two lines along one straight line)
  is the newest click alone: a line waits for its partner, and a point names the corner at it at once.

After **Enter** the second line of the pair stays lit — it is the first of the next pair. A second click on the same
line or point takes the choice back.

A click after which the window names nothing takes the field of the corner before it down: there is no sense in
showing the preview of a corner the selection no longer holds. Leaving the mode — the button, **Esc**, another tool —
lets go of the selection too, and the lines it had chosen go dark.

**A joint of two pieces of one straight line** (an angle of 180 degrees) is not a corner: the second line becomes the
first, and the search carries on with the next neighbour of it.

One step of undo per operation: **Ctrl+Z** brings the sharp corner back.

## What is shown before Enter

While the field stands open the sheet shows where the corner will go: the segment or the arc, and two marks on the
lines — where they will be cut. The lines themselves are not changed; this is only the preview, and **Enter** applies it.

- A value too big for that corner draws no preview, and the field says so.
- Where four lines meet at a point named by a point alone, the corner follows the cursor: point it at the sector to
  be taken. The cursor counts only while it stands near the point, at about three times the radius the point itself is
  caught from: move it away and the corner stands as it was named by the point.
- Where two lines named the corner, it is that corner and no other: the cursor cannot move the arc to another of the
  four.
- The corner the cursor named last stays the one in force while the cursor is away: the preview under the value being
  typed does not jump to another corner.
- Where there is no corner at all (the lines do not meet at an angle), the field says that too.

## What you get

A fillet inserts an arc and adds **two tangencies** and a **radius dimension** - or, given by its chord or arc length,
a dimension of the chord or of the arc length; a chamfer adds a segment and
its dimensions, measured from the sharp corner: the legs, or a leg and the angle. The sharp corner stays as a point
the dimensions stand on. All of these are constraints: move a side — the corner rebuilds itself; change the dimension — the fillet
changes.

Fillets usually go **last**, once the contour is defined: before that they get in the way of picking corners.

## If it did not work

- The size is not taken — it is more than the corner holds (longer than a side); the reason is written at the field.
- A chord or an arc length is not taken — the arc it makes reaches past the end of a side. Type a smaller value.
- The angle of a chamfer is not taken — with that angle the cut does not meet the other line. Make the angle smaller.
- The legs of a chamfer went the other way round — click the corner nearer to the line the first value should run
  along.
- The click did not take the corner — not exactly two lines meet at that point, or two of them lie along one straight
  line (180 degrees). Click right on the vertex of the corner, or on the two lines that meet there at an angle.
- The wrong corner of the four at one point was taken — the cursor has to stand in that sector, and where a line was
  picked first, only the corners that line takes part in are among the answers.
