# Example: module READMEs

A README earns its place by answering what a module is for, where to start or
what will surprise a caller. Route by intent; do not recap the file listing.
A concise page is complete when its reader can act without a follow-up question.

The examples below are illustrations. The crate exists, but its real README is
the authority on its paths and APIs. Use inspected entry points when writing a real README.

## A code module

> # Terminal graphics (`crates/terminal-graphics`)
>
> Fits checked RGB rasters into terminal cells. It does not own the application
> loop, terminal setup or game state.
>
> - Decode input: start at `StillImage::from_png` in `src/image.rs`.
> - Draw the same picture repeatedly: keep one `ImageCache`; changing the
>   source, dimensions or cell aspect invalidates it.
> - Investigate a bad shape: compare the configured cell aspect with the real
>   font's cell dimensions, then inspect the encoded buffer.
>
> Surprises: linear RGB is not sRGB. Octant support depends on the selected font.
> Headless font rasterization is not proof that a real terminal renders identically.
>
> Evidence: list the exact exercised command and artifact in a real module.
> Limits: name unverified font, clipping or terminal behaviour; do not guess.

## An art folder

> # Reference illustrations (`assets/reference-images`)
>
> Neutral source images and their generation/export recipes.
>
> - Add an image: record its source/license and intended display size.
> - Regenerate an image: use its recorded recipe; preserve hand-edited files.
> - Review readability: render through the production encoder at the smallest
>   supported cell size and compare it beside the other subjects.
>
> Surprises: a large source PNG can look distinct while its small encoded output
> does not. Freshly generated variants may change composition.
>
> Evidence: link the actual contact sheet when one exists.
> Limits: do not label generated content human-approved without a recorded review.

Both examples keep provenance near the artifact, route by intent and expose
specific surprises. Use owner/status fields only when the repository has such a
workflow; never invent a team, date, test result or approval to fill a footer.
