# Page forms

Templates are aids, not required paperwork. Use the smallest form that lets the
reader act. Group entries of one kind and subject in one file; split when a part
has a different owner, update cadence or audience. An accepted historical decision
gets its own file so its history does not mix with mutable instructions.

Use standard Markdown links. Preserve useful heading anchors. A stable ID is
optional unless the repository already routes by IDs. Skip an inapplicable field
explicitly rather than inventing evidence. The examples below are fictional;
paths, keys and outcomes illustrate the forms, not this repository's installed APIs.

## Idiom

```markdown
### <practice>
**Do:** <action and condition>
**Not:** <the tempting alternative>
**Because:** <reason with inspected source or measured evidence>
**Checked by:** <actual guard, or unverified/no automated guard>
```

Example: **Do:** reuse the resampling buffer after dimensions settle. **Not:**
allocate a pixel vector for each redraw. **Because:** the area is unchanged while
focus moves. **Checked by:** an allocation measurement would be needed; none is
claimed by this example.

## Pitfall

```markdown
### <symptom the reader searches for>
**Symptom:** <observable failure>
**Cause:** <confirmed mechanism, or explicitly unconfirmed hypothesis>
**Fix:** <action, with a link to steps if needed>
**Prevented by:** <actual guard, or none>
```

Example: **Symptom:** each Windows key acts twice. **Cause:** a handler processes
both press and release events. **Fix:** filter on `KeyEventKind::Press` in the
input boundary. **Prevented by:** a scripted press/release test, if present.

## Concept

```markdown
### <name>
**Is:** <what this component actually does>
**Is not:** <nearest confusing responsibility>
**Surprises:** <specific gotchas>
**Where:** <real entry point, when documenting an actual component>
```

Example: a decoder validates external media and produces a checked raster. It is
not a scene renderer or a source of game rules. Its dimensions and colour space
are part of its contract. An actual page names its constructor and error type.

## Workflow

```markdown
### <task in the reader's words>
**Before you start:** <required inputs and authority>
**Steps:** <ordered imperative actions>
**Done when:** <observable end-to-end result>
**When it breaks:** <first evidence to inspect>
```

Example: before comparing terminal portraits, obtain an allowed source image and
a known font. Render the same image at the intended cell sizes. Save a contact
sheet and inspect the smallest size. Done means the subject remains distinct,
not merely that PNG export succeeds. If it fails, inspect sampled colour masses
and glyph coverage before changing the prompt.

## Decision

```markdown
### <ID if used>: <choice>
**Context:** <dated situation and source>
**Options:** <seriously considered alternatives>
**Choice:** <choice and deciding reason>
**Accepted costs:** <real concession>
**What we did not know:** <unknowns at decision time, not backfilled later>
```

Example: an explicitly accepted record could choose atomic file replacement over
in-place writes. Its accepted costs include temporary disk space, and its unknowns
include platform durability semantics not yet measured. This is an example, not
a persistence requirement of this repository or an accepted decision. Supersede an actual
accepted record when its choice changes; edit mutable current guidance in place.
