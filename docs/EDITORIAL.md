# Makers, editorial and release feeds

The public catalogue carries maker attribution and optional scheduled stories or picks. Records have stable IDs and revisions, attributed makers, linked app IDs, UTC visibility windows and an explicit rights statement. A maker profile reports scoped project-control verification only while its recorded verification interval is current. This is not personal identity, seller verification or compatibility evidence. Unclaimed community listings remain explicit.

`GET /api/v1/makers`, `/api/v1/makers/{id}` and `/api/v1/editorial` expose bounded native read models. Editorial candidates use the same immutable revision, review and exact-publication authority as app submissions. Editorial decisions additionally require a current editor role; project conflict checks still apply. Scheduling controls visibility after approved delivery; it cannot publish a draft by itself.

`GET /feed.xml` and `/makers/{id}/feed.xml` return RSS 2.0 with ETags. Entries are generated in the transaction that records publicly observed delivery. Each app/release pair has a stable non-permalink GUID and first delivery timestamp. Correcting a name or summary updates that entry without announcing a new release. Entries link to native `omastore://app/{id}` identities. Feeds describe releases; current distribution and compatibility must still be checked separately.

B10a implements these contracts. Native maker pages, editorial authoring and trusted published-context submission are the next slice. No real publisher or editorial selection has been added to the public catalogue.
