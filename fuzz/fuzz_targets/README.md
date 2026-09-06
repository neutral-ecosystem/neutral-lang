<!-- SPDX-License-Identifier: Apache-2.0 -->

# Fuzz targets

Each target drives one untrusted-input or consumer boundary and accepts no host
authority. Target bodies rely on production limits so arbitrary bytes cannot
silently become authoritative partial output.
