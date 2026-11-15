# -*- coding: utf-8 -*-
# Routeliner для RustGIS - события на маршрутах и пикетаж.
# Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
# SPDX-License-Identifier: GPL-3.0-or-later
"""Две страницы Routeliner для www.informpp.ru.

    python -I site/build_site.py

Просьба автора 16.11.2026 - страницы модуля QGIS Routeliner и расширения
Routeliner для RustGIS по образцу Topoliner и Isoliner. Вид и переключатель
языка взяты со страницы Topoliner (topoliner/site/topoliner_landing.html),
тексты модуля - из его README.ru.md и README.md. Итог - два файла рядом
со сборщиком, код для блока «Встроить» Google Сайтов.
"""
import html
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))

MODULE_VERSION = "0.5.2"
RUST_VERSION = "0.1.1"
CATALOG = "https://plugins.qgis.org/plugins/routeliner/"
CATALOG_VERSION = CATALOG + "version/%s/" % MODULE_VERSION
MODULE_CODE = "https://github.com/Valery35/routeliner"
MODULE_RELEASES = MODULE_CODE + "/releases"
RUST_CODE = "https://github.com/Valery35/routeliner-rustgis"
RUST_RELEASE = RUST_CODE + "/releases/tag/rustgis-v" + RUST_VERSION
RUST_RELEASES = RUST_CODE + "/releases"
RUSTGIS = "https://github.com/opengeos/RustGIS"
SITE = "https://www.informpp.ru/"
ORDER = "https://www.informpp.ru/главная-страница/предприятиям"
PAGE_MODULE = "https://www.informpp.ru/главная-страница/qgis-routeliner"
PAGE_RUST = "https://www.informpp.ru/главная-страница/routeliner-rustgis"

CSS = """
@import url('https://fonts.googleapis.com/css2?family=Bitter:wght@500;600;700&family=Golos+Text:wght@400;500;600&display=swap');
:root{
  --paper:#F1F3EE; --paper-2:#E7EBE3; --ink:#16221F; --ink-soft:#4C5A55;
  --teal:#0E7C66; --teal-deep:#0A5446; --amber:#C2622C;
  --line:rgba(22,34,31,.14); --r:14px; --maxw:1080px;
  --display:'Bitter',Georgia,serif; --body:'Golos Text','Segoe UI',system-ui,sans-serif;
}
*{box-sizing:border-box}
html{scroll-behavior:smooth}
section[id],[id]{scroll-margin-top:74px}
body{margin:0;background:var(--paper);color:var(--ink);
  font-family:var(--body);font-size:17px;line-height:1.6;
  -webkit-font-smoothing:antialiased}
.wrap{max-width:var(--maxw);margin:0 auto;padding:0 24px}
a{color:inherit}
h1,h2,h3{font-family:var(--display);font-weight:700;line-height:1.1;margin:0;
  letter-spacing:-.01em}
p{margin:0}
.eyebrow{font-size:13px;font-weight:600;letter-spacing:.16em;
  text-transform:uppercase;color:var(--teal-deep);margin-bottom:18px;
  display:inline-flex;gap:10px;align-items:center}
.eyebrow::before{content:"";width:26px;height:2px;background:var(--amber);
  display:inline-block}
.hdr{position:sticky;top:0;z-index:20;background:rgba(241,243,238,.86);
  backdrop-filter:blur(8px);border-bottom:1px solid var(--line)}
.hdr .wrap{display:flex;align-items:center;justify-content:space-between;
  height:62px;gap:18px}
.brand{display:flex;align-items:baseline;gap:10px;font-family:var(--display);
  font-weight:700;text-decoration:none;font-size:20px}
.brand span{font-family:var(--body);font-weight:500;font-size:13px;
  color:var(--ink-soft)}
.right{display:flex;align-items:center;gap:22px}
.nav{display:flex;gap:22px;font-size:15px}
.nav a{text-decoration:none;color:var(--ink-soft)}
.nav a:hover{color:var(--teal-deep)}
@media(max-width:860px){.nav{display:none}}
.lang{display:flex;border:1px solid var(--line);border-radius:10px;
  overflow:hidden;font-size:13px;font-weight:600}
.lang button{border:0;background:transparent;padding:6px 11px;cursor:pointer;
  font:inherit;color:var(--ink-soft)}
.lang button.on{background:var(--teal);color:#fff}
.hero{padding:74px 0 48px}
.hero h1{font-size:clamp(34px,5vw,56px);max-width:18ch}
.hero .lead{margin-top:22px;font-size:20px;max-width:64ch;color:var(--ink-soft)}
.cta{margin-top:34px;display:flex;gap:14px;flex-wrap:wrap}
.btn{display:inline-block;padding:13px 22px;border-radius:var(--r);
  text-decoration:none;font-weight:600;font-size:16px}
.btn-main{background:var(--teal);color:#fff}
.btn-main:hover{background:var(--teal-deep)}
.btn-ghost{border:1px solid var(--line);color:var(--ink)}
.btn-ghost:hover{border-color:var(--teal)}
section{padding:56px 0;border-top:1px solid var(--line)}
section h2{font-size:clamp(26px,3.3vw,38px);max-width:24ch}
section .sub{margin-top:16px;max-width:70ch;color:var(--ink-soft)}
.pair{display:grid;grid-template-columns:1fr 1fr;gap:28px;margin-top:34px}
@media(max-width:860px){.pair{grid-template-columns:1fr}}
.card{background:var(--paper-2);border:1px solid var(--line);
  border-radius:var(--r);padding:22px 24px}
.card h3{font-size:19px;margin-bottom:10px}
.card p{color:var(--ink-soft);font-size:16px}
.tools{margin-top:34px;border:1px solid var(--line);border-radius:var(--r);
  overflow:hidden;background:#fff}
.tool{display:grid;grid-template-columns:88px 1fr;
  border-bottom:1px solid var(--line)}
.tool:last-child{border-bottom:none}
.tool .num{background:var(--paper-2);padding:16px 12px;font-weight:600;
  font-family:var(--display);color:var(--teal-deep);text-align:center}
.tool .txt{padding:16px 20px}
.tool .txt b{display:block;font-size:16px}
.tool .txt span{color:var(--ink-soft);font-size:15px}
.tool.off .num{color:var(--ink-soft)}
.tool.off .txt b{color:var(--ink-soft)}
.group{background:var(--paper-2);padding:10px 20px;font-size:13px;
  font-weight:600;letter-spacing:.1em;text-transform:uppercase;
  color:var(--teal-deep);border-bottom:1px solid var(--line)}
.facts{display:grid;grid-template-columns:repeat(3,1fr);gap:22px;margin-top:34px}
@media(max-width:860px){.facts{grid-template-columns:1fr}}
.fact{border-left:3px solid var(--amber);padding-left:16px}
.fact b{display:block;font-family:var(--display);font-size:30px;line-height:1.1}
.fact span{color:var(--ink-soft);font-size:15px}
.limits{margin:28px 0 0;padding-left:22px;max-width:76ch;color:var(--ink-soft)}
.limits li{margin-top:10px}
.ftr{padding:40px 0 60px;border-top:1px solid var(--line);
  color:var(--ink-soft);font-size:15px}
.ftr a{color:var(--teal-deep)}
"""

SCRIPT = """
var TEXTS = %s;
function apply(lang){
  var d = TEXTS[lang] || TEXTS.ru;
  document.documentElement.lang = lang;
  document.querySelectorAll('[data-i18n]').forEach(function(el){
    var value = d[el.getAttribute('data-i18n')];
    if (value !== undefined) el.innerHTML = value;
  });
  document.querySelectorAll('[data-i18n-attr]').forEach(function(el){
    var value = d[el.getAttribute('data-i18n-attr')];
    if (value !== undefined) el.setAttribute('content', value);
  });
  document.querySelectorAll('.lang button').forEach(function(b){
    b.classList.toggle('on', b.getAttribute('data-lang') === lang);
  });
  try { localStorage.setItem('%s', lang); } catch (e) {}
}
document.querySelectorAll('.lang button').forEach(function(b){
  b.addEventListener('click', function(){ apply(b.getAttribute('data-lang')); });
});
// По умолчанию русский, страница живёт на русском сайте.
var saved = null;
try { saved = localStorage.getItem('%s'); } catch (e) {}
apply(saved || 'ru');
"""

# Инструменты модуля - номер, группа, (имя, итог) по-русски и по-английски.
GROUPS = {
    "1": ("1. Подготовка", "1. Preparation"),
    "2": ("2. События по пикетам", "2. Events by station"),
    "3": ("3. Динамические слои", "3. Live layers"),
    "4": ("4. Пикетаж и привязка", "4. Chainage and locating"),
    "5": ("5. Профили", "5. Profiles"),
}
TOOLS = (
    ("1.01", "Демонстрационный пример",
     "GeoPackage с эталонными ответами и самопроверка модуля.",
     "Demo example",
     "GeoPackage with reference answers and a self-check of the plugin."),
    ("1.02", "Проверка маршрутов", "Собранные маршруты, разрывы и развилки.",
     "Route check", "Assembled routes, gaps and branches."),
    ("1.03", "Калибровка маршрутов",
     "Маршруты с пикетажем в M-значениях вершин.",
     "Calibrate routes",
     "Routes with chainage in the M values of their vertices."),
    ("2.01", "Точечные события", "Точки по пикету со смещением от оси.",
     "Point events", "Points by station with an offset from the axis."),
    ("2.02", "Участки (линейные события)",
     "Отрезки маршрута между двумя пикетами.",
     "Sections (line events)", "Route pieces between two stations."),
    ("3.01", "Динамический слой точечных событий",
     "Слой, который пересчитывается после правок исходных данных.",
     "Live layer of point events",
     "A layer recalculated after edits of the source data."),
    ("3.02", "Динамический слой участков", "То же для участков.",
     "Live layer of sections", "The same for sections."),
    ("4.01", "Пикетная разбивка", "Точки целых пикетов с учётом уравнений.",
     "Picket stakeout",
     "Points of whole pickets with station equations taken into account."),
    ("4.02", "Привязка точек к маршрутам",
     "Мера, пикет и смещение для каждой точки.",
     "Locate points on routes", "Measure, station and offset for every point."),
    ("5.01", "Таблица профиля", "Точки профиля с пикетами и отметками растров.",
     "Profile table", "Profile points with stations and raster levels."),
    ("5.02", "Чертёж профиля",
     "Продольный профиль с сеткой и развёрнутым планом для компоновки.",
     "Profile drawing",
     "Longitudinal profile with a grid and a straightened plan for a print layout."),
)

# Чего нет в расширении - номер и причина. Остальные инструменты есть.
MISSING = {
    "1.01": ("Нет в расширении. Инструмент пишет папку с GeoPackage и растром, "
             "а у расширений RustGIS нет поля папки и выхода набором слоёв.",
             "Not in the extension. The tool writes a folder with a GeoPackage "
             "and a raster, and RustGIS extensions have neither a folder field "
             "nor an output as a set of layers."),
    "3.01": ("Нет в расширении. Пересчёт слоя при правках требует сессий "
             "расширений, их ещё нет в выпуске RustGIS. Тот же расчёт одним "
             "запуском даёт 2.01.",
             "Not in the extension. Recomputing a layer on edits needs "
             "extension sessions, and these are not in a RustGIS release yet. "
             "Tool 2.01 gives the same computation in one run."),
    "3.02": ("Нет в расширении. Тот же расчёт одним запуском даёт 2.02.",
             "Not in the extension. Tool 2.02 gives the same computation in "
             "one run."),
}
RUST_NOTES = {
    "5.01": ("Точки профиля с пикетами. Растров у расширений RustGIS нет, "
             "поэтому отметок рельефа нет.",
             "Profile points with stations. RustGIS extensions have no "
             "rasters, so there are no terrain levels."),
    "5.02": ("Продольный профиль с сеткой и развёрнутым планом. Строки сетки "
             "берутся из шаблона «Трубопровод».",
             "Longitudinal profile with a grid and a straightened plan. The "
             "grid rows come from the «Pipeline» template."),
}

FOOTER_RU = ("Routeliner развивается на задачах реальных предприятий. Если "
             "вашему производству не хватает функции, напишите нам.")
FOOTER_EN = ("Routeliner grows on the tasks of real enterprises. If your "
             "operation is missing a feature, write to us.")

MODULE_TEXTS = {
    "ru": {
        "title": "Routeliner - события на маршрутах и пикетаж для QGIS · Информ++",
        "desc": "Routeliner ставит на оси дорог, путей, трубопроводов и линий "
                "электропередачи записи таблиц с пикетом вместо координат.",
        "brand.sub": "для QGIS",
        "nav.idea": "Идея", "nav.tools": "Инструменты",
        "nav.rustgis": "RustGIS", "nav.install": "Установка",
        "hero.eyebrow": "Модуль QGIS · Processing",
        "hero.h1": "Записи с пикетом встают на ось маршрута",
        "hero.lead": "Routeliner ставит на оси дорог, путей, трубопроводов и "
                     "линий электропередачи записи таблиц с пикетом вместо "
                     "координат, читает пять принятых в СНГ записей "
                     "пикета и учитывает ведомость с рублеными пикетами и "
                     "пикетажными уравнениями.",
        "cta.install": "Установить в QGIS", "cta.code": "Исходный код",
        "idea.eyebrow": "Как это устроено",
        "idea.h2": "Каждая запись либо на карте, либо в таблице ошибок",
        "idea.sub": "Модуль не пропускает записи без сообщения, и запись, "
                    "которую нельзя поставить, попадает в таблицу ошибок с кодом причины. По "
                    "маршруту строится продольный профиль с отметками растров, "
                    "уклонами и пикетами по ведомости.",
        "idea.c1.h": "Пикетаж по ведомости",
        "idea.c1.p": "Рубленые пикеты и пикетажные уравнения учитываются в "
                     "событиях, разбивке, привязке точек и профиле, а пикетаж "
                     "берётся из ведомости, из контрольных точек или из длины "
                     "по оси.",
        "idea.c2.h": "Пикетаж в M-значениях",
        "idea.c2.p": "Пикетаж маршрута можно взять из M-значений его геометрии "
                     "и записать в M результатов, и такие слои читают PostGIS, "
                     "ArcGIS и инструменты QGIS для M.",
        "tools.eyebrow": "Одиннадцать инструментов",
        "tools.h2": "Номер инструмента одинаков в панели, меню и руководстве",
        "tools.sub": "Инструменты лежат в панели «Инструменты анализа» в группе "
                     "Routeliner и повторены в меню «Модули - Routeliner» с теми "
                     "же номерами. Все они работают в моделях и в пакетном "
                     "режиме.",
        "rg.eyebrow": "Без QGIS",
        "rg.h2": "Routeliner для RustGIS",
        "rg.sub": "Восемь инструментов модуля работают в RustGIS, настольной ГИС "
                  "на Rust. Расчёт повторяет модуль и сверен со всеми 20 его "
                  "прогонами. QGIS для этого не нужен.",
        "rg.c1.h": "Что умеет",
        "rg.c1.p": "Проверка и калибровка маршрутов, точечные события и "
                   "участки, пикетная разбивка, привязка точек, таблица и "
                   "чертёж продольного профиля. Номера инструментов, поля rl_* "
                   "и коды ошибок те же, что у модуля.",
        "rg.c2.h": "Чем отличается от модуля",
        "rg.c2.p": "У расширения своя вкладка ленты RustGIS. Динамических слоёв, "
                   "демонстрационного примера, растров и M-значений в RustGIS "
                   "0.6.0 нет. Нужен RustGIS 0.6.0 или новее и Windows.",
        "rg.page": "Страница расширения",
        "rg.release": "Выпуск " + RUST_VERSION,
        "rg.code": "Исходный код расширения",
        "inst.eyebrow": "Установка и документация",
        "inst.h2": "Каталог модулей QGIS или ZIP-архив",
        "inst.sub": "Модуль ставится из каталога модулей QGIS или из ZIP-архива "
                    "через пункт «Модули - Управление модулями - Установить из "
                    "ZIP». Нужен QGIS 3.40 или новее, включая QGIS 4 на Qt6. "
                    "Внешних зависимостей нет, расчёт идёт на NumPy из состава "
                    "QGIS. Язык интерфейса выбирается по языку QGIS.",
        "inst.c1.h": "Руководство",
        "inst.c1.p": "Руководство в PDF открывается пунктом «Модули - "
                     "Routeliner - Руководство (PDF)» и описывает параметры "
                     "инструментов и поля всех создаваемых слоёв. Руководство "
                     "есть на русском и английском.",
        "inst.c2.h": "Журнал работы",
        "inst.c2.p": "Кнопка «О модуле» на панели Routeliner показывает версию, "
                     "историю изменений и журнал работы, который пишется в "
                     "файл routeliner.log в папке профиля QGIS. При ошибке этот "
                     "файл достаточно приложить к сообщению.",
        "inst.catalog": "Версия " + MODULE_VERSION + " в каталоге",
        "inst.releases": "Выпуски на GitHub",
        "ftr.line1": "лицензия GNU GPL v2 или новее · QGIS 3.40 и новее",
        "ftr.line2": "Разработано ООО «Информ++»",
        "ftr.line3": FOOTER_RU,
    },
    "en": {
        "title": "Routeliner - route events and chainage for QGIS · Inform++",
        "desc": "Routeliner places table records on the axes of roads, tracks, "
                "pipelines and power lines when a record carries a station "
                "instead of coordinates.",
        "brand.sub": "for QGIS",
        "nav.idea": "Idea", "nav.tools": "Tools",
        "nav.rustgis": "RustGIS", "nav.install": "Installation",
        "hero.eyebrow": "QGIS plugin · Processing",
        "hero.h1": "Records with a station land on the route axis",
        "hero.lead": "Routeliner places table records on the axes of roads, "
                     "tracks, pipelines and power lines when a record carries a "
                     "station instead of coordinates. The plugin reads the five "
                     "station notations accepted in the CIS countries and takes "
                     "into account a ledger with broken pickets and station "
                     "equations.",
        "cta.install": "Install in QGIS", "cta.code": "Source code",
        "idea.eyebrow": "How it works",
        "idea.h2": "Every record is either on the map or in the error table",
        "idea.sub": "The plugin never skips a record silently. A record that "
                    "cannot be placed goes to the error table with a reason "
                    "code. A longitudinal profile with raster levels, grades "
                    "and ledger stations is built along a route.",
        "idea.c1.h": "Chainage by the ledger",
        "idea.c1.p": "Broken pickets and station equations are taken into "
                     "account in events, stakeout, point locating and the "
                     "profile. The chainage comes from the ledger, from control "
                     "points or from the length along the axis.",
        "idea.c2.h": "Chainage in M values",
        "idea.c2.p": "The route chainage can be taken from the M values of its "
                     "geometry and written into the M of the results. PostGIS, "
                     "ArcGIS and the QGIS tools for M read such layers.",
        "tools.eyebrow": "Eleven tools",
        "tools.h2": "A tool has the same number in the toolbox, the menu and the manual",
        "tools.sub": "The tools sit in the Processing Toolbox in the Routeliner "
                     "group and are repeated in the Plugins - Routeliner menu "
                     "under the same numbers. All of them work in models and in "
                     "batch mode.",
        "rg.eyebrow": "Without QGIS",
        "rg.h2": "Routeliner for RustGIS",
        "rg.sub": "Eight tools of the plugin run in RustGIS, a desktop GIS "
                  "written in Rust. The computation repeats the plugin and is "
                  "checked against all 20 of its runs. QGIS is not needed.",
        "rg.c1.h": "What it does",
        "rg.c1.p": "Route check and calibration, point events and sections, "
                   "picket stakeout, point locating, the longitudinal profile "
                   "table and drawing. The tool numbers, the rl_* fields and "
                   "the error codes are the same as in the plugin.",
        "rg.c2.h": "How it differs from the plugin",
        "rg.c2.p": "The extension has its own RustGIS ribbon tab. RustGIS 0.6.0 "
                   "has no live layers, no demo example, no rasters and no M "
                   "values. RustGIS 0.6.0 or newer and Windows are required.",
        "rg.page": "Extension page",
        "rg.release": "Release " + RUST_VERSION,
        "rg.code": "Extension source code",
        "inst.eyebrow": "Installation and documentation",
        "inst.h2": "The QGIS plugin repository or a ZIP archive",
        "inst.sub": "The plugin is installed from the QGIS plugin repository or "
                    "from a ZIP archive through Plugins - Manage and Install "
                    "Plugins - Install from ZIP. It requires QGIS 3.40 or newer, "
                    "QGIS 4 on Qt6 included. There are no external dependencies, "
                    "the computation runs on the NumPy that ships with QGIS. The "
                    "interface language follows the language of QGIS.",
        "inst.c1.h": "Manual",
        "inst.c1.p": "The PDF manual opens from Plugins - Routeliner - Manual "
                     "(PDF) and describes the tool parameters and the fields of "
                     "every layer the plugin creates. The manual comes in "
                     "Russian and English.",
        "inst.c2.h": "Work log",
        "inst.c2.p": "The About button on the Routeliner toolbar shows the "
                     "version, the version history and the work log. The log is "
                     "written to routeliner.log in the QGIS profile folder, and "
                     "in case of an error it is enough to attach it to a "
                     "message.",
        "inst.catalog": "Version " + MODULE_VERSION + " in the repository",
        "inst.releases": "Releases on GitHub",
        "ftr.line1": "GNU GPL v2 or later · QGIS 3.40 and newer",
        "ftr.line2": "Developed by Inform++ LLC",
        "ftr.line3": FOOTER_EN,
    },
}

RUST_TEXTS = {
    "ru": {
        "title": "Routeliner для RustGIS - события на маршрутах и пикетаж · Информ++",
        "desc": "Routeliner для RustGIS ставит события на маршруты и ведёт "
                "пикетаж в RustGIS так же, как модуль QGIS Routeliner.",
        "brand.sub": "для RustGIS",
        "nav.tools": "Инструменты", "nav.check": "Сверка",
        "nav.limits": "Ограничения", "nav.install": "Установка",
        "hero.eyebrow": "Расширение RustGIS",
        "hero.h1": "Пикетаж и события на маршрутах в RustGIS",
        "hero.lead": "Routeliner для RustGIS ставит события на маршруты и ведёт "
                     "пикетаж в RustGIS так же, как модуль QGIS Routeliner, и "
                     "QGIS для этого не нужен. Номера инструментов, имена полей "
                     "rl_* и коды ошибок у расширения и у модуля одни и те же.",
        "cta.release": "Скачать выпуск " + RUST_VERSION,
        "cta.code": "Исходный код",
        "tools.eyebrow": "Восемь инструментов из одиннадцати",
        "tools.h2": "Своя вкладка ленты с теми же номерами",
        "tools.sub": "У расширения своя вкладка ленты Routeliner с группами "
                     "«Маршруты», «События», «Пикетаж», «Профиль» и «Справка». "
                     "Итог событий и привязки повторяет поля исходной записи и "
                     "добавляет поля rl_*. Запись, которую нельзя поставить, "
                     "попадает в таблицу ошибок вместе со своими полями.",
        "check.eyebrow": "Сверка с модулем",
        "check.h2": "Каждый инструмент сверен с прогонами модуля",
        "check.sub": "Сверка идёт по прогонам модуля Routeliner "
                     + MODULE_VERSION + " на демонстрационном примере 1.01 с "
                     "зерном 1 в зоне UTM 40N. Пояснения rl_message сравниваются "
                     "по-русски слово в слово, а подписи вида «ПК 14+71.24» "
                     "совпадают с модулем знак в знак.",
        "fact1": "прогонов модуля по восьми инструментам, поля совпадают без "
                 "отклонений.",
        "fact2": "м - наибольшее расхождение вершин, у M-значений не больше "
                 "5·10⁻¹¹ м.",
        "fact3": "м - предел задания для пути через RustGIS в градусах WGS 84, "
                 "длины, меры и смещения сходятся в нём.",
        "lim.eyebrow": "Ограничения RustGIS 0.6.0",
        "lim.h2": "Чем расширение отличается от модуля",
        "lim.sub": "RustGIS отдаёт слои в градусах WGS 84 без Z и без M. "
                   "Расширение считает в метрах зоны UTM по середине входа, а "
                   "итог возвращает в градусах, и из этого перевода и из "
                   "устройства RustGIS следуют такие различия.",
        "lim.1": "Пикетаж из M-значений геометрии не работает, у слоя RustGIS "
                 "нет M. Маршрут без ведомости получает пикетаж по длине.",
        "lim.2": "M-значения итога 1.03, 2.01, 2.02 и 4.01 расширение считает, "
                 "а в слой RustGIS они не попадают. Пикетаж и мера остаются в "
                 "полях итога.",
        "lim.3": "Растров у расширений RustGIS нет, поэтому 5.01 не снимает "
                 "отметки рельефа. Отметки для чертежа 5.02 берутся из "
                 "числового поля таблицы.",
        "lim.4": "В окне 5.02 нет матрицы строк сетки и точки привязки чертежа. "
                 "Строки сетки берутся из шаблона «Трубопровод», а чертёж "
                 "ставится на 100 мм ниже таблицы.",
        "lim.5": "Дуг в GeoJSON нет, маршрут приходит отрезками, и длина "
                 "считается по ним.",
        "inst.eyebrow": "Установка",
        "inst.h2": "Пакет ставится из файла",
        "inst.sub": "Пакет routeliner-" + RUST_VERSION + ".rgplugin со страницы "
                    "выпуска ставится пунктом Install From File в окне Plugin "
                    "Manager. Нужен RustGIS 0.6.0 или новее и Windows. Для "
                    "проверки годятся слои демонстрационного примера модуля "
                    "QGIS.",
        "inst.module": "Модуль Routeliner для QGIS",
        "inst.rustgis": "RustGIS",
        "ftr.line1": "лицензия GNU GPL v3 или новее · RustGIS 0.6.0 и новее",
        "ftr.line2": "Разработано ООО «Информ++»",
        "ftr.line3": FOOTER_RU,
    },
    "en": {
        "title": "Routeliner for RustGIS - route events and chainage · Inform++",
        "desc": "Routeliner for RustGIS places events on routes and keeps "
                "chainage in RustGIS the way the Routeliner plugin for QGIS "
                "does.",
        "brand.sub": "for RustGIS",
        "nav.tools": "Tools", "nav.check": "Check",
        "nav.limits": "Limits", "nav.install": "Installation",
        "hero.eyebrow": "RustGIS extension",
        "hero.h1": "Chainage and route events in RustGIS",
        "hero.lead": "Routeliner for RustGIS places events on routes and keeps "
                     "chainage in RustGIS the way the Routeliner plugin for "
                     "QGIS does. The tool numbers, the rl_* field names and the "
                     "error codes are the same in the extension and in the "
                     "plugin. QGIS is not needed.",
        "cta.release": "Download release " + RUST_VERSION,
        "cta.code": "Source code",
        "tools.eyebrow": "Eight tools out of eleven",
        "tools.h2": "A ribbon tab of its own with the same numbers",
        "tools.sub": "The extension has a ribbon tab of its own, Routeliner, "
                     "with the groups «Routes», «Events», «Chainage», «Profile» "
                     "and «Help». The result of events and locating repeats the "
                     "fields of the source record and adds the rl_* fields. A "
                     "record that cannot be placed goes to the error table "
                     "together with its fields.",
        "check.eyebrow": "Check against the plugin",
        "check.h2": "Every tool is checked against the plugin runs",
        "check.sub": "The check uses the runs of Routeliner " + MODULE_VERSION
                     + " on demo 1.01 with seed 1 in UTM zone 40N. The "
                     "rl_message explanations are compared in Russian word for "
                     "word, and a label like «ПК 14+71.24» matches the plugin "
                     "character for character.",
        "fact1": "plugin runs over eight tools, the fields agree without "
                 "deviation.",
        "fact2": "m is the largest vertex difference, M values differ by no "
                 "more than 5·10⁻¹¹ m.",
        "fact3": "m is the task limit for the path through RustGIS in WGS 84 "
                 "degrees, lengths, measures and offsets agree within it.",
        "lim.eyebrow": "Limits of RustGIS 0.6.0",
        "lim.h2": "How the extension differs from the plugin",
        "lim.sub": "RustGIS sends layers in WGS 84 degrees without Z and "
                   "without M. The extension computes in metres of the UTM zone "
                   "at the middle of the input and returns the result in "
                   "degrees. This and the design of RustGIS give the following "
                   "differences.",
        "lim.1": "Chainage from geometry M values does not work, a RustGIS "
                 "layer has no M. A route without a ledger gets chainage by "
                 "length.",
        "lim.2": "The extension computes the M values of the 1.03, 2.01, 2.02 "
                 "and 4.01 results, but they do not reach the RustGIS layer. "
                 "The chainage and the measure stay in the result fields.",
        "lim.3": "RustGIS extensions have no rasters, so 5.01 does not sample "
                 "terrain levels. The levels for the 5.02 drawing come from a "
                 "numeric table field.",
        "lim.4": "The 5.02 window has neither the grid row matrix nor the "
                 "drawing anchor point. The grid rows come from the «Pipeline» "
                 "template, and the drawing is placed 100 mm below the table.",
        "lim.5": "GeoJSON has no arcs, a route comes as segments, and the "
                 "length is measured along them.",
        "inst.eyebrow": "Installation",
        "inst.h2": "The package is installed from a file",
        "inst.sub": "The package routeliner-" + RUST_VERSION + ".rgplugin from "
                    "the release page is installed with Install From File in "
                    "the Plugin Manager window. RustGIS 0.6.0 or newer and "
                    "Windows are required. The demo layers of the QGIS plugin "
                    "are good for a check.",
        "inst.module": "Routeliner plugin for QGIS",
        "inst.rustgis": "RustGIS",
        "ftr.line1": "GNU GPL v3 or later · RustGIS 0.6.0 and newer",
        "ftr.line2": "Developed by Inform++ LLC",
        "ftr.line3": FOOTER_EN,
    },
}


def tool_rows(texts, rust):
    """Строки инструментов по группам, ключи текстов дописываются в texts."""
    out, group = [], None
    for code, ru_h, ru_p, en_h, en_p in TOOLS:
        g = code.split(".")[0]
        if g != group:
            group = g
            texts["ru"]["g" + g], texts["en"]["g" + g] = GROUPS[g]
            out.append('<div class="group" data-i18n="g%s"></div>' % g)
        key = "t" + code.replace(".", "")
        off = rust and code in MISSING
        if rust and code in MISSING:
            ru_p, en_p = MISSING[code]
        elif rust and code in RUST_NOTES:
            ru_p, en_p = RUST_NOTES[code]
        texts["ru"][key + ".h"], texts["ru"][key + ".p"] = ru_h, ru_p
        texts["en"][key + ".h"], texts["en"][key + ".p"] = en_h, en_p
        out.append('<div class="tool%s"><div class="num">%s</div><div class="txt">'
                   '<b data-i18n="%s.h"></b><span data-i18n="%s.p"></span>'
                   '</div></div>' % (" off" if off else "", code, key, key))
    return "\n      ".join(out)


def page(name, brand, nav, body, texts, footer):
    links = "\n        ".join('<a href="#%s" data-i18n="nav.%s"></a>' % (a, k)
                               for a, k in nav)
    data = json.dumps(texts, ensure_ascii=False)
    store = name + "-lang"
    return """<!-- ============================================================= -->
<!-- %s - страница для www.informpp.ru                     -->
<!-- Собирается сценарием site/build_site.py хранилища               -->
<!-- routeliner-rustgis, править лучше его.                          -->
<!-- ============================================================= -->
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="description" data-i18n-attr="desc" content="">
<title data-i18n="title"></title>
<style>%s</style>

<header class="hdr">
  <div class="wrap">
    <a class="brand" href="#">%s <span data-i18n="brand.sub"></span></a>
    <div class="right">
      <nav class="nav">
        %s
      </nav>
      <div class="lang">
        <button type="button" data-lang="ru">RU</button>
        <button type="button" data-lang="en">EN</button>
      </div>
    </div>
  </div>
</header>
%s
<footer class="ftr">
  <div class="wrap">
    %s &middot; <span data-i18n="ftr.line1"></span><br>
    <span data-i18n="ftr.line2"></span>,
    <a href="%s" target="_blank" rel="noopener">www.informpp.ru</a><br>
    <span data-i18n="ftr.line3"></span>
    <a href="%s" target="_blank" rel="noopener">www.informpp.ru/предприятиям</a>
  </div>
</footer>

<script>%s</script>
""" % (brand, CSS, brand, links, body, html.escape(footer), SITE, ORDER,
       SCRIPT % (data, store, store))


def cards(prefix):
    return """<div class="pair">
      <div class="card"><h3 data-i18n="%s.c1.h"></h3>
        <p data-i18n="%s.c1.p"></p></div>
      <div class="card"><h3 data-i18n="%s.c2.h"></h3>
        <p data-i18n="%s.c2.p"></p></div>
    </div>""" % (prefix, prefix, prefix, prefix)


def head(prefix):
    return """<div class="eyebrow" data-i18n="%s.eyebrow"></div>
    <h2 data-i18n="%s.h2"></h2>
    <p class="sub" data-i18n="%s.sub"></p>""" % (prefix, prefix, prefix)


def button(cls, key, href):
    # Песочница блока «Встроить» не пускает переход верхнего окна, поэтому
    # внешние ссылки открываются новой вкладкой.
    return ('<a class="btn btn-%s" data-i18n="%s" href="%s" target="_blank" '
            'rel="noopener"></a>' % (cls, key, href))


def module_page():
    texts = json.loads(json.dumps(MODULE_TEXTS))
    body = """
<div class="wrap hero">
  <div class="eyebrow" data-i18n="hero.eyebrow"></div>
  <h1 data-i18n="hero.h1"></h1>
  <p class="lead" data-i18n="hero.lead"></p>
  <div class="cta">
    %s
    %s
  </div>
</div>

<section id="idea"><div class="wrap">
    %s
    %s
</div></section>

<section id="tools"><div class="wrap">
    %s
    <div class="tools">
      %s
    </div>
</div></section>

<section id="rustgis"><div class="wrap">
    %s
    %s
    <div class="cta">
      %s
      %s
    </div>
</div></section>

<section id="install"><div class="wrap">
    %s
    %s
    <div class="cta">
      %s
      %s
      %s
    </div>
</div></section>
""" % (button("main", "cta.install", CATALOG), button("ghost", "cta.code", MODULE_CODE),
       head("idea"), cards("idea"),
       head("tools"), tool_rows(texts, False),
       head("rg"), cards("rg"),
       button("main", "rg.page", PAGE_RUST), button("ghost", "rg.release", RUST_RELEASE),
       head("inst"), cards("inst"),
       button("main", "cta.install", CATALOG),
       button("ghost", "inst.catalog", CATALOG_VERSION),
       button("ghost", "inst.releases", MODULE_RELEASES))
    nav = (("idea", "idea"), ("tools", "tools"), ("rustgis", "rustgis"),
           ("install", "install"))
    return page("routeliner", "Routeliner", nav, body, texts,
                "Routeliner " + MODULE_VERSION)


def rust_page():
    texts = json.loads(json.dumps(RUST_TEXTS))
    limits = "\n      ".join('<li data-i18n="lim.%d"></li>' % i for i in range(1, 6))
    body = """
<div class="wrap hero">
  <div class="eyebrow" data-i18n="hero.eyebrow"></div>
  <h1 data-i18n="hero.h1"></h1>
  <p class="lead" data-i18n="hero.lead"></p>
  <div class="cta">
    %s
    %s
  </div>
</div>

<section id="tools"><div class="wrap">
    %s
    <div class="tools">
      %s
    </div>
</div></section>

<section id="check"><div class="wrap">
    %s
    <div class="facts">
      <div class="fact"><b>20</b><span data-i18n="fact1"></span></div>
      <div class="fact"><b>6&middot;10<sup>&minus;11</sup></b><span data-i18n="fact2"></span></div>
      <div class="fact"><b>0,001</b><span data-i18n="fact3"></span></div>
    </div>
</div></section>

<section id="limits"><div class="wrap">
    %s
    <ul class="limits">
      %s
    </ul>
</div></section>

<section id="install"><div class="wrap">
    %s
    <div class="cta">
      %s
      %s
      %s
    </div>
</div></section>
""" % (button("main", "cta.release", RUST_RELEASE), button("ghost", "cta.code", RUST_CODE),
       head("tools"), tool_rows(texts, True),
       head("check"),
       head("lim"), limits,
       head("inst"),
       button("main", "cta.release", RUST_RELEASE),
       button("ghost", "inst.module", PAGE_MODULE),
       button("ghost", "inst.rustgis", RUSTGIS))
    nav = (("tools", "tools"), ("check", "check"), ("limits", "limits"),
           ("install", "install"))
    return page("routeliner-rustgis", "Routeliner", nav, body, texts,
                "Routeliner для RustGIS " + RUST_VERSION)


def module_page_texts():
    texts = json.loads(json.dumps(MODULE_TEXTS))
    tool_rows(texts, False)
    return texts


def rust_page_texts():
    texts = json.loads(json.dumps(RUST_TEXTS))
    tool_rows(texts, True)
    return texts


def main():
    for name, build in (("routeliner_landing.html", module_page),
                        ("routeliner_rustgis_landing.html", rust_page)):
        with open(os.path.join(HERE, name), "w", encoding="utf-8", newline="\n") as f:
            f.write(build())
    # Тексты для проверки - по файлу на язык, абзац на ключ. Ритм measure.py
    # считается по абзацам, поэтому заголовки, кнопки и имена инструментов
    # идут в отдельный файл и проверяются только check_texts.py.
    for lang in ("ru", "en"):
        prose, short = [], []
        for source in (module_page_texts(), rust_page_texts()):
            for k, v in source[lang].items():
                (prose if is_prose(k) else short).append(v)
        for suffix, lines in (("", prose), (".short", short)):
            name = "site_texts%s.%s.md" % (suffix, lang)
            with open(os.path.join(HERE, name), "w", encoding="utf-8",
                      newline="\n") as f:
                f.write("\n\n".join(lines) + "\n")


def is_prose(key):
    return (key in ("desc", "hero.lead")
            or key.endswith((".sub", ".p")) and not key.startswith("t")
            or key.startswith("lim.") and key[4:].isdigit())


if __name__ == "__main__":
    main()
