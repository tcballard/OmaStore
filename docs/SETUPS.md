# Selective setups

A setup has a stable ID/revision, creator attribution, explicit sharing rights, optional parent ID/revision, app/release references, dependencies, conflicts, media and typed setting references. There are at most 100 components and 20 setting references. Unknown object fields are rejected. Optional added fields are omitted when absent so existing canonical content remains stable.

The read-only selection model starts with required components and includes the dependency closure of explicitly chosen optional components. Each row explains what requires it. Conflicting selections and references to an older app release remain visible and cannot silently substitute the current release. No model operation queries or changes the host, installs software or applies settings.

Costs are itemised. Free/donation offers have no required purchase amount. Fixed offers need a price check within 24 hours to contribute to a known estimate. Older/missing quotes and variable offers leave the total unknown. Different currencies, tax treatments and billing intervals are grouped separately; no exchange rate or combined grand total is invented. External checkout remains the final price authority.

Selection exports contain only stable public setup/app/release identities, attribution, rights and setting references. Import is bounded, rejects extra fields and compares the exact current published context. An unavailable revision or changed attribution requires review rather than substitution. Opening/importing/exporting a selection never authorises execution.

Authors submit recipes through the private draft, exact preview and independent-review path. Referenced app facts are frozen published context, preserving their existing evidence and media. Recipe media follows normalised private upload, immutable submitted-media access, reviewer inspection and exact publication. A new setup does not announce another release of its component apps. Changing an existing recipe requires its owner and a new revision; attributing it to an existing maker requires that maker's control.

B11 includes the native setup browser and component picker, required-dependency explanations, conflicts, separate price estimates, current distribution checks, verified media, creator links, exact export preview and native import/export dialogs. Authors can create a setup draft and choose current published app releases before private preview and review. Three real editorial recipe candidates require actual tested components and media; the development examples remain fictional.
