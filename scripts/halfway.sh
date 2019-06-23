#!/bin/sh
read x
case "$x" in
    n)
        exit 0
        ;;
    *)
        sleep 1
        exit 1
        ;;
esac
