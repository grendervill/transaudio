use std::collections::HashMap;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::config::ExportFormat;
use crate::model::EngineSegment;
use crate::transcribe::CompletedTranscript;

pub fn srt_timestamp(seconds: f64) -> String {
    let total_ms = (seconds.max(0.0) * 1000.0).round() as u64;
    let millis = total_ms % 1000;
    let total_seconds = total_ms / 1000;
    let secs = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;
    format!("{hours:02}:{minutes:02}:{secs:02},{millis:03}")
}

pub fn format_duration(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let secs = total % 60;
    let minutes = (total / 60) % 60;
    let hours = total / 3600;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

fn human_timecode(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let secs = total % 60;
    let minutes = (total / 60) % 60;
    let hours = total / 3600;
    format!("{hours:02}:{minutes:02}:{secs:02}")
}

pub fn render(
    format: ExportFormat,
    name: &str,
    segments: &[EngineSegment],
    duration_secs: f64,
) -> String {
    match format {
        ExportFormat::Md => render_markdown(name, segments, duration_secs),
        ExportFormat::Srt => render_srt(segments),
        ExportFormat::Txt => render_txt(segments),
    }
}

fn render_markdown(name: &str, segments: &[EngineSegment], duration_secs: f64) -> String {
    let mut output = format!(
        "# {name}

**Длительность:** {}
**Движок:** Parakeet V3 · локально

---

",
        format_duration(duration_secs)
    );
    for segment in segments {
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }
        output.push_str(&format!(
            "**[{}]** {text}

",
            human_timecode(segment.start)
        ));
    }
    output
}

fn render_srt(segments: &[EngineSegment]) -> String {
    let mut output = String::new();
    for (index, segment) in segments.iter().enumerate() {
        let text = segment.text.trim();
        if text.is_empty() {
            continue;
        }
        let end = segment.end.max(segment.start);
        output.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            index + 1,
            srt_timestamp(segment.start),
            srt_timestamp(end),
            text
        ));
    }
    output
}

fn render_txt(segments: &[EngineSegment]) -> String {
    let mut output = segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if !output.is_empty() {
        output.push('\n');
    }
    output
}

pub fn safe_stem(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());
    for character in name.trim().chars() {
        if is_format_control(character) {
            continue;
        }
        if character.is_control()
            || matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
        {
            sanitized.push('_');
        } else {
            sanitized.push(character);
        }
    }
    let trimmed = sanitized.trim_matches(|character| character == '.' || character == ' ');
    let mut result = String::new();
    for character in trimmed.chars().take(100) {
        if result.len() + character.len_utf8() > 120 {
            break;
        }
        result.push(character);
    }
    if result.is_empty() {
        result = "transcript".into();
    }
    let upper = result.to_uppercase();
    let reserved = matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    if reserved {
        result.push('_');
    }
    result
}

pub fn export_completed(
    results: &HashMap<String, CompletedTranscript>,
    output_dir: &Path,
    ids: &[String],
    format: ExportFormat,
) -> Result<Vec<PathBuf>, String> {
    if ids.is_empty() {
        return Err("Выберите хотя бы одну расшифровку".into());
    }
    let mut written = Vec::with_capacity(ids.len());
    for id in ids {
        let transcript = results
            .get(id)
            .ok_or_else(|| format!("Расшифровка {id} не найдена"))?;
        let stem = Path::new(&transcript.name)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(&transcript.name);
        written.push(write_result(
            output_dir,
            stem,
            format,
            &transcript.segments,
            transcript.duration_secs,
        )?);
    }
    Ok(written)
}

pub fn write_result(
    output_dir: &Path,
    name: &str,
    format: ExportFormat,
    segments: &[EngineSegment],
    duration_secs: f64,
) -> Result<PathBuf, String> {
    fs::create_dir_all(output_dir)
        .map_err(|error| format!("Не удалось создать папку результатов: {error}"))?;
    let stem = safe_stem(name);
    let content = render(format, name, segments, duration_secs);
    let mut attempt = 0_u32;
    loop {
        let filename = if attempt == 0 {
            format!("{stem}.{}", format.extension())
        } else {
            format!("{stem} ({}).{}", attempt + 1, format.extension())
        };
        let path = output_dir.join(filename);
        let mut temporary = tempfile::NamedTempFile::new_in(output_dir)
            .map_err(|error| format!("Не удалось создать временный файл: {error}"))?;
        temporary
            .write_all(content.as_bytes())
            .map_err(|error| format!("Не удалось записать расшифровку: {error}"))?;
        temporary
            .as_file_mut()
            .sync_all()
            .map_err(|error| format!("Не удалось сохранить расшифровку: {error}"))?;
        match temporary.persist_noclobber(&path) {
            Ok(_) => return Ok(path),
            Err(error) if error.error.kind() == ErrorKind::AlreadyExists => {
                attempt = attempt.saturating_add(1);
                if attempt > 10_000 {
                    return Err("Слишком много файлов с одинаковым именем".into());
                }
            }
            Err(error) => {
                return Err(format!("Не удалось сохранить расшифровку: {}", error.error));
            }
        }
    }
}

fn is_format_control(character: char) -> bool {
    matches!(
        character as u32,
        0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0xFEFF
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use crate::config::ExportFormat;
    use crate::model::EngineSegment;

    use super::{
        export_completed, format_duration, render, safe_stem, srt_timestamp, write_result,
    };

    fn segments() -> Vec<EngineSegment> {
        vec![
            EngineSegment {
                start: 3.0,
                end: 7.5,
                text: "Первая фраза.".into(),
            },
            EngineSegment {
                start: 8.25,
                end: 12.0,
                text: "Вторая фраза.".into(),
            },
        ]
    }

    #[test]
    fn formats_srt_timestamps_with_carry() {
        assert_eq!(srt_timestamp(0.0), "00:00:00,000");
        assert_eq!(srt_timestamp(3.0), "00:00:03,000");
        assert_eq!(srt_timestamp(59.9996), "00:01:00,000");
        assert_eq!(srt_timestamp(3661.5), "01:01:01,500");
    }

    #[test]
    fn formats_human_duration() {
        assert_eq!(format_duration(0.0), "0:00");
        assert_eq!(format_duration(62.0), "1:02");
        assert_eq!(format_duration(3723.0), "1:02:03");
    }

    #[test]
    fn renders_srt() {
        let output = render(ExportFormat::Srt, "Лекция", &segments(), 12.0);
        assert!(output.contains("1\n00:00:03,000 --> 00:00:07,500\nПервая фраза."));
        assert!(output.contains("2\n00:00:08,250 --> 00:00:12,000\nВторая фраза."));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn renders_markdown_with_header_and_timecodes() {
        let output = render(ExportFormat::Md, "Лекция", &segments(), 12.0);
        assert!(output.starts_with("# Лекция\n"));
        assert!(output.contains("**Длительность:** 0:12"));
        assert!(output.contains("**[00:00:03]** Первая фраза."));
        assert!(output.contains("**[00:00:08]** Вторая фраза."));
    }

    #[test]
    fn renders_plain_text_without_timecodes() {
        let output = render(ExportFormat::Txt, "Лекция", &segments(), 12.0);
        assert_eq!(output, "Первая фраза.\n\nВторая фраза.\n");
    }

    #[test]
    fn sanitizes_names_for_all_platforms() {
        assert_eq!(safe_stem("Лекция: часть 1/2?"), "Лекция_ часть 1_2_");
        assert_eq!(safe_stem("CON"), "CON_");
        assert_eq!(safe_stem("  ...  "), "transcript");
        assert_eq!(safe_stem(""), "transcript");
        assert_eq!(safe_stem("нормальное имя"), "нормальное имя");
        let long = safe_stem(&"я".repeat(130));
        assert!(long.chars().count() <= 100);
        assert!(long.len() <= 120);
    }

    #[test]
    fn writes_file_with_correct_extension() {
        let temp = TempDir::new().unwrap();
        let path = write_result(
            temp.path(),
            "Лекция 11",
            ExportFormat::Srt,
            &segments(),
            12.0,
        )
        .unwrap();
        assert_eq!(path.file_name().unwrap(), "Лекция 11.srt");
        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("00:00:03,000"));
    }
    #[test]
    fn exports_selected_completed_results() {
        use crate::transcribe::CompletedTranscript;
        use std::collections::HashMap;

        let temp = TempDir::new().unwrap();
        let mut results = HashMap::new();
        results.insert(
            "a".to_string(),
            CompletedTranscript {
                id: "a".into(),
                name: "Интервью.mp3".into(),
                source_path: "/tmp/Интервью.mp3".into(),
                segments: segments(),
                duration_secs: 12.0,
                saved_path: temp.path().join("Интервью.md").to_string_lossy().into(),
            },
        );
        results.insert(
            "b".to_string(),
            CompletedTranscript {
                id: "b".into(),
                name: "Лекция.mp4".into(),
                source_path: "/tmp/Лекция.mp4".into(),
                segments: segments(),
                duration_secs: 12.0,
                saved_path: temp.path().join("Лекция.md").to_string_lossy().into(),
            },
        );

        let written =
            export_completed(&results, temp.path(), &["b".into()], ExportFormat::Srt).unwrap();

        assert_eq!(written.len(), 1);
        assert_eq!(written[0].file_name().unwrap(), "Лекция.srt");
        assert!(!temp.path().join("Интервью.srt").exists());
    }

    #[test]
    fn rejects_unknown_result_id_during_export() {
        use std::collections::HashMap;
        let temp = TempDir::new().unwrap();
        let results = HashMap::new();
        let error = export_completed(
            &results,
            temp.path(),
            &["missing".into()],
            ExportFormat::Txt,
        )
        .unwrap_err();
        assert!(error.contains("не найдена"));
    }
    #[test]
    fn never_overwrites_an_existing_transcript() {
        let temp = TempDir::new().unwrap();
        let first =
            write_result(temp.path(), "Выпуск", ExportFormat::Md, &segments(), 12.0).unwrap();
        let second =
            write_result(temp.path(), "Выпуск", ExportFormat::Md, &segments(), 12.0).unwrap();

        assert_ne!(first, second);
        assert!(first.is_file());
        assert!(second.is_file());
        assert_eq!(second.file_name().unwrap(), "Выпуск (2).md");
    }

    #[test]
    fn limits_sanitized_names_by_utf8_bytes() {
        let name = safe_stem(&"😀".repeat(100));
        assert!(name.len() <= 120);
        assert!(!name.is_empty());
    }
}
