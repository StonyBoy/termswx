# ANSI Escape Sequences examples

These were all captured from a prompt sent from a Linux console.

## CSI ? 2004 l: Turn off bracketed paste mode

    0x1b <ESC>
    0x5b [
    0x3f ?
    0x32 2
    0x30 0
    0x30 0
    0x34 4
    0x6c l

## CSI ? 2004 h: Turn on bracketed paste mode

    0x1b <ESC>
    0x5b [
    0x3f ?
    0x32 2
    0x30 0
    0x30 0
    0x34 4
    0x68 h

## CSI fg ; bg m : Reset, then set foreground to Red

    0x1b <ESC>
    0x5b [
    0x30 0
    0x3b ;
    0x33 3
    0x31 1
    0x6d m

## CSI fg ; bg m : Reset, then set foreground to Blue

    0x1b <ESC>
    0x5b [
    0x30 0
    0x3b ;
    0x33 3
    0x34 4
    0x6d m

## CSI fg ; bg m : Reset, then set foreground to Cyan

    0x1b <ESC>
    0x5b [
    0x30 0
    0x3b ;
    0x33 3
    0x36 6
    0x6d m

## CSI fg ; bg m : Reset, then set foreground to Blue

    0x1b <ESC>
    0x5b [
    0x30 0
    0x3b ;
    0x33 3
    0x34 4
    0x6d m

## CSI fg ; bg m : Reset, then set foreground to Green

    0x1b <ESC>
    0x5b [
    0x30 0
    0x3b ;
    0x33 3
    0x32 2
    0x6d m

## CSI fg ; bg m : Reset, then set foreground to Magenta

    0x1b <ESC>
    0x5b [
    0x30 0
    0x3b ;
    0x33 3
    0x35 5
    0x6d m

## CSI fg m : Reset

    0x1b <ESC>
    0x5b [
    0x30 0
    0x6d m

[modeline]: # ( vim: set ts=4 sw=4 sts=4 tw=80 cc=80 et ft=markdown : )
