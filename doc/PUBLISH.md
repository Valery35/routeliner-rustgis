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
