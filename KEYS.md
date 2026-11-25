# beeswax — key reference

Navigation is vi-style; arrow keys, Home/End/PgUp/PgDn and the F-keys work
everywhere too, so nothing requires the vi bindings. `^` below means Ctrl.

**Unprefixed** — vi verbs and motions, acting on the item under the cursor.
**Shift** — the reverse, or the larger version, of the unshifted key.
**Ctrl** — whole-file and whole-view commands.
**Alt** — the structure around the items: sections, columns, category tree.

Structural editing follows VimOutliner, which leaves it to plain vim: `o`/`O`
makes a sibling, `>>`/`<<` changes its level, `u` and `Ctrl+R` undo and redo.

## Moving around

```
┌─────────────────────────────────┬──────────────────────────────────────────────────┐
│ Key                             │ Action                                           │
╞═════════════════════════════════╪══════════════════════════════════════════════════╡
│ k  j   ↑ ↓                      │ Up / down                                        │
│ h  l   ← →   Tab / Shift+Tab    │ Left / right across columns                      │
│ Enter / Shift+Enter             │ Down / up                                        │
│ PgDn / PgUp   ^F / ^B           │ One screenful down / up                          │
│ Home / End   { }                │ Section head / last item; again for the previous │
│                                 │ / next section                                   │
│ Shift+Home / Shift+End          │ Head of the previous / next section              │
│ ^Home / ^End   gg  G            │ Top / bottom of the view                         │
│ H  M  L                         │ First / middle / last visible row                │
│ zz   z Enter   z-               │ Put the cursor at center / top / bottom          │
│ ^PgUp / ^PgDn                   │ Previous / next view                             │
│ /  ?   then  n  N               │ Search forward / back, then repeat / reverse     │
└─────────────────────────────────┴──────────────────────────────────────────────────┘
```

## Items

```
┌─────────────────────────────────┬──────────────────────────────────────────────────┐
│ Key                             │ Action                                           │
╞═════════════════════════════════╪══════════════════════════════════════════════════╡
│ i   F2                          │ Edit item text or cell value                     │
│ o / O   Insert                  │ New item below / above                           │
│ x   Delete                      │ Remove from this section's category; on a head   │
│                                 │ remove the section or column; in a data column   │
│                                 │ clear the value                                  │
│ Shift+Delete                    │ Discard from all categories, permanently         │
│ u   ^Z                          │ Undo                                             │
│ ^R   ^Y                         │ Redo                                             │
│ F3 / F4 / F5                    │ Assign categories or pick a value / mark done /  │
│                                 │ note in $EDITOR                                  │
│ F6                              │ Item, section or column properties               │
│ F8 / F9 / F10                   │ View Manager / Category Manager / Menu           │
│ ^S   Alt+Q   F1                 │ Save / quit / help                               │
└─────────────────────────────────┴──────────────────────────────────────────────────┘
```

## Structure — View screen

```
┌─────────────────────────────────┬──────────────────────────────────────────────────┐
│ Key                             │ Action                                           │
╞═════════════════════════════════╪══════════════════════════════════════════════════╡
│ Alt+o                           │ New section: the Add dialog, with the category   │
│                                 │ picker already open                              │
│ Alt+c                           │ New column: the Add dialog, with the column-head │
│                                 │ picker already open                              │
│ Alt+h / Alt+l   Alt+← / Alt+→   │ Move this column left / right                    │
│ Alt+s                           │ Sort this section now                            │
└─────────────────────────────────┴──────────────────────────────────────────────────┘
```

Both dialogs carry the direction as a field rather than as a key: Down to the
Insert field (section: Below / Above) or the Position field (column: Right /
Left), then Left/Right to set it. F3 reopens the picker for a field.

## Structure — Category Manager (F9)

```
┌─────────────────────────────────┬──────────────────────────────────────────────────┐
│ Key                             │ Action                                           │
╞═════════════════════════════════╪══════════════════════════════════════════════════╡
│ o / O   Insert                  │ New sibling below / above                        │
│ >>   Alt+l   Alt+→              │ Demote one level — o then >> makes a child       │
│ <<   Alt+h   Alt+←              │ Promote one level                                │
│ Alt+j / Alt+k   Alt+↓ / Alt+↑   │ Move down / up among siblings                    │
│ Enter   F2   i                  │ Rename                                           │
│ Alt+F10                         │ Move mode: plain arrows, Enter when done         │
│ Delete   x                      │ Delete                                           │
│ F6                              │ Category properties                              │
└─────────────────────────────────┴──────────────────────────────────────────────────┘
```

On a root category `o` and `O` both create a first child, a root having no
siblings. Ctrl+arrows are aliases for the Alt keys above.

## Lists and pickers

The Category Manager, View Manager, F3 pickers, Quick Add, Section Select and
the sort and filter dialogs share one set: `k`/`j`, PgUp/PgDn, `gg`/`G`,
`H`/`M`/`L`, `/` to search, `i`/`o`/`x`/`u` to rename, create, delete and undo
where supported, Enter to confirm, Esc to cancel. In the View Manager and in
View Properties' Sections field, Alt+j / Alt+k reorder the list.
