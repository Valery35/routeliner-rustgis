// Routeliner для RustGIS - события на маршрутах и пикетаж.
// Copyright (C) 2026 ООО «Информ++». Лицензия GPL 3.0 или новее.
// SPDX-License-Identifier: GPL-3.0-or-later

//! Routeliner как продукт крейта `rustgis-kit`: что у расширения своё.
//!
//! Протокол RustGIS, окно инструмента по `tools.json`, перевод градусов
//! в метры зоны UTM и обратно, итоги слоями и таблицами, упаковка - в
//! крейте. Здесь расчёт инструментов ([`crate::tools`]) и команда «О
//! расширении».

use rustgis_kit::manifest::{Group, Item, Label, Layout, Menu, Tab};
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
    /// Привязки динамических слоёв сессии (3.01, 3.02).
    pub live: crate::live::Live,
}

impl Routeliner {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            catalog: Catalog::parse(TOOLS_JSON)?,
            live: crate::live::Live::default(),
        })
    }

    /// Динамический инструмент по id команды RustGIS.
    fn live_of(&self, id: &str) -> Option<&'static str> {
        crate::live::LIVE
            .iter()
            .map(|(l, _)| *l)
            .find(|l| self.catalog.tool(l).is_some_and(|t| t.rg_id() == id))
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

    /// 1.01 без входных слоёв: пример строится в метрах EPSG:32640, итог
    /// уходит в RustGIS в этих метрах с членом `crs`.
    fn native_frame(&self, tool: &Tool) -> Option<String> {
        (tool.name == "demo").then(|| crate::core::demo::CRS.to_string())
    }

    fn command(&self, host: &mut dyn Call, req: &J, lang: Lang) -> J {
        match req["id"].as_str().unwrap_or("") {
            "about" => about(lang),
            id => match self.live_of(id) {
                Some(live) => {
                    let params = rustgis_kit::host::params_of(req);
                    self.live.start(self, host, live, &params, lang)
                }
                None => no_command(id, lang),
            },
        }
    }

    fn event(&self, host: &mut dyn Call, name: &str) {
        if name == "layers" {
            self.live.changed(self, host);
        }
    }
}

/// Основа `plugin.json`: всё, кроме разделов `tools` и `tab`, их строит
/// [`manifest`].
pub const MANIFEST: &str = r#"{
  "id": "routeliner",
  "name": "Routeliner",
  "version": "0.2.0",
  "description": "Route events and chainage for RustGIS. Routes are assembled from shuffled parts, chainage comes from a ledger with station equations or from control points, and every record that cannot be placed goes to the error table with its reason. The tools compute as the Routeliner plugin for QGIS does.",
  "author": "Inform++ LLC",
  "homepage": "https://github.com/Valery35/routeliner-rustgis",
  "license": "GPL-3.0-or-later",
  "runtime": "process",
  "session": true,
  "run": { "windows": ["bin/routeliner-rustgis.exe", "rustgis-plugin"] },
  "minRustgisVersion": "0.7.0",
  "commands": [
    { "id": "about", "label": "About\nRouteliner", "help": "About the extension: version, a short description and links.", "icon": "about", "params": "{} (answers the title, the version, text paragraphs and links in the RustGIS language)" }
  ],
  "i18n": {
    "ru": {
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

/// Меню вкладки: у инструмента в RustGIS значка нет, поэтому инструменты
/// лежат в меню со значком. Значки - имена из набора RustGIS (`ALL`
/// в `crates/ui-egui/src/icons.rs`), чужое имя рисуется пустым квадратом.
static ROUTES: Menu = Menu {
    label: Label {
        en: "Check and calibrate",
        ru: "Проверка и калибровка",
    },
    icon: Some("directions"),
    help: Some(Label {
        en: "Demo example, route check and calibration.",
        ru: "Демонстрационный пример, проверка и калибровка маршрутов.",
    }),
    layout: None,
    items: &[
        Item::Tool("demo"),
        Item::Tool("check_routes"),
        Item::Tool("calibrate_routes"),
    ],
};
static EVENTS: Menu = Menu {
    label: Label {
        en: "Place events",
        ru: "Постановка событий",
    },
    icon: Some("placement"),
    help: Some(Label {
        en: "Point events and sections on routes, also as live layers.",
        ru: "Точечные события и участки на маршрутах, в том числе динамическими слоями.",
    }),
    layout: None,
    items: &[
        Item::Tool("point_events"),
        Item::Tool("line_events"),
        Item::Command("live-point-events"),
        Item::Command("live-line-events"),
    ],
};
static CHAINAGE: Menu = Menu {
    label: Label {
        en: "Pickets and locating",
        ru: "Пикеты и привязка",
    },
    icon: Some("measure"),
    help: Some(Label {
        en: "Picket stakeout and locating points on routes.",
        ru: "Пикетная разбивка и привязка точек к маршрутам.",
    }),
    layout: None,
    items: &[Item::Tool("pickets"), Item::Tool("locate_points")],
};
static PROFILE: Menu = Menu {
    label: Label {
        en: "Table and drawing",
        ru: "Таблица и чертёж",
    },
    icon: Some("chart"),
    help: Some(Label {
        en: "Profile table and profile drawing.",
        ru: "Таблица и чертёж профиля.",
    }),
    layout: None,
    items: &[Item::Tool("profile_table"), Item::Tool("profile_drawing")],
};

/// Своя вкладка ленты: группы модуля и справка.
pub static TAB: Tab = Tab {
    label: Label {
        en: "Routeliner",
        ru: "Routeliner",
    },
    groups: &[
        Group {
            label: Label {
                en: "Routes",
                ru: "Маршруты",
            },
            layout: Layout::Large,
            items: &[Item::Menu(&ROUTES)],
        },
        Group {
            label: Label {
                en: "Events",
                ru: "События",
            },
            layout: Layout::Large,
            items: &[Item::Menu(&EVENTS)],
        },
        Group {
            label: Label {
                en: "Chainage",
                ru: "Пикетаж",
            },
            layout: Layout::Large,
            items: &[Item::Menu(&CHAINAGE)],
        },
        Group {
            label: Label {
                en: "Profile",
                ru: "Профиль",
            },
            layout: Layout::Large,
            items: &[Item::Menu(&PROFILE)],
        },
        Group {
            label: Label {
                en: "Help",
                ru: "Справка",
            },
            layout: Layout::Large,
            items: &[Item::Command("about")],
        },
    ],
};

/// Значок команд 3.01 и 3.02 из набора RustGIS (`ALL` в `icons.rs`).
const LIVE_ICON: &str = "run";

/// Строка окна 1.01 в RustGIS: пример ложится в проект слоями.
const DEMO_SUMMARY: (&str, &str) = (
    "Puts the demo example into the project as layers and tables: four routes, the ledger, point and line events and defects with reference answers, and runs it through the extension tools.",
    "Кладёт демонстрационный пример в проект слоями и таблицами: четыре маршрута, ведомость, точечные и линейные события и дефекты с эталонными ответами, и прогоняет его через инструменты расширения.",
);

/// `plugin.json` с разделами `tools` и `tab` и переводами их строк.
pub fn manifest(catalog: &Catalog) -> Result<String, String> {
    let (mut tools, mut ru) =
        rustgis_kit::manifest::tools_section_hiding(catalog, &OPTIONS, HIDDEN);
    // 1.01 модуля пишет GeoPackage, расширение кладёт пример слоями
    for t in &mut tools {
        if t["id"] == "demo" {
            t["summary"] = json!(DEMO_SUMMARY.0);
            ru.insert(DEMO_SUMMARY.0.into(), json!(DEMO_SUMMARY.1));
        }
    }
    // 3.01 и 3.02 - команды сессии с окном полей инструмента модуля
    let live_opts = Options {
        toolbox: OPTIONS.toolbox,
        offered: &["live_point_events", "live_line_events"],
        xy_tables: &[],
    };
    let (live, live_ru) = rustgis_kit::manifest::tools_section_hiding(catalog, &live_opts, HIDDEN);
    ru.extend(live_ru);
    let mut ids = vec!["about".to_string()];
    let mut lines = Vec::new();
    for t in &live {
        let id = t["id"].as_str().unwrap_or("").to_string();
        let cmd = json!({
            "id": id, "label": t["name"], "help": t["summary"], "icon": LIVE_ICON,
            "form": t["params"],
        });
        lines.push(serde_json::to_string(&cmd).map_err(|e| e.to_string())?);
        ids.push(id);
    }
    let base = MANIFEST.replacen(
        "}\n  ],\n  \"i18n\"",
        &format!("}},\n    {}\n  ],\n  \"i18n\"", lines.join(",\n    ")),
        1,
    );
    let names: Vec<&str> = ids.iter().map(String::as_str).collect();
    let (tab, tab_ru) = rustgis_kit::manifest::tab_section(&TAB, catalog, &OPTIONS, &names)?;
    ru.extend(tab_ru);
    rustgis_kit::manifest::splice_all(&base, &tools, Some(&tab), &ru)
}
