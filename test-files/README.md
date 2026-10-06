# Test files

Small Guitar Pro files written to exercise one feature each (bends, slides,
tuplets, repeats...), in every format version: `.gp3`, `.gp4`, `.gp5`, `.gpx`
(Guitar Pro 6) and `.gp` (Guitar Pro 7).

They come from the test data of [alphaTab](https://github.com/CoderLine/alphaTab)
(`packages/alphatab/test-data/`), by Daniel Kuschny and contributors, under the
Mozilla Public License 2.0: see `LICENSE-alphaTab.txt`. The `playback-*.gp5`
files are its `audio/` repeat fixtures, renamed.

`gold-generated-midi/` holds the MIDI events built from each file, checked by
the tests; a missing one is written on the next run.

No song transcriptions are kept here: they are copyrighted.
