<!-- SPDX-License-Identifier: Apache-2.0 -->

# Expression transport

This directory owns the explicit complete `NIR-PROJECT-CBOR/3` codec. Its
22-position envelope retains typed expression trees, public templates, policy,
facts and original-source companions. Encoding requires complete reader authority;
decoding reconstructs every section and independently validates before publication.

The shared composition helpers retain the historical nominal/value/source tuple
grammar. This codec neither evaluates trees nor enables successor source capture.
