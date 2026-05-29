# Muscii

Render sheet music as ASCII art in your terminal.


## Usage

Muscii reads [ABC notation](https://abcnotation.com/) and renders it as an
ASCII staff. Pass a file, or pipe ABC in on stdin:

```sh
cargo run -- examples/scale.abc
# or
cat examples/scale.abc | cargo run -- -
```

Given `examples/scale.abc`:

```abc
X:1
T:C Major Scale
K:C
CDEF GABc | cBAG FEDC |
```

it prints the tune's title and key followed by the rendered staff. The five
staff lines are framed by a box whose outer lines are drawn heavy. The staff is
a treble clef (bottom line E4, top line F5), so middle C sits one ledger line
below the staff and a C-major scale starts there and steps up (see the C Major
example below). Notes that fall on a line are drawn
as a `⬤` head; notes on a space fill the gap between the two lines with half
blocks (`▄▄` above, `▀▀` below), so a stepwise scale renders as a staircase.
Bar lines and ledger lines are drawn as needed. Notes are laid out one column
each (durations are not yet scaled), and chords stack into a single column.

Built on the [`abc-parser`](https://crates.io/crates/abc-parser) crate.


## Syntax

### Pure ASCII

```txt
                              ____
     _                       |____|
 ___( )__|_____|\____________|____|_______||
|___|/___|_____|_______|\__(0)__(0)_______||
|__/|____|___(0)_______|\_________________||
|_( | )__|___________(0)__________________||
|___|____|__________________________(_)___||
    |                                 |
                                      |
```


### Unicode

```txt
--------------------------
--------------------------
--██--▅▅--------------▅▅__
------▀▀--██--▅▅--██--▀▀--
--------------▀▀----------
```

```txt
                           ██
------------------------██-----|
__________________▄▄_▀▀________|
            ▁▁ ██ ▔▔           |
▔▔▔▔▔▔▔▔▔██▔▀▀▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔|
------██-----------------------|
▄▄_██__________________________|
▔▔
```

```txt
                            ▄▄
━━━━━━━━━━━━━━━━━━━━━━▄▄━⬛━▀▀━┓
────────────────▄▄─⬛─▀▀───────┫
──────────▄▄─⬛─▀▀─────────────┫
────▄▄─⬛─▀▀───────────────────┫
━⬛━▀▀━━━━━━━━━━━━━━━━━━━━━━━━━┛

```


#### C Major

```txt

┏━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━┓
┣──────────────────────▄▄─╂─▄▄─────────────────────┫
┣────────────────▄▄─⬤──▀▀─╂─▀▀─⬤──▄▄───────────────┫
┣──────────▄▄─⬤──▀▀───────╂───────▀▀─⬤──▄▄─────────┫
┗━━━━▄▄━⬤━━▀▀━━━━━━━━━━━━━┻━━━━━━━━━━━━━▀▀━⬤━━▄▄━━━┛
 ─⬤──▀▀                                       ▀▀─⬤──
```


### Different Circles:

Name | Sign
-----|------
Ideographic Number Zero | `───〇───`
Large Circle | `───◯───`
Black Large Circle | `───⬤───`
Bold White Circle | `───🞆───`
Bullseye | `───◎───`
Circled Digit One | `───①─②─Ⓐ─⓫─⓵───`
Circled Crossing Lanes | `───⛒───`
Circled White Star | `───✪───`
Circle with Horizontal Bar | `───⦵───`
Circled Open Centre Eight Pointed Star | `───❂───`
Combining Enclosing Circle Backslash | `───⃠───`
N-Ary Circled Dot Operator | `───⨀───`
N-Ary Circled Times Operator | `───⨂───`
N-Ary Circled Plus Operator | `───⨁───`
Large Red Circle | `───🔴───`
Heavy Large Circle | `───⭕───`
Heavy Circle with Circle Inside | `───⭗───`
Heavy Circled Saltire | `───⭙───`
Combining Enclosing Circle | `── ⃝ ───`
Sun | `───☉───`
New Moon with Face | `───🌚───`
New Moon Symbol | `───🌑───`
Globe with Meridians | `───🌐───`
Full Moon Symbol | `───🌕───`
Sun with Face | `───🌞───`
Earth Globe Europe-Africa | `───🌍───`
