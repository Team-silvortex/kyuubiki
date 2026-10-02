.PHONY: tree disk-usage clean-dev-cache

tree:
	@find . -maxdepth 3 -type d | sort

disk-usage:
	@$(ENTRYPOINT) dev-disk --test-binaries

clean-dev-cache:
	@$(ENTRYPOINT) dev-disk --apply --test-binaries
