# Routeliner для RustGIS 0.1.0 - текст выпуска на GitHub

Метка rustgis-v0.1.0, заголовок «Routeliner для RustGIS 0.1.0», к выпуску прикладывается пакет `routeliner-0.1.0.rgplugin`.

## Текст

Routeliner для RustGIS ставит события на маршруты и ведёт пикетаж в RustGIS так же, как модуль QGIS Routeliner. У расширения своя вкладка ленты, на ней в группах «Маршруты», «События», «Пикетаж» и «Профиль» лежат восемь инструментов модуля.

- 1.02 и 1.03 собирают маршрут из перепутанных частей и калибруют его по ведомости с пикетажными уравнениями или по контрольным точкам.
- 2.01 и 2.02 ставят точечные события и участки по пикетам со смещением от оси.
- 4.01 ставит точки целых пикетов, а 4.02 находит для точки ближайший маршрут, пикет и смещение.
- 5.01 собирает таблицу продольного профиля, а 5.02 строит по ней чертёж профиля с сеткой, шкалой отметок и развёрнутым планом.

Каждая запись, которую нельзя поставить, попадает в таблицу ошибок с причиной. Номера инструментов, имена полей и коды ошибок те же, что у модуля QGIS, а расчёт сверен с модулем на его же примере.

В RustGIS у слоёв нет M и Z, а у расширений нет растров. Поэтому пикетаж из M, длина по 3D и отметки рельефа на профиле пока работают только в модуле QGIS.

Расширение ставится кнопкой «Install From File» в окне Plugins › Plugin Manager RustGIS из файла `routeliner-0.1.0.rgplugin`. Нужен RustGIS 0.5.0 или новее.

## Text (English)

Routeliner for RustGIS places events on routes and keeps chainage in RustGIS the way the Routeliner plugin for QGIS does. The extension has a ribbon tab of its own with eight plugin tools in the groups «Routes», «Events», «Chainage» and «Profile».

- 1.02 and 1.03 assemble a route from shuffled parts and calibrate it by a ledger with station equations or by control points.
- 2.01 and 2.02 place point events and sections by stations with an offset from the axis.
- 4.01 places the points of whole pickets, and 4.02 finds the nearest route, the station and the offset of a point.
- 5.01 builds a longitudinal profile table, and 5.02 draws the profile from it with a grid, an elevation scale and a straightened plan.

Every record that cannot be placed goes to the error table with its reason. The tool numbers, field names and error codes are those of the QGIS plugin, and the computation is checked against the plugin on its own demo.

RustGIS layers have no M and no Z, and extensions have no rasters. So chainage from M, 3D length and terrain elevations on the profile work only in the QGIS plugin for now.

Install it with «Install From File» in Plugins › Plugin Manager of RustGIS, file `routeliner-0.1.0.rgplugin`. RustGIS 0.5.0 or later is needed.
