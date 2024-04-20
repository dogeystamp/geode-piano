#!/usr/bin/env python3

"""Generates the MIDI `Note` enum based on https://gist.github.com/dimitre/439f5ab75a0c2e66c8c63fc9e8f7ea77."""

import csv

with open("note_freq_440_432.csv") as f:
    reader = csv.reader(f)
    for row in reader:
        note_code: int = int(row[0])
        note_name: str = row[1]
        octave: int = int(row[2])

        identifier = f"{note_name.replace('#', 'S')}{octave}"
        print(f"{identifier} = {note_code},")
