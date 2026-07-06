SHELL := /bin/sh
ENTRYPOINT := ./scripts/kyuubiki

include make/help.mk
include make/checks.mk
include make/tests.mk
include make/benchmarks.mk
include make/build.mk
include make/runtime.mk
include make/desktop.mk
include make/misc.mk
