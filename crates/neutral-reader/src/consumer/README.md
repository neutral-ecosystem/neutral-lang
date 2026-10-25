<!-- SPDX-License-Identifier: Apache-2.0 -->

# Current expression consumers

`consumer::expression::ExpressionConsumerBoundary` applies explicit report and
binding policy to a `ValidatedExpressionProject`. It borrows checked authority,
validates public contracts and associations, and shares cancellation and work
budgets across registration and evaluation. It does not execute external
backends, authorize effects, or change project identity.

`EntryRequest` and `EntrySelection` are the shared typed request for current
expression entry discovery. They identify an exact public nominal contract and
an explicit enumerate, unique, or root selection. The expression reader
validates the request against the `/3` project and exposes only public entries.

Run `cargo test -p neutral-reader expression::` for current expression reader
coverage. Raw composition catalogue inspection lives in `composition`; the
former `/2` complete-composition consumer boundary has been retired.
