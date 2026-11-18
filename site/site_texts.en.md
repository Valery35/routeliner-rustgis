Routeliner places table records on the axes of roads, tracks, pipelines and power lines when a record carries a station instead of coordinates.

for QGIS

Routeliner places table records on the axes of roads, tracks, pipelines and power lines when a record carries a station instead of coordinates. The plugin reads the five station notations accepted in the CIS countries and takes into account a ledger with broken pickets and station equations.

The plugin never skips a record silently. A record that cannot be placed goes to the error table with a reason code. A longitudinal profile with raster levels, grades and ledger stations is built along a route.

Broken pickets and station equations are taken into account in events, stakeout, point locating and the profile. The chainage comes from the ledger, from control points or from the length along the axis.

The route chainage can be taken from the M values of its geometry and written into the M of the results. PostGIS, ArcGIS and the QGIS tools for M read such layers.

Eight tools of the plugin run in RustGIS, a desktop GIS written in Rust. The computation repeats the plugin and is checked against all 20 of its runs. QGIS is not needed.

Route check and calibration, point events and sections, picket stakeout, point locating, the longitudinal profile table and drawing. The tool numbers, the result fields and the error codes are the same as in the plugin.

The extension has its own RustGIS ribbon tab. RustGIS 0.6.0 has no live layers, no demo example, no rasters and no M values. RustGIS 0.6.0 or newer and Windows are required.

The plugin is installed from the QGIS plugin repository or from a ZIP archive through Plugins - Manage and Install Plugins - Install from ZIP. It requires QGIS 3.40 or newer, QGIS 4 on Qt6 included. There are no external dependencies, the computation runs on the NumPy that ships with QGIS. The interface language follows the language of QGIS.

The PDF manual opens from Plugins - Routeliner - Manual (PDF) and describes the tool parameters and the fields of every layer the plugin creates. The manual comes in Russian and English.

The About button on the Routeliner toolbar shows the version, the version history and the work log. The log is written to routeliner.log in the QGIS profile folder, and in case of an error it is enough to attach it to a message.

Routeliner for RustGIS places events on routes and keeps chainage in RustGIS the way the Routeliner plugin for QGIS does.

for RustGIS

Routeliner for RustGIS places events on routes and keeps chainage in RustGIS the way the Routeliner plugin for QGIS does. The tool numbers, the result field names and the error codes are the same in the extension and in the plugin. QGIS is not needed.

The check uses the runs of Routeliner 0.5.4 on demo 1.01 with seed 1 in UTM zone 40N. The explanation texts are compared in Russian word for word, and a label like «ПК 14+71.24» matches the plugin character for character.

RustGIS sends layers in WGS 84 degrees without Z and without M. The extension computes in metres of the UTM zone at the middle of the input and returns the result in degrees. This and the design of RustGIS give the following differences.

Chainage from geometry M values does not work, a RustGIS layer has no M. A route without a ledger gets chainage by length.

The extension computes the M values of the 1.03, 2.01, 2.02 and 4.01 results, but they do not reach the RustGIS layer. The chainage and the measure stay in the result fields.

RustGIS extensions have no rasters, so 5.01 does not sample terrain levels. The levels for the 5.02 drawing come from a numeric table field.

The 5.02 window has neither the grid row matrix nor the drawing anchor point. The grid rows come from the «Pipeline» template, and the drawing is placed 100 mm below the table.

GeoJSON has no arcs, a route comes as segments, and the length is measured along them.

The package routeliner-0.1.1.rgplugin from the release page is installed with Install From File in the Plugin Manager window. RustGIS 0.6.0 or newer and Windows are required. The demo layers of the QGIS plugin are good for a check.
