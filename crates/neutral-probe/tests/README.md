<!-- SPDX-License-Identifier: Apache-2.0 -->

# Standalone probe tests

The expression tests construct current `/3` artifacts without linking the
compiler into the probe executable. They check public root selection,
redaction, current identities, bounded rendering, cancellation, allocation
failure, and rejection of historical or unknown artifact magic. The command
tests keep help, version, and argument handling stable.
