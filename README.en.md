# Routeliner for RustGIS

Routeliner for RustGIS places events on routes and keeps chainage in
RustGIS the way the Routeliner plugin for QGIS does. The tool numbers, the
`rl_*` field names and the error codes are the same in the extension and
in the plugin. We port the plugin's computation to Rust step by step and
check every tool against the plugin's runs.

The Russian version of this text, with the same sections and tables, is
[README.md](README.md).

## Tools

| Code | Tool | In the extension |
|---|---|---|
| 1.01 | Demo example | no, the reason is in the section on limits |
| 1.02 | Route check | yes |
| 1.03 | Calibrate routes | yes |
| 2.01 | Point events | yes |
| 2.02 | Sections (line events) | yes |
| 3.01 | Live layer of point events | no, 2.01 gives the same computation |
| 3.02 | Live layer of sections | no, 2.02 gives the same computation |
| 4.01 | Picket stakeout | yes |
| 4.02 | Locate points on routes | yes |
| 5.01 | Profile table | yes, without rasters |
| 5.02 | Profile drawing | yes, grid rows by the template |

In RustGIS the extension has a ribbon tab of its own, Routeliner, with the
groups «Routes», «Events», «Chainage», «Profile» and «Help». Every tool
group holds one menu with an icon, and the «Help» group holds the About
button. The `rustgis-kit` crate builds the tab from the `TAB` description
in `src/product.rs`.

Tool 1.02 assembles a route from all features with the same ID, orders the
parts by their joints and sends routes with a gap or a branch to the error
table. Tool 1.03 takes the chainage from control points, from a ledger with
station equations or from the length along the axis, and writes the
chainage of the start and the end of every route.

Tools 2.01 and 2.02 place the records of a table on the routes by a station
or by a pair of stations, with an offset from the axis. Tool 4.01 places the
points of whole pickets with the equations taken into account, and 4.02
finds the nearest route, the measure, the station and the offset of a
point. The result of events and locating repeats the fields of the source
record and adds the `rl_*` fields, and a record that cannot be placed goes
to the error table together with its fields.

Tool 5.01 collects the points of a longitudinal profile at the start and
the end of a route, at equations, whole pickets, axis vertices, at a
constant step and at events within a corridor from the axis. Tool 5.02
builds from this table a profile drawing in paper millimetres with an
elevation scale, a grid of rows and a straightened plan. The plugin
computes the grid row expressions as QGIS expressions, while the extension
parses them itself. It understands numbers, table fields, the four
arithmetic operations, parentheses and unary minus.

## Check against the plugin

The check uses the file `fixtures/tool_runs.json` of Routeliner 0.5.2. The
plugin script `tools/make_tool_runs.py` builds it on demo 1.01 with seed 1
in UTM zone 40N on WGS 84, and the computation is the plugin core without
QGIS. An unchanged copy of the file lies in the `fixtures` folder of this
repository, and only the plugin rebuilds it.

| Tool | Plugin runs |
|---|---|
| 1.02 | default, with gaps, in the reverse direction |
| 1.03 | ledger with chainage in M, ledger with measure in M, control points |
| 2.01 | ledger, chainage in M, offset to the right |
| 2.02 | ledger, chainage in M, measure in M |
| 4.01 | step 100 m with the ledger, step 500 m with chainage in M, step 100 m by length |
| 4.02 | radius 50 m, radius 5 m |
| 5.01 | demo without raster with events, all vertices of R1 and R3 |
| 5.02 | drawing of R1 from the 5.01 table with terrain and the 4.02 plan |

On all 20 runs the fields agree without deviation, the vertices differ by
no more than 6·10⁻¹¹ m and M by no more than 5·10⁻¹¹ m. The test
`tests/tool_runs.rs` prints these values when run with
`cargo test -- --nocapture`. The run tolerance is 10⁻⁹ for fields and
10⁻⁶ m for vertices and M, and the limit of the task is 0.001 m. The test
compares the `rl_message` explanations in Russian word for word.

Python rounds `round(x, 3)` by the exact binary value of the number and
gives an exact half to the even digit. The extension rounds the `rl_*` fields and
the ПК labels by the same rule, so a label like «ПК 14+71.24» matches the
plugin character for character.

The test `tests/rustgis.rs` checks the whole path through RustGIS for 1.02,
1.03, 2.01, 4.02 and for 5.01 followed by 5.02. The demo layers come in WGS 84 degrees without Z, as
RustGIS sends them, and the lengths, measures, coordinates and offsets agree
with the plugin run within 0.001 m.

## Limits of RustGIS 0.6.0

RustGIS sends layers in WGS 84 degrees without Z and without M. The
`rustgis-kit` crate converts them to metres of the UTM zone at the middle
of the input and returns the result in degrees. This conversion and the
design of RustGIS give seven differences between the extension and the
plugin.

- Chainage from geometry M values does not work, because a RustGIS layer
  has no M. The fields «Chainage from the M values of the route geometry»
  and «Metres per M unit» are hidden in the RustGIS window, and a route
  without a ledger gets chainage by length.
- The extension computes the M values of the 1.03, 2.01, 2.02 and 4.01
  results, but they do not reach the RustGIS layer. The chainage and the
  measure stay in the result fields.
- There is no tool 1.01, because it writes a folder with a GeoPackage and
  a raster, and the crate has neither a folder field nor an output as a set
  of layers.
- There are no live layers 3.01 and 3.02, because recomputing a layer on
  edits of routes and tables needs extension sessions, and these are not
  in a RustGIS release yet. Tools 2.01 and 2.02 give the same computation
  in one run.
- RustGIS extensions have no rasters, so 5.01 does not sample terrain
  elevations and gives no `z_*` fields. The `z_axis` field is empty,
  because a RustGIS layer has no Z. The elevations for the 5.02 drawing
  come from a numeric table field that the user picks in the tool window.
- The 5.02 window has neither the grid row matrix nor the drawing anchor
  point, because RustGIS has no such field kinds. The grid rows come from
  the «Pipeline» template, and the drawing is placed 100 mm below the
  table, as the plugin does without an anchor point.
- GeoJSON has no arcs, a route comes already as segments, and the
  extension measures the length along these segments. The plugin cuts an
  arc itself, at 0.02° per chord (`CURVE_ANGLE_DEG` in
  `layers/route_source.py`).

## Build and check

The `rustgis-kit` crate comes in as a git dependency from
[github.com/Valery35/rustgis-kit](https://github.com/Valery35/rustgis-kit)
at tag v0.1.10.

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
```

The `tools` section of `plugin.json` is built from the tool descriptions.
The test `plugin_json_is_current` checks the file against the descriptions,
and the environment variable `ROUTELINER_WRITE_MANIFEST=1` with
`cargo test` rewrites it.

The script `scripts\package.ps1` builds the package. It makes a release
build and calls the packaging script of the crate from the neighbouring
folder `..\rustgis-kit`. The `dist` folder then holds the `routeliner`
folder for «Add Plugin Folder…» and the package `routeliner-0.1.0.rgplugin`
for installing from a file.

```bash
powershell -File scripts/package.ps1
```

The descriptions `fixtures/tools.json` and the runs `fixtures/tool_runs.json`
come from the plugin's `fixtures` folder unchanged. After the plugin
rebuilds its runs, both copies are replaced as a whole.

## Layout

| Folder or file | Contents |
|---|---|
| src/core | The core, a port of the plugin's `routeliner/core`. Stations, the chainage system with equations, route geometry with M, route assembly, event placement, error codes |
| src/tools | Tools on the values of a plugin run, reading routes, the ledger, events and points |
| scripts | Building the `.rgplugin` package |
| .github/workflows | A Telegram message to the author about a published release, a file shared by the plugins of the line |
| site | Release and post texts |
| src/product.rs | The `rustgis-kit` product, the About command, the base of `plugin.json` |
| src/pyfmt.rs | Rounding and message templates the way the plugin's Python writes them |
| fixtures | Tool descriptions and plugin runs, an unchanged copy |
| tests | The check against the runs and the path through RustGIS |

## License

GNU GPL version 3 or later, the full text is in [LICENSE](LICENSE). The
Routeliner plugin comes under GPL version 2 or later, and the extension is
built with the `rustgis-kit` crate under Apache 2.0, which is compatible
with GPL version 3.

- - -

Developed with the support of Inform++ LLC ([www.informpp.ru](https://www.informpp.ru)).

Plugin page: [github.com/Valery35/routeliner](https://github.com/Valery35/routeliner)

Routeliner for RustGIS v0.1.0
