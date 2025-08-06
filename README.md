# progpick

Bruteforce with a stream of permutations of a specific pattern. Also prints a
progress bar and calculates an ETA.

In case you tend to forget your LUKS password as well.

# Examples

```sh
# This is going to allocate 3GB
bash -c 'for x in {a..z}{a..z}{a..z}{a..z}{a..z}; do echo $x; done'
# this is not
progpick '{a..z}{a..z}{a..z}{a..z}{a..z}'

# With progress bar
progpick '{a..z}{a..z}{a..z}{a..z}{a..z}' > /dev/null
# Without progress bar
progpick -q '{a..z}{a..z}{a..z}{a..z}{a..z}' > /dev/null

# regular expression
[a-z]{2-4}[0-9]{2}
# progpick pattern
progpick '{{a..z},}{{a..z},}{a..z}{a..z}{0..9}{0..9}'

# Run a script for each result
progpick 'a{b,c{d,e{f,g}}}' | while read -r x; do
    ./script "$x"
done
# Send the result to stdin
progpick -e './script.sh' 'a{b,c{d,e{f,g}}}'

# Attempt to open a luks partition
sudo progpick -e 'cryptsetup open --test-passphrase /dev/sdc1' 'a{b,c{d,e{f,g}}}'

# Recover the passphrase of your gpg secret key
progpick -e 'env GNUPGHOME=/backup/.gnupg/ gpg --batch --passphrase-fd 0 --pinentry-mode loopback --export-secret-keys YOUR_FINGERPRINT' "$(cat pattern.txt)"
```

# License

GPLv3+
