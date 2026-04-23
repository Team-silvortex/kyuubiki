# System Surfaces

This directory contains runtime/configuration/data-administration surfaces for
the frontend workbench, including the system section shell.

Reusable runtime metrics and observer cards should prefer this directory over
growing `workbench.tsx`.

When a full runtime subsection becomes stable enough, prefer giving it its own
panel shell here instead of rebuilding the same card composition inside the
main workbench file.
