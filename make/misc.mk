.PHONY: tree

tree:
	@find . -maxdepth 3 -type d | sort
