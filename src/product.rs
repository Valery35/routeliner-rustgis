// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Routeliner как продукт крейта `rustgis-kit`: что у расширения своё.
//!
//! Протокол RustGIS, окно инструмента по `tools.json`, перевод градусов
//! в метры зоны UTM и обратно, итоги слоями и таблицами, упаковка - в
//! крейте. Здесь расчёт инструментов ([`crate::tools`]) и команда «О
//! расширении».

use rustgis_kit::runner::no_command;
use rustgis_kit::{Call, Catalog, Failure, Lang, Options, Product, Store, Tool};
use serde_json::{Value as J, json};

/// Описания инструментов - выгрузка модуля QGIS (`fixtures/tools.json`
/// модуля, собирает `tools/export_tools.py`).
pub const TOOLS_JSON: &str = include_str!("../fixtures/tools.json");

/// Инструменты Routeliner в RustGIS, в порядке панели.
pub const OPTIONS: Options = Options {
    toolbox: "Routeliner",
    offered: crate::tools::DONE,
    xy_tables: &[],
};

/// Страница модуля.
pub const HOME: &str = "https://github.com/Valery35/routeliner";

pub struct Routeliner {
    pub catalog: Catalog,
}

impl Routeliner {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            catalog: Catalog::parse(TOOLS_JSON)?,
        })
    }
}

/// «О расширении» на языке показа.
pub fn about(lang: Lang) -> J {
    let (text, links) = match lang {
        Lang::Ru => (
            vec![
                "События на маршрутах и пикетаж для RustGIS. Инструменты считают так же, как модуль QGIS Routeliner, номера инструментов и поля rl_* те же.",
                "Разработано при поддержке ООО «Информ++» (www.informpp.ru).",
            ],
            json!([
                {"label": "Страница модуля", "url": HOME},
                {"label": "ООО «Информ++»", "url": "https://www.informpp.ru"},
            ]),
        ),
        Lang::En => (
            vec![
                "Route events and chainage for RustGIS. The tools compute as the Routeliner plugin for QGIS does, with the same tool numbers and rl_* fields.",
                "Developed with the support of Inform++ LLC (www.informpp.ru).",
            ],
            json!([
                {"label": "Plugin page", "url": HOME},
                {"label": "Inform++ LLC", "url": "https://www.informpp.ru"},
            ]),
        ),
    };
    json!({"ok": {
        "title": "Routeliner",
        "version": env!("CARGO_PKG_VERSION"),
        "text": text,
        "links": links,
    }})
}

impl Product for Routeliner {
    fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    fn options(&self) -> &Options {
        &OPTIONS
    }

    fn store(&self) -> Store {
        Store::new(rustgis_kit::store::temp_home("routeliner").join("bodies"))
    }

    fn compute(&self, tool: &Tool, values: &J, lang: Lang) -> Result<(J, Vec<String>), Failure> {
        crate::tools::run(tool, values, lang)
    }

    fn command(&self, _host: &mut dyn Call, req: &J, lang: Lang) -> J {
        match req["id"].as_str().unwrap_or("") {
            "about" => about(lang),
            id => no_command(id, lang),
        }
    }
}

/// Основа `plugin.json`: всё, кроме раздела `tools`. Вкладка ленты `tab`
/// прячет инструменты в четыре меню по группам модуля, значки - имена
/// из набора RustGIS (`crates/ui-egui/src/icons.rs`), иначе RustGIS
/// рисует пазл.
pub const MANIFEST: &str = r#"{
  "id": "routeliner",
  "name": "Routeliner",
  "version": "0.1.0",
  "description": "Route events and chainage for RustGIS. Routes are assembled from shuffled parts, chainage comes from a ledger with station equations or from control points, and every record that cannot be placed goes to the error table with its reason. The tools compute as the Routeliner plugin for QGIS does.",
  "author": "Inform++ LLC",
  "license": "GPL-3.0-or-later",
  "runtime": "process",
  "session": true,
  "run": { "windows": ["bin/routeliner-rustgis.exe", "rustgis-plugin"] },
  "minRustgisVersion": "0.5.0",
  "commands": [
    { "id": "about", "label": "About\nRouteliner", "help": "About the extension: version, a short description and links.", "icon": "about", "params": "{} (answers the title, the version, text paragraphs and links in the RustGIS language)" }
  ],
  "tab": {
    "label": "Routeliner",
    "groups": [
      {
        "label": "Routeliner",
        "items": [
          { "menu": "Routes", "icon": "directions", "help": "Route check and calibration.", "items": ["tool:check-routes", "tool:calibrate-routes"] },
          { "menu": "Events", "icon": "placement", "help": "Point events and sections on routes.", "items": ["tool:point-events", "tool:line-events"] },
          { "menu": "Chainage", "icon": "measure", "help": "Picket stakeout and locating points on routes.", "items": ["tool:pickets", "tool:locate-points"] },
          { "menu": "Profile", "icon": "chart", "help": "Profile table and profile drawing.", "items": ["tool:profile-table", "tool:profile-drawing"] },
          "about"
        ]
      }
    ]
  },
  "i18n": {
    "ru": {
      "Routes": "Маршруты",
      "Route check and calibration.": "Проверка и калибровка маршрутов.",
      "Events": "События",
      "Point events and sections on routes.": "Точечные события и участки на маршрутах.",
      "Chainage": "Пикетаж",
      "Picket stakeout and locating points on routes.": "Пикетная разбивка и привязка точек к маршрутам.",
      "Profile": "Профиль",
      "Profile table and profile drawing.": "Таблица и чертёж профиля.",
      "Route events and chainage for RustGIS. Routes are assembled from shuffled parts, chainage comes from a ledger with station equations or from control points, and every record that cannot be placed goes to the error table with its reason. The tools compute as the Routeliner plugin for QGIS does.": "События на маршрутах и пикетаж для RustGIS. Маршруты собираются из перепутанных частей, пикетаж берётся из ведомости с пикетажными уравнениями или по контрольным точкам, а каждая запись, которую нельзя поставить, попадает в таблицу ошибок с причиной. Инструменты считают так же, как модуль QGIS Routeliner.",
      "About\nRouteliner": "О расширении\nRouteliner",
      "About the extension: version, a short description and links.": "О расширении: версия, краткое описание и ссылки."
    }
  }
}
"#;

/// Поля, скрытые в окне RustGIS: (инструмент Processing, ключ), `*` - у
/// всех. У слоёв RustGIS нет M, пикетаж из M там не работает, расчёт
/// берёт умолчания из `tools.json`.
pub const HIDDEN: &[(&str, &str)] = &[
    ("*", "USE_M"),
    ("*", "M_FACTOR"),
    // канал растров без растров не нужен
    ("profile_table", "BAND"),
];

/// `plugin.json` с разделом `tools` и переводами его строк.
pub fn manifest(catalog: &Catalog) -> Result<String, String> {
    let (tools, ru) = rustgis_kit::manifest::tools_section_hiding(catalog, &OPTIONS, HIDDEN);
    rustgis_kit::manifest::splice(MANIFEST, &tools, &ru)
}
