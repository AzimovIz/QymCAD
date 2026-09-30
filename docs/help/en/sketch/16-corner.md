# Corner fillet and chamfer

![A sharp corner and the same one rounded: the radius becomes a dimension, the tangencies become constraints.](img/sketch-corner/)

## How to do it

- **Fillet** (key **F**): click the corner of two lines or of a line and an arc, type the radius into the field at
  the corner, **Enter**.
- **Chamfer**: click the corner of two lines, type the size into the field at the corner, **Enter**.
- **Fillet every corner of the contour**: select the contour, press the button, type the **R of every corner**,
  **Enter**.

One step of undo per operation: **Ctrl+Z** brings the sharp corner back.

## What you get

A fillet inserts an arc and adds **two tangencies** and a **radius dimension**; a chamfer adds a segment with a
dimension. All of these are constraints: move a side — the corner rebuilds itself; change the dimension — the fillet
changes.

Fillets usually go **last**, once the contour is defined: before that they get in the way of picking corners.

## If it did not work

- The size is not taken — it is more than the corner holds (longer than a side); the reason is written at the field.
- The click did not take the corner — not exactly two lines meet at that point. Click right on the vertex of the
  corner.
