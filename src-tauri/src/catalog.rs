use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFamily {
    Parakeet,
    Canary,
    Gigaam,
    Whisper,
    Sensevoice,
    Qwen,
}

#[derive(Debug, Clone, Copy)]
pub struct ModelSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub family: ModelFamily,
    pub languages: &'static [&'static str],
    pub language_count: u16,
    pub quality_en: Option<f32>,
    pub quality_ru: Option<f32>,
    pub speed_x: f32,
    pub size_bytes: u64,
    pub filename: &'static str,
    pub repository: &'static str,
    pub revision: &'static str,
    pub sha256: &'static str,
    pub license: &'static str,
    pub needs_language: bool,
    pub recommended_for: &'static [&'static str],
    pub badge: Option<&'static str>,
}

const EUROPE_25: &[&str] = &[
    "bg", "hr", "cs", "da", "nl", "en", "et", "fi", "fr", "de", "el", "hu", "it", "lv", "lt", "mt",
    "pl", "pt", "ro", "ru", "sk", "sl", "es", "sv", "uk",
];
const FOUR: &[&str] = &["en", "de", "es", "fr"];
const EN: &[&str] = &["en"];
const RU: &[&str] = &["ru"];
const ASIAN_5: &[&str] = &["zh", "yue", "en", "ja", "ko"];
const WHISPER_SAMPLE: &[&str] = &[
    "en", "ru", "de", "fr", "es", "it", "pt", "uk", "pl", "nl", "tr", "ar", "zh", "ja", "ko",
];
const QWEN_SAMPLE: &[&str] = &[
    "zh", "en", "yue", "ar", "de", "fr", "es", "it", "ja", "ko", "pt", "ru", "tr", "vi",
];

pub static MODELS: &[ModelSpec] = &[
    ModelSpec {
        id: "parakeet-tdt-0.6b-v3",
        name: "Parakeet TDT 0.6B v3",
        description: "Быстрая и точная, хороший выбор для 25 европейских языков.",
        family: ModelFamily::Parakeet,
        languages: EUROPE_25,
        language_count: 25,
        quality_en: Some(4.83),
        quality_ru: Some(6.54),
        speed_x: 13.66,
        size_bytes: 739_508_576,
        filename: "parakeet-tdt-0.6b-v3-Q8_0.gguf",
        repository: "handy-computer/parakeet-tdt-0.6b-v3-gguf",
        revision: "85ac09ea12fc4b1112fa76810059364bc6adc9de",
        sha256: "5859f77944efcd8eafa23a6350731960b2b55b2203df51f319665c807d802cc7",
        license: "CC-BY-4.0",
        needs_language: false,
        recommended_for: &[],
        badge: None,
    },
    ModelSpec {
        id: "parakeet-unified-en-0.6b",
        name: "Parakeet Unified English",
        description: "Лучшая точность для английского при высокой скорости.",
        family: ModelFamily::Parakeet,
        languages: EN,
        language_count: 1,
        quality_en: Some(3.99),
        quality_ru: None,
        speed_x: 13.96,
        size_bytes: 731_357_568,
        filename: "parakeet-unified-en-0.6b-Q8_0.gguf",
        repository: "handy-computer/parakeet-unified-en-0.6b-gguf",
        revision: "7e948f21b7bdbac698d3318db9d350f1096f3b6c",
        sha256: "4b50b6dd862bf6e346929aaf4f5eaacec003bfa3f56462d6c874b41ef2f38795",
        license: "CC-BY-4.0",
        needs_language: false,
        recommended_for: &["en"],
        badge: None,
    },
    ModelSpec {
        id: "parakeet-ultra",
        name: "Parakeet Ultra",
        description: "Максимальная точность среди мультиязычных Parakeet.",
        family: ModelFamily::Parakeet,
        languages: EUROPE_25,
        language_count: 25,
        quality_en: Some(4.26),
        quality_ru: Some(5.63),
        speed_x: 13.29,
        size_bytes: 740_363_168,
        filename: "parakeet-ultra-Q8_0.gguf",
        repository: "handy-computer/parakeet-ultra-gguf",
        revision: "39eeb55181f0d354fd934f06e92fd8d5037fed8e",
        sha256: "007a59761e9258779f189df396b50d12b83bb5b79e66df4b955c230d2f2a0a59",
        license: "CC-BY-4.0",
        needs_language: false,
        recommended_for: &[],
        badge: None,
    },
    ModelSpec {
        id: "parakeet-tdt_ctc-110m",
        name: "Parakeet 110M",
        description: "Крошечная английская модель, почти мгновенная.",
        family: ModelFamily::Parakeet,
        languages: EN,
        language_count: 1,
        quality_en: Some(6.11),
        quality_ru: None,
        speed_x: 50.03,
        size_bytes: 135_373_280,
        filename: "parakeet-tdt_ctc-110m-Q8_0.gguf",
        repository: "handy-computer/parakeet-tdt_ctc-110m-gguf",
        revision: "9d66d34f9e1594075c5dd72c90c0f4c321b29f21",
        sha256: "7dd44c74a331d788a4e5f8b16913b3feb29ced22cf5613aad0e0f6cd30516296",
        license: "CC-BY-4.0",
        needs_language: false,
        recommended_for: &[],
        badge: Some("мгновенная"),
    },
    ModelSpec {
        id: "canary-180m-flash",
        name: "Canary 180M Flash",
        description: "Компактная и быстрая для английского, немецкого, испанского и французского.",
        family: ModelFamily::Canary,
        languages: FOUR,
        language_count: 4,
        quality_en: Some(5.98),
        quality_ru: None,
        speed_x: 21.73,
        size_bytes: 218_447_648,
        filename: "canary-180m-flash-Q8_0.gguf",
        repository: "handy-computer/canary-180m-flash-gguf",
        revision: "46b86384d1db59108d6e060a0fc00924cb58499f",
        sha256: "f942ab746040b531a29ab78ca3afc8dc9564efd8e4c942b5541e53644be9c408",
        license: "CC-BY-4.0",
        needs_language: true,
        recommended_for: &[],
        badge: None,
    },
    ModelSpec {
        id: "canary-1b-v2",
        name: "Canary 1B v2",
        description: "Высокая точность для 25 европейских языков; укажите язык записи.",
        family: ModelFamily::Canary,
        languages: EUROPE_25,
        language_count: 25,
        quality_en: Some(4.47),
        quality_ru: Some(7.83),
        speed_x: 8.33,
        size_bytes: 1_144_290_016,
        filename: "canary-1b-v2-Q8_0.gguf",
        repository: "handy-computer/canary-1b-v2-gguf",
        revision: "58d13c2c0102229aad45f7e19a77ddc42b41dd9a",
        sha256: "224f83d1bc487b3303b495a7d6874912fdece93de19d1a04b550829c30a5d289",
        license: "CC-BY-4.0",
        needs_language: true,
        recommended_for: &[],
        badge: None,
    },
    ModelSpec {
        id: "gigaam-v3-e2e-rnnt",
        name: "GigaAM v3 E2E-RNNT",
        description: "Лучший выбор для русского; английский ненадёжен.",
        family: ModelFamily::Gigaam,
        languages: RU,
        language_count: 1,
        quality_en: None,
        quality_ru: Some(5.35),
        speed_x: 7.88,
        size_bytes: 273_724_832,
        filename: "gigaam-v3-e2e-rnnt-Q8_0.gguf",
        repository: "handy-computer/gigaam-v3-e2e-rnnt-gguf",
        revision: "f719d70812344f4d0fb8c11c0887b190501a7465",
        sha256: "78d63b47723b7f8d78c6113a6ef983b5a86e2a86f6c273e1f5cb6967b1c4467a",
        license: "MIT",
        needs_language: false,
        recommended_for: &["ru"],
        badge: None,
    },
    ModelSpec {
        id: "SenseVoiceSmall",
        name: "SenseVoice Small",
        description: "Мгновенная расшифровка китайского, японского и корейского.",
        family: ModelFamily::Sensevoice,
        languages: ASIAN_5,
        language_count: 5,
        quality_en: Some(7.14),
        quality_ru: None,
        speed_x: 19.74,
        size_bytes: 241_000_000,
        filename: "SenseVoiceSmall-Q8_0.gguf",
        repository: "handy-computer/SenseVoiceSmall-gguf",
        revision: "4a08b8e900b38a977e32eb08d5d0697d6e72ba04",
        sha256: "6c759ee4c9748c9b3f7a5a60ca74f0f7e685fb9d45d1378fce7cfd62f59adf29",
        license: "Model license",
        needs_language: false,
        recommended_for: &[],
        badge: Some("азиатские"),
    },
    ModelSpec {
        id: "whisper-small",
        name: "Whisper Small",
        description: "99 языков в компактной модели.",
        family: ModelFamily::Whisper,
        languages: WHISPER_SAMPLE,
        language_count: 99,
        quality_en: Some(6.51),
        quality_ru: Some(11.90),
        speed_x: 5.86,
        size_bytes: 269_751_136,
        filename: "whisper-small-Q8_0.gguf",
        repository: "handy-computer/whisper-small-gguf",
        revision: "c0214bd34be9296695486f838e0142f900803159",
        sha256: "9b9c8811bbcc82a7766f0fb0925614bdacb0923b2cc630daeac17108b655b860",
        license: "Apache-2.0",
        needs_language: false,
        recommended_for: &[],
        badge: None,
    },
    ModelSpec {
        id: "whisper-large-v3-turbo",
        name: "Whisper Large v3 Turbo",
        description: "Высокая точность на многих языках, медленнее на CPU.",
        family: ModelFamily::Whisper,
        languages: WHISPER_SAMPLE,
        language_count: 100,
        quality_en: Some(4.38),
        quality_ru: Some(5.93),
        speed_x: 1.43,
        size_bytes: 886_381_760,
        filename: "whisper-large-v3-turbo-Q8_0.gguf",
        repository: "handy-computer/whisper-large-v3-turbo-gguf",
        revision: "5eaf945c7978e564bae5b28a5b1639dd93c2bfb1",
        sha256: "b2e30cc286bc9f3aba4db9099fc7403543497c05ce7100d0d83091ddfd25a183",
        license: "Apache-2.0",
        needs_language: false,
        recommended_for: &[],
        badge: None,
    },
    ModelSpec {
        id: "whisper-large-v3",
        name: "Whisper Large v3",
        description: "Максимальная точность, но на CPU работает примерно в реальном времени.",
        family: ModelFamily::Whisper,
        languages: WHISPER_SAMPLE,
        language_count: 100,
        quality_en: Some(4.03),
        quality_ru: Some(4.96),
        speed_x: 0.97,
        size_bytes: 1_668_741_440,
        filename: "whisper-large-v3-Q8_0.gguf",
        repository: "handy-computer/whisper-large-v3-gguf",
        revision: "e3e29bee6389c7da4a141406f07bb80ddac5337c",
        sha256: "2fa1a5f179f8a511a53e2108db270aa4af3ce08cd976af4180e2854666bb4ba3",
        license: "Apache-2.0",
        needs_language: false,
        recommended_for: &[],
        badge: Some("максимум точности"),
    },
    ModelSpec {
        id: "Qwen3-ASR-0.6B",
        name: "Qwen3-ASR 0.6B",
        description: "Сильный для китайского и азиатских языков, понимает английский.",
        family: ModelFamily::Qwen,
        languages: QWEN_SAMPLE,
        language_count: 30,
        quality_en: Some(4.23),
        quality_ru: Some(10.30),
        speed_x: 5.29,
        size_bytes: 811_000_000,
        filename: "Qwen3-ASR-0.6B-Q8_0.gguf",
        repository: "handy-computer/Qwen3-ASR-0.6B-gguf",
        revision: "e4e16599b900eb0cb36e524514756bb92eb092b7",
        sha256: "f081b2d5e23bd669d92cc331d722a8a0681943b8e6f34b48996fd5c319b5acd8",
        license: "Apache-2.0",
        needs_language: false,
        recommended_for: &[],
        badge: Some("азиатские"),
    },
];

pub fn find(id: &str) -> Option<&'static ModelSpec> {
    MODELS.iter().find(|model| model.id == id)
}

pub fn default_model() -> &'static ModelSpec {
    &MODELS[0]
}

pub fn download_url(spec: &ModelSpec) -> String {
    format!(
        "https://huggingface.co/{}/resolve/{}/{}",
        spec.repository, spec.revision, spec.filename
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_twelve_unique_models() {
        assert_eq!(MODELS.len(), 12);
        let mut ids: Vec<_> = MODELS.iter().map(|model| model.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), MODELS.len());
    }

    #[test]
    fn every_model_has_real_download_metadata() {
        for model in MODELS {
            assert!(model.size_bytes > 50_000_000, "{}", model.id);
            assert_eq!(model.sha256.len(), 64, "{}", model.id);
            assert!(model.sha256.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(model.filename.ends_with(".gguf"), "{}", model.id);
            let url = download_url(model);
            assert!(url.contains(model.repository), "{}", model.id);
            assert!(url.contains(model.revision), "{}", model.id);
            assert!(url.ends_with(model.filename), "{}", model.id);
        }
    }

    #[test]
    fn recommendations_are_only_for_russian_and_english() {
        for model in MODELS {
            for language in model.recommended_for {
                assert!(*language == "ru" || *language == "en", "{}", model.id);
            }
        }
        assert_eq!(find("gigaam-v3-e2e-rnnt").unwrap().recommended_for, &["ru"]);
        assert_eq!(
            find("parakeet-unified-en-0.6b").unwrap().recommended_for,
            &["en"]
        );
    }

    #[test]
    fn only_canary_requires_an_explicit_language() {
        for model in MODELS {
            let expected = matches!(model.family, ModelFamily::Canary);
            assert_eq!(model.needs_language, expected, "{}", model.id);
        }
    }
}
