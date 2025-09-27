SHELL := /bin/bash
ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))

.PHONY: lint test build run evidence hooks.dry watchers.dry bootstrap

lint test build run evidence hooks.dry watchers.dry:
$(ROOT)/scripts/bootstrap.sh $@

bootstrap:
$(ROOT)/scripts/bootstrap.sh bootstrap
