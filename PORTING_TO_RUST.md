# Porting Construct to Rust: Step 1

This document collects information gathered from the repository to help decide which parts of the Python implementation should be ported to Rust first.

## Module overview

The main package is located in `construct` and contains:

- `core.py` – the core implementation of the `Construct` class hierarchy.
- `expr.py` – expression definitions used by various constructs.
- `debug.py` – debug helpers wrapping constructs.
- `version.py` – version metadata.
- `lib/` – utility modules:
  - `binary.py` – integer/byte/bit conversions and endianness helpers.
  - `bitstream.py` – classes for stream‑oriented bit access.
  - `containers.py` – custom container types used throughout the library.
  - `hex.py` – helpers for hex dumps and string formatting.
  - `py3compat.py` – compatibility helpers for Python 2/3 differences.

Additional folders like `gallery` and `deprecated_gallery` provide usage examples and protocol definitions. They are not required by the core library.

## Dependencies

`setup.py` specifies an empty `install_requires` list. The code relies only on the Python standard library.

## Public API

The list of exported names is defined in `construct/__init__.py` via the `__all__` variable. These names represent the official API and must remain accessible to Python when the Rust implementation is introduced.

## Candidate components for an initial Rust port

To keep the scope manageable, the following areas could be ported first:

1. Utility functions in `construct/lib/*.py`.
2. Performance‑critical parts of `construct/core.py` that parse and build primitive fields.

By re‑implementing these parts in Rust while preserving the Python API, we can begin improving performance without rewriting the entire library at once.

