# Публикация выпусков Routeliner для RustGIS

Журнал публикаций расширения Routeliner для RustGIS. Заведён 16 ноября
2026 года. Общие правила и группы площадок - журнал
`C:\Dev\planetx-lite\doc\PUBLISH.md` и навык `publish-release`, о модуле
QGIS - `C:\Dev\routeliner\doc\PUBLISH.md`.

## Правила

- Тег выпуска `rustgis-v<номер>`, название «Routeliner для RustGIS <номер>».
- К выпуску прикрепляется пакет `routeliner-<номер>.rgplugin`, его собирает
  `scripts/package.ps1`.
- Dropbox `ИИ\Routeliner` - пакет последнего выпуска, прежние в `arhiv`.
  Архивы модуля QGIS лежат там же и не трогаются.
- Telegram - сообщение автору с кнопкой «Подготовить пост» по выпуску
  GitHub (`.github/workflows/telegram-release.yml`, секреты заводит автор).
- Тексты - `site/release_<номер>.md` и `site/posts_<номер>_social.md`.

## Журнал

### Routeliner для RustGIS 0.1.0 - 16 ноября 2026 года

Тег `rustgis-v0.1.0`, по слову автора «Публикуй и в мой растгис
добавляй». Восемь инструментов модуля 0.5.2, своя вкладка ленты. Сайт -
по слову автора позже, «Иногда» не публиковалось. Тексты -
`site/release_0.1.0.md`, `site/posts_0.1.0_social.md`.

| Площадка | Запись |
|---|---|
| GitHub | https://github.com/Valery35/routeliner-rustgis/releases/tag/rustgis-v0.1.0 - пакет `routeliner-0.1.0.rgplugin`, SHA-256 6452a422...e749 |
| Dropbox | `ИИ\Routeliner\routeliner-0.1.0.rgplugin`, прежних пакетов расширения не было |
| RustGIS автора | установлено 0.1.0 (`rustgis-cli plugins install`), тот же SHA-256 |
| Telegram | сообщение автору по выпуску - workflow «Telegram о выпуске» отработал, https://github.com/Valery35/routeliner-rustgis/actions/runs/43763619749 |
| Mastodon | https://en.osm.town/@Vaery35/117622572608039329 |
| X | https://x.com/Valery35/status/2122079341395796254 |
| LinkedIn | https://www.linkedin.com/feed/update/urn:li:activity:7527845455574102016/ |
| Facebook | https://www.facebook.com/valery35/posts/pfbid0ux6VQ3SfELB67xYijNaoNvsnFYNLKh1GX64imgvc5WBaudCxC478bGEUB3oC18ngl |
| ВК | https://vk.ru/wall-241229428_80 |
| Сайт informpp.ru | не публиковалось - по слову автора позже |
