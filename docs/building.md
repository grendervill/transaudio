# Сборка и запуск Transaudio

## Что нужно

- Node.js 20.19+ (лучше 22+).
- Rust 1.90+ и Cargo.
- **CMake и C++ toolchain** — движок `transcribe-cpp` и `sherpa-onnx`
  собираются из исходников при первой сборке.
- Платформенные зависимости Tauri 2:
  - **macOS:** Xcode Command Line Tools (включают clang и make) и CMake
    (`brew install cmake`).
  - **Windows:** Visual Studio Build Tools с C++ workload, CMake и WebView2
    Runtime (в Windows 10/11 обычно уже есть).
  - **Linux:** Ubuntu 24.04 (glibc 2.39) или новее — предсобранный ONNX Runtime,
    который использует sherpa-onnx, требует современный glibc. Нужны WebKitGTK
    4.1, GTK/GLib, OpenSSL, librsvg, CMake и toolchain:
    ```bash
    sudo apt update
    sudo apt install libwebkit2gtk-4.1-dev build-essential cmake curl wget file \
      libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```
- Интернет при первом запуске — для загрузки выбранной модели и, при
  необходимости, FFmpeg и моделей спикеров.

## Запуск для разработки

```bash
git clone <repo>
cd transaudio
npm ci
npm run tauri dev
```

`tauri dev` сам запускает Vite на порту 1420. Окно использует локальные шрифты
из `public/fonts`. Первая сборка C++ занимает несколько минут, дальше работает
кэш.

## Модели

При первом запуске открывается каталог моделей. Скачивается только выбранная,
по требованию, с Hugging Face:

- GGUF-файл кладётся в `$APP_DATA/models/<id>/`;
- размер и SHA-256 берутся из каталога и проверяются после загрузки;
- выбранная модель сохраняется в `settings.json` и загружается при следующем
  запуске;
- переключение модели блокируется, пока выполняется очередь.

`$APP_DATA`:

- macOS: `~/Library/Application Support/com.transaudio.desktop`
- Windows: `%APPDATA%\com.transaudio.desktop`
- Linux: `~/.local/share/com.transaudio.desktop`

Разрешение на загрузку моделей хранится в `settings.json`
(`model_consent: true`) и повторно не запрашивается.

### Диаризация

При первом включении «Спикеров» скачиваются pyannote segmentation 3.0 и CAM++
(~35 МБ). Они лежат в `$APP_DATA/models/diarization`. Проверка SHA-256 такая же.

Сценарий выбирается рядом с переключателем: интервью (2–3), встреча (4–6),
много участников (7+), авто. Для файлов длиннее 2 часов диаризация отключается —
аудио читается в память целиком.

## FFmpeg

Для извлечения аудио из медиа используется FFmpeg. Приложение:

1. ищет `$APP_DATA/bin/ffmpeg`;
2. ищет `ffmpeg` в `PATH`;
3. если не находит — скачивает сборку для текущей ОС в `$APP_DATA/bin` через
   `ffmpeg-sidecar`.

На macOS:

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

Для распространения macOS-сборки нужно подписать приложение Developer ID и
нотаризовать его. Для Windows — подписать установщик. В CI без секретов эти
шаги не выполняются.

## Кроссплатформенные замечания

- Все пути строятся через `PathBuf`/Tauri; в UI показывается путь текущей ОС.
- Имена экспортируемых файлов очищаются от символов, запрещённых Windows/macOS.
- `transcribe-cpp` линкуется статически; Metal включается на macOS
  автоматически, на Windows/Linux используется CPU.
- `sherpa-onnx` линкуется статически; отдельный dylib не нужен.
- FFmpeg скачивается для платформы, на которой запущено приложение.

## GitHub Actions

Workflow `.github/workflows/build.yml` собирает пять нативных целей: macOS Apple
Silicon, Linux x64, Linux ARM64, Windows x64 и Windows ARM64. Linux-сборка
использует Ubuntu 24.04. Для каждой цели запускаются `npm test`, `cargo test` и
Tauri-сборка без подписи. Секреты для подписи в него не входят.

## Если загрузка модели не начинается

Проверьте `transaudio.log` в каталоге данных приложения. В нём записываются
старт приложения, выбранная модель, URL загрузки, HTTP-статус, ошибки сети,
этапы распаковки и загрузка моделей спикеров.

Типичные причины:

- брандмауэр или антивирус блокирует исходящее HTTPS-соединение;
- корпоративный прокси не пропускает `huggingface.co` или `github.com`:
  задайте `HTTPS_PROXY`/`HTTP_PROXY`;
- прерванная загрузка: нажмите «Повторить»; при ошибке SHA-256 удалите папку
  модели в `models/` и повторите.

Соединение устанавливается с таймаутом 15 секунд, поэтому зависший запрос
превращается в видимую ошибку, а не в «мёртвую» кнопку.
