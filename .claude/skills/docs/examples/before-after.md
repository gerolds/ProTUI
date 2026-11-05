# Before / after

This fictional example compares fluent prose with actionable documentation.
It does not describe a component of this repository or claim measured performance.

## Before

> Our image system provides efficient, flexible asset rendering. It minimizes
> loading time and supports high-quality output. Use the cache where possible
> and make sure to release resources. In typical scenes it improves performance
> by up to 30%.

Defects: intent is presented as behaviour, the number has no source, the cache
condition is missing, and the reader cannot locate an entry point or reproduce
anything. Polished wording does not repair missing evidence.

## After

> The converter reads checked RGB pixels and writes glyphs and foreground/background
> colours into a caller-owned buffer. It does not own terminal setup.
>
> Keep one conversion cache while the source, target dimensions and cell aspect
> stay unchanged. Rebuild its sampled image after any of those change. A small
> decoded PNG and a real terminal exercise cover different boundaries; list each
> exercised command separately in the handoff.
>
> Unknown: no allocation measurement is attached to this example. Do not turn the
> caching strategy into an invented frame-time claim.

A real page must replace these generic statements with the inspected local API
and evidence. Keep a sentence when removing it would force the reader to ask a
question. Delete it when it merely repeats a name or persuades without evidence.
