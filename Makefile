docs: contrib/progpick.1

contrib/%.1: contrib/%.1.scd
	scdoc < $^ > $@
