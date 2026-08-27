# aoi

Музыкальный плеер на Tauri 2 и Rust.

## Сборка

```powershell
cd src-tauri
cargo tauri build --no-bundle
```

Готовый файл: `dist/aoi.exe` (или `src-tauri/target/release/aoi.exe`)

## Структура

```
aoi/
  ui/          фронтенд
  src-tauri/   Rust / Tauri
```

Данные: `%APPDATA%\aoi.player\`
