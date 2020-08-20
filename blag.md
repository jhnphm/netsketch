# NetSketch
This is a project mostly intended to learn Rust, and to satisfy some itches friends have had with
other collaborative painting applications (lack of brushes + infinite canvas).

## Design decisions

### Frameworks
Rust is used on both the frontend (using Yew) and backend (using Warp), in order to allow the use of
common serialization/deserialization libraries (serde/bincode/flate2) and data structures, plus as
the whole point of this exercise is to learn rust, maximizing its usage would be beneficial to
further this goal.

### Serialization
Serialization is done via the bincode library on both the server and client to transmit
brushstrokes, as this provides a very compact schemaless representtation of data. This is further
compacted by running DEFLATE on the bytestream using the flate2 library

### Infinite canvas
Infinite canvas is implemented by loading tiles from the server containing a set of brushstrokes to
be drawn in order. In order to solve the problem of dependencies between brush strokes, tiles are
collected serverside and sent all at once in order, using `BTreeSet` to keep them in order and
deduplicate the same brushstrokes. Caching of bitmaps and tiles clientside is TBD. Tiles are
implemented as `Vec` of indices into thte main paint stack containing all paint strokes. 

### Undo
Undo is implemented on a per-user basis. The main paint stack is iterated until a paintstroke by the
undoing user is found. This is then removed from the main paint stack, as well as any tile `Vec`'s
referencing the former index. All tile indexes to the undone paintstroke have to be updated to
account for the removed paintstroke.

### Panning (2020-08-19)

Turns out panning is a bit more difficult than envisioned. To smoothly pan we must redraw the canvas
while panning with a transform. Options are actually redrawing the canvas, which would be
prohibitively slow if reloaded from the network, redrawing from a local tile cache of the canvas, or
panning an image snapshot. Canvas `putImage`, however, is prohibitively slow. We can cache the
canvas onto another canvas and draw from there, using drawImage which is faster. However I'm going
to redo the backend and implement a local tile cache, as this will be needed as drawings get more
complex. 
