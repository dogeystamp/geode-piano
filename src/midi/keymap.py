#!/usr/bin/env python3
"""
Helper to generate keymaps.

Takes in data through stdin with this format:

[note name] [GND pin]
[n1 input pin] 
[n2 input pin]

Use cargo fmt to de-messify the output once pasted in the source.
"""

from dataclasses import dataclass
import sys


@dataclass
class Note:
    note_name: str
    gnd_pin: int
    n1_pin: int
    n2_pin: int


data: list[Note] = []
col_dedup: set[int] = set()
row_dedup: set[int] = set()

for i in range(88):
    note_name, gnd_pin = input().split()
    gnd_pin = int(gnd_pin)
    n1_pin = int(input())
    n2_pin = int(input())
    data.append(Note(note_name, gnd_pin, n1_pin, n2_pin))
    col_dedup.add(gnd_pin)
    row_dedup.add(n1_pin)
    row_dedup.add(n2_pin)

col_pins = list(col_dedup)
row_pins = list(row_dedup)

# [row][column]
mat = [["" for _ in range(len(col_pins))] for _ in range(len(row_pins))]

for d in data:
    # this is inefficient but whatever
    mat[row_pins.index(d.n1_pin)][col_pins.index(d.gnd_pin)] = f"N1({d.note_name})"
    mat[row_pins.index(d.n1_pin)][col_pins.index(d.gnd_pin)] = f"NOP"
    mat[row_pins.index(d.n2_pin)][col_pins.index(d.gnd_pin)] = f"N({d.note_name}, 64)"

empty_counter = 0

for i in range(len(mat)):
    for j in range(len(mat[i])):
        if mat[i][j] == "":
            mat[i][j] = "NOP"
            empty_counter += 1


print("[")
for col in mat:
    print(f"[{', '.join(col)}],")
print("]")

print(f"{len(row_pins)} rows, {len(col_pins)} cols", file=sys.stderr)
print(f"row pins: [{', '.join([str(i) for i in row_pins])}]", file=sys.stderr)
print(f"col pins: [{', '.join([str(i) for i in col_pins])}]", file=sys.stderr)
print(f"{empty_counter} empty cells", file=sys.stderr)
