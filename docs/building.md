# Сборка и запуск Transaudio

## Что нужно

- Node.js 20.19+ (лучше 22+).
- Rust 1.90+ и Cargo.
- Платформенные зависимости Tauri 2:
  - **macOS:** Xcode Command Line Tools.
  - **Windows:** Visual Studio Build Tools с C++ workload и WebView2 Runtime (в Windows 10/11 обычно уже есть).
  - **Linux:** Ubuntu 24.04 (glibc 2.39) или новее — предсобранный ONNX Runtime требует современный glibc. Нужны WebKitGTK 4.1, GTK/GLib, OpenSSL, librsvg и toolchain:
    ```bash
    sudo apt update
    sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
      libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```
- Интернет при первом запуске — для загрузки модели Parakeet V3 и, при необходимости, FFmpeg.

## Запуск для разработки

```bash
git clone <repo>
cd transaudio
npm ci
npm run tauri dev
```

`tauri dev` сам запускает Vite на порту 1420. Окно приложения использует локальные шрифты из `public/fonts`.

## Модель

При первом запуске приложение показывает экран разрешения. После разрешения:

- скачивается `parakeet-v3-int8` (~456 МБ архив, ~640 МБ на диске) с `blob.handy.computer`;
- архив проверяется по SHA-256;
- модель распаковывается в `$APP_DATA/models/parakeet-v3-int8`;
- модель загружается в фоне при каждом следующем запуске.

`$APP_DATA`:

- macOS: `~/Library/Application Support/com.transaudio.desktop`
- Windows: `%APPDATA%\com.transaudio.desktop`
- Linux: `~/.local/share/com.transaudio.desktop`

После выдачи разрешения оно хранится в `$APP_DATA/settings.json` (`model_consent: true`), повторно приложение его не запрашивает.

## FFmpeg

Для извлечения аудио из медиа используется FFmpeg. Приложение:

1. ищет `$APP_DATA/bin/ffmpeg`;
2. ищет `ffmpeg` в `PATH`;
3. если не находит — скачивает сборку для текущей ОС в `$APP_DATA/bin` через `ffmpeg-sidecar`.

На macOS с Homebrew можно поставить заранее:

```bash
brew install ffmpeg
```

На Ubuntu:

```bash
sudo apt install ffmpeg
```

FFmpeg не коммитится в репозиторий.

## Тесты и проверка сборки

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

## Сборка бандлов

```bash
npm run tauri build
```

Артефакты появятся в `src-tauri/target/release/bundle`:

- macOS — `.app` и `.dmg`;
- Windows — `.msi` и `.exe`;
- Linux — `.deb` и `.AppImage`.

Для распространения macOS-сборки нужно подписать приложение Developer ID и нотаризовать его. Для Windows — подписать установщик. Эти шаги в CI без секретов не выполняются.

## Кроссплатформенные замечания

- Все пути строятся через `PathBuf`/Tauri; в UI показывается путь текущей ОС.
- Имена экспортируемых файлов очищаются от символов, запрещённых Windows/macOS.
- ONNX Runtime линкуется в бинарник через `ort`; отдельный dylib на проверенной macOS-сборке не нужен.
- FFmpeg скачивается для платформы, на которой запущено приложение.
- Проект не использует macOS-only API в Rust-ядре.

## GitHub Actions

Workflow `.github/workflows/build.yml` собирает пять нативных целей: macOS Apple Silicon, Linux x64, Linux ARM64, Windows x64 и Windows ARM64. Linux-сборка использует Ubuntu 24.04 — это требование предсобранного ONNX Runtime. Для каждой цели запускаются `npm test`, `cargo test` и Tauri-сборка без подписи. Секреты для подписи в него не входят.
