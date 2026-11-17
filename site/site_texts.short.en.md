Routeliner - route events and chainage for QGIS · Inform++

Idea

Tools

RustGIS

Installation

QGIS plugin · Processing

Records with a station land on the route axis

Install in QGIS

Source code

How it works

Every record is either on the map or in the error table

Chainage by the ledger

Chainage in M values

Eleven tools

A tool has the same number in the toolbox, the menu and the manual

The tools sit in the Processing Toolbox in the Routeliner group and are repeated in the Plugins - Routeliner menu under the same numbers. All of them work in models and in batch mode.

Without QGIS

Routeliner for RustGIS

What it does

How it differs from the plugin

Extension page

Release 0.1.1

Extension source code

Installation and documentation

The QGIS plugin repository or a ZIP archive

Manual

Work log

Version 0.5.4 in the repository

Releases on GitHub

GNU GPL v2 or later · QGIS 3.40 and newer

Developed by Inform++ LLC

Routeliner grows on the tasks of real enterprises. If your operation is missing a feature, write to us.

1. Preparation

Demo example

GeoPackage with reference answers and a self-check of the plugin.

Route check

Assembled routes, gaps and branches.

Calibrate routes

Routes with chainage in the M values of their vertices.

2. Events by station

Point events

Points by station with an offset from the axis.

Sections (line events)

Route pieces between two stations.

3. Live layers

Live layer of point events

A layer recalculated after edits of the source data.

Live layer of sections

The same for sections.

4. Chainage and locating

Picket stakeout

Points of whole pickets with station equations taken into account.

Locate points on routes

Measure, station and offset for every point.

5. Profiles

Profile table

Profile points with stations and raster levels.

Profile drawing

Longitudinal profile with a grid and a straightened plan for a print layout.

Routeliner for RustGIS - route events and chainage · Inform++

Tools

Check

Limits

Installation

RustGIS extension

Chainage and route events in RustGIS

Download release 0.1.1

Source code

Eight tools out of eleven

A ribbon tab of its own with the same numbers

The extension has a ribbon tab of its own, Routeliner, with the groups «Routes», «Events», «Chainage», «Profile» and «Help». The result of events and locating repeats the fields of the source record and adds the rl_* fields. A record that cannot be placed goes to the error table together with its fields.

Check against the plugin

Every tool is checked against the plugin runs

plugin runs over eight tools, the fields agree without deviation.

m is the largest vertex difference, M values differ by no more than 5·10⁻¹¹ m.

m is the task limit for the path through RustGIS in WGS 84 degrees, lengths, measures and offsets agree within it.

Limits of RustGIS 0.6.0

How the extension differs from the plugin

Installation

The package is installed from a file

Routeliner plugin for QGIS

RustGIS

GNU GPL v3 or later · RustGIS 0.6.0 and newer

Developed by Inform++ LLC

Routeliner grows on the tasks of real enterprises. If your operation is missing a feature, write to us.

1. Preparation

Demo example

Not in the extension. The tool writes a folder with a GeoPackage and a raster, and RustGIS extensions have neither a folder field nor an output as a set of layers.

Route check

Assembled routes, gaps and branches.

Calibrate routes

Routes with chainage in the M values of their vertices.

2. Events by station

Point events

Points by station with an offset from the axis.

Sections (line events)

Route pieces between two stations.

3. Live layers

Live layer of point events

Not in the extension. Recomputing a layer on edits needs extension sessions, and these are not in a RustGIS release yet. Tool 2.01 gives the same computation in one run.

Live layer of sections

Not in the extension. Tool 2.02 gives the same computation in one run.

4. Chainage and locating

Picket stakeout

Points of whole pickets with station equations taken into account.

Locate points on routes

Measure, station and offset for every point.

5. Profiles

Profile table

Profile points with stations. RustGIS extensions have no rasters, so there are no terrain levels.

Profile drawing

Longitudinal profile with a grid and a straightened plan. The grid rows come from the «Pipeline» template.
