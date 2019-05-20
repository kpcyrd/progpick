# progpick

Bruteforce with a stream of permutations of a specific pattern.

In case you tend to forget your LUKS password as well.

# Examples

    # This is going to allocate 3GB
    bash -c 'for x in {a..z}{a..z}{a..z}{a..z}{a..z}; do echo $x; done'
    # this is not
    progpick '{a..z}{a..z}{a..z}{a..z}{a..z}'

    # regular expression
    [a-z]{2-4}[0-9]{2}
    # progpick pattern
    progpick '{{a..z},}{{a..z},}{a..z}{a..z}{0..9}{0..9}'

    # Run a script for each result
    progpick 'a{b,c{d,e{f,g}}}' | while read -r x; do
        ./script "$x"
    done

# TODO

- implement some of the flags in main.rs
- add progress bar
- estimate remaining time

# License

GPLv3+
