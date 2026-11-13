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
| 2.01 | Point events | next step |
| 2.02 | Sections (line events) | next step |
| 3.01 | Live layer of point events | after 2.01 |
| 3.02 | Live layer of sections | after 2.02 |
| 4.01 | Picket stakeout | after group 3 |
| 4.02 | Locate points on routes | after group 3 |
| 5.01 | Profile table | last step |
| 5.02 | Profile drawing | last step |

Tool 1.02 assembles a route from all features with the same ID, orders the
parts by their joints and sends routes with a gap or a branch to the error
table. Tool 1.03 takes the chainage from control points, from a ledger with
station equations or from the length along the axis, and writes the
chainage of the start and the end of every route.

## Check against the plugin

The check uses the file `fixtures/tool_runs.json` of Routeliner 0.5.2. The
plugin script `tools/make_tool_runs.py` builds it on demo 1.01 with seed 1
in UTM zone 40N on WGS 84, and the computation is the plugin core without
QGIS. An unchanged copy of the file lies in the `fixtures` folder of this
repository, and only the plugin rebuilds it.

| Tool | Runs | Field deviation | Vertex deviation | M deviation |
|---|---|---|---|---|
| 1.02 | 3 | 0 | 0 | no M |
| 1.03 | 3 | 0 | 0 | 5·10⁻¹¹ m |

The test `tests/tool_runs.rs` prints the numbers of the table when run with
`cargo test -- --nocapture`. The run tolerance is 10⁻⁹ for fields and
10⁻⁶ m for vertices and M, and the limit of the task is 0.001 m. The test
compares the `rl_message` explanations in Russian word for word.

Python rounds `round(x, 3)` by the exact binary value of the number and
gives an exact half to the even digit. The extension rounds the `rl_*` fields and
the ПК labels by the same rule, so a label like «ПК 14+71.24» matches the
plugin character for character.

The test `tests/rustgis.rs` checks the whole path through RustGIS. The demo
layers come in WGS 84 degrees without Z, as RustGIS sends them, and the
route lengths and the R3 chainage agree with the plugin run within
0.001 m.

## Limits of RustGIS 0.6.0

RustGIS sends layers in WGS 84 degrees without Z and without M. The
`rustgis-kit` crate converts them to metres of the UTM zone at the middle
of the input and returns the result in degrees. This conversion and the
design of RustGIS give four differences between the extension and the
plugin.

- Chainage from geometry M values does not work, because a RustGIS layer
  has no M. The fields «Chainage from the M values of the route geometry»
  and «Metres per M unit» are hidden in the RustGIS window, and a route
  without a ledger gets chainage by length.
- The extension computes the M values of the 1.03 result, but they do not
  reach the RustGIS layer. The chainage of the start and the end stays in
  the `st_from` and `st_to` fields.
- There is no tool 1.01, because it writes a folder with a GeoPackage and
  a raster, and the crate has neither a folder field nor an output as a set
  of layers.
- GeoJSON has no arcs, a route comes already as segments, and the
  extension measures the length along these segments. The plugin cuts an
  arc itself, at 0.02° per chord (`CURVE_ANGLE_DEG` in
  `layers/route_source.py`).

## Build and check

The `rustgis-kit` crate comes in as a git dependency from
[github.com/Valery35/rustgis-kit](https://github.com/Valery35/rustgis-kit)
at tag v0.1.4.

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
```

The `tools` section of `plugin.json` is built from the tool descriptions.
The test `plugin_json_is_current` checks the file against the descriptions,
and the environment variable `ROUTELINER_WRITE_MANIFEST=1` with
`cargo test` rewrites it.

The descriptions `fixtures/tools.json` and the runs `fixtures/tool_runs.json`
come from the plugin's `fixtures` folder unchanged. After the plugin
rebuilds its runs, both copies are replaced as a whole.

## Layout

| Folder or file | Contents |
|---|---|
| src/core | The core, a port of the plugin's `routeliner/core`. Stations, the chainage system with equations, route geometry with M, route assembly, event placement, error codes |
| src/tools | Tools on the values of a plugin run, reading routes, the ledger and control points |
| src/product.rs | The `rustgis-kit` product, the About command, the base of `plugin.json` |
| src/pyfmt.rs | Rounding and message templates the way the plugin's Python writes them |
| fixtures | Tool descriptions and plugin runs, an unchanged copy |
| tests | The check against the runs and the path through RustGIS |

## License

GNU GPL version 2 or later, as the Routeliner plugin.

- - -

Developed with the support of Inform++ LLC ([www.informpp.ru](https://www.informpp.ru)).

Plugin page: [github.com/Valery35/routeliner](https://github.com/Valery35/routeliner)

Routeliner for RustGIS v0.1.0
