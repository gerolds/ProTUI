# Theater patterns: how this process gets faked

Each pattern below is a way to produce artifacts that satisfy the letter of
the process while the decision was actually made before the process began.
During the Phase 6 audit, check your own artifacts against every one of
these. The checker script cannot detect most of them — only you can.

## 1. Retro-fitted rubric
**Shape:** the rubric's weights happen to align exactly with the eventual
winner's strengths, and its kill conditions happen to hit only the losers.
**Detection:** for each H-weighted criterion, ask "would I have written this
weight before knowing the options?" Check the ledger for rubric amendments
after Phase 3 — each one is a suspect. A rubric where the winner scores top
on every H item is statistically suspicious; real decisions involve losing
on something (that's what the concession section is for).
**Repair:** re-derive weights from constraint IDs alone, blind to options,
and rescore.

## 2. Decoy options (strawmanning)
**Shape:** two of the three options are obviously bad — over-engineered,
mis-scoped, or missing an ingredient any competent proposer would include.
**Detection:** read each losing option and ask "would a smart advocate of
this approach recognize their position here?" If a losing option can be
demolished in one sentence, it was a decoy.
**Repair:** rewrite the option as its best advocate (steelman), re-critique.

## 3. Generic critique / severity laundering
**Shape:** every option gets objections, so the critique *looks* balanced,
but the winner's objections are vague ("some added complexity") while
losers' are concrete ("violates C3 under load"). Severity is laundered
through word choice rather than analysis.
**Detection:** count ID citations per critique; compare specificity of the
winner's FATAL-IF against the losers'. If the winner's fatal condition is
one you'd never test for and the losers' are ones you immediately verified,
that asymmetry is the tell.
**Repair:** rewrite the winner's critique targeting its load-bearing
assumptions (A#) specifically.

## 4. Resolution hand-waving
**Shape:** Phase 5 dismisses a FATAL-IF with "this is unlikely in practice"
or "can be mitigated" without saying how, or what evidence supports the
probability judgment.
**Detection:** every FATAL-IF resolution must contain either evidence, a
concrete mitigation with its cost, or an explicit "unknown — this is a
revisit condition." Anything else is hand-waving.
**Repair:** downgrade confidence, move the condition into the revisit list,
or change the decision.

## 5. Hybrid escape hatch
**Shape:** recommending "a combination of A and B" to avoid conceding
anything. Hybrids feel safe but often inherit both parents' fatal
objections while the decision file resolves neither.
**Detection:** does the hybrid section resolve the FATAL-IF of *each*
parent? Does it introduce integration cost as a new liability, and is that
scored?
**Repair:** either do the full inheritance analysis or pick a parent.

## 6. Constraint smuggling
**Shape:** the decision cites a consideration ("the team knows Postgres")
that appears nowhere in the Phase 1 inventory — it was smuggled in at
decision time, exactly when it can no longer be adversarially examined.
**Detection:** every noun phrase doing justificatory work in `05_decision.md`
should trace to a C/A/U/F/R ID. Grep for justifications without IDs.
**Repair:** add it to the inventory via AMEND, let the critique phase react
to it (does it change any option's critique?), then re-decide.

## 7. Vagueness as armor
**Shape:** the winning option is specified at a higher level of abstraction
than the losers, so concrete objections can't attach to it. "Event-driven
architecture" can't fail; "Kafka with exactly-once semantics for order
events" can.
**Detection:** compare word counts and concreteness of mechanism sections
across options. The unfalsifiable option is the armored one.
**Repair:** force all options to the same level of mechanism detail before
critique.

## 8. Sealed-lean anchoring
**Shape:** subtler than peeking — you remember what you wrote in Phase 0
and steer everything toward it without ever opening the file.
**Detection:** honestly hard. Proxy: in Phase 3, was the lean's approach the
*first* option you wrote, and did it get the most detailed advocacy? Did
its critique arrive shortest?
**Repair:** write the lean's option file *last* and give it the *longest*
critique. Order is a cheap debiasing lever — use it.

## 9. Audit as ceremony
**Shape:** Phase 6 says "the decision matched my initial lean, and all
objections were addressed in Phase 5" without quoting the resolutions.
**Detection:** the audit must contain verbatim quotes of each FATAL-IF and
its specific resolution. A summary is not an audit.
**Repair:** do the line-by-line version. If it feels tedious, that is the
tedium doing its job.

## The meta-pattern

All nine reduce to one move: **transferring information backward in time** —
letting knowledge of the preferred answer contaminate artifacts that are
supposed to precede it. The process's defenses (pre-registration, blind
critique, sealed lean, ledger) all work by cutting backward channels. When
you find a new way to cheat that isn't on this list, it will be a new
backward channel. Name it, add it here, and cut it.