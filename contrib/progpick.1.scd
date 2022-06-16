progpick(1)

# NAME

progpick - Bruteforce with a stream of permutations of a specific pattern

# SYNOPSIS

*progpick* [-vqc] [-e _cmd_] _PATTERN_

# DESCRIPTION

*progpick* is a program to generate permutations of an explicitly provided
pattern. It's useful if you forgot some details about a passphrase but you
still remember most of it.

# OPTIONS

*-v*, *--verbose*
	Verbose logs (can be used multiple times, maximum: 4)

*-q*, *--quiet*
	Do not print progress bar

*-c*, *--count*
	Count total number of permutations instead of printing them

*-e* _cmd_, *--exec* _cmd_
	Send permutations to stdin of a subprocess. _cmd_ is parsed into a list of
	arguments and then executed directly instead of going through /bin/sh, so
	shell quoting works, but other shell features don't.

*-h*, *--help*
	Prints help information.

# EXAMPLES

Generate all combinations from a-z with a length of 5

	*progpick '{a..z}{a..z}{a..z}{a..z}{a..z}'*

With progress bar

	*progpick '{a..z}{a..z}{a..z}{a..z}{a..z}' > /dev/null*

Without progress bar

	*progpick -q '{a..z}{a..z}{a..z}{a..z}{a..z}' > /dev/null*

To make a character optional

	*progpick '{a..z}{{a..z},}'*

From a-z and 0-9

	*progpick '{{a..z},{0..9}}'*

Run a script for each result

	*progpick 'a{b,c{d,e{f,g}}}' | while read -r x; do*
		*./script "$x"*
	*done*

Send the result to stdin of a script

	*progpick -e './script.sh' 'a{b,c{d,e{f,g}}}'*

Attempt to open a luks partition

	*sudo progpick -e 'cryptsetup open --test-passphrase /dev/sdc1' 'a{b,c{d,e{f,g}}}'*

# AUTHORS

This program was originally written and is currently maintained by kpcyrd.
Bug reports and patches are welcome on github:

	_https://github.com/kpcyrd/progpick_
