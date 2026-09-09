# ADR 0011: native storefront polish

Accepted implementation direction, 9 September 2026. Tom requested the level
of polish of the macOS App Store and authorised prioritising Discover and one
complete app-detail-to-installation journey before catalogue expansion.

The visual direction is a quiet editorial storefront: warm neutral canvas,
forest-green feature surface, system typography, consistent compact actions
and app rows. StoreTheme owns the shared colour, size and spacing roles.
Native Qt controls retain keyboard and accessibility behaviour. Feedback is
immediate; this pass adds no animated movement or transparency dependency.

The functional signature is browsing by purpose: category shortcuts such as
“Write something” and “Everyday essentials” lead to actual filtered results.
The featured OmaCalc links to its real catalogue identity and shows a pinned,
attributed upstream screenshot under MIT. It is labelled as catalogue content,
not a reviewed editorial pick. Empty editorial shelves are omitted until
approved stories exist. Category symbols are not publisher logos.

Details lead with identity, price/licence/version, save/source actions and a
read-only installation review. Evidence and unavailable distribution remain
visible; reporting tools are an explicit disclosure lower down. The existing
server-side eligibility, execution confirmation and commerce boundaries remain
in force. The installation sheet groups requirements and retains recovery.

The browse scroll position survives an app-detail round trip. QA exercises
the real catalogue feature, saving, blocked installation review and return,
separately from the fictional commerce/lifecycle suite. Screenshots are actual
native offscreen captures; Omarchy desktop acceptance remains a separate gate.

Scope: B03/B12 presentation and navigation. This does not claim the full store
has reached Apple's finish: wider catalogue artwork, all other screens and
real desktop acceptance need further passes.
