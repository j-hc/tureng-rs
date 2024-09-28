use miniserde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct RespRoot {
    // #[serde(rename = "AFullTextResults")]
    // pub afull_text_results: Vec<RespResult>,
    #[serde(rename = "AResults")]
    pub aresults: Vec<RespResult>,
    // #[serde(rename = "AccentInsensitive")]
    // pub accent_insensitive: bool,
    // #[serde(rename = "AvailabilityOnOtherDictionaries")]
    // pub availability_on_other_dictionaries: AvailabilityOnOtherDictionaries,
    // #[serde(rename = "BFullTextResults")]
    // pub bfull_text_results: Vec<RespResult>,
    #[serde(rename = "BResults")]
    pub bresults: Vec<RespResult>,
    // #[serde(rename = "HasSlangTerms")]
    // pub has_slang_terms: bool,
    // #[serde(rename = "IsFound")]
    // pub is_found: bool,
    // #[serde(rename = "IsSearchTermSlang")]
    // pub is_search_term_slang: bool,
    // #[serde(rename = "PrimeATerm")]
    // pub prime_aterm: String,
    // #[serde(rename = "SearchedTerm")]
    // pub searched_term: String,
    // #[serde(rename = "Suggestions")]
    // pub suggestions: Value,
    // #[serde(rename = "VoiceId")]
    // pub voice_id: String,
    // #[serde(rename = "VoiceLanguage")]
    // pub voice_language: String,
    // #[serde(rename = "Voices")]
    // pub voices: Vec<Voice>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RespResult {
    // #[serde(rename = "CategoryId")]
    // pub category_id: String,
    #[serde(rename = "CategoryTextA")]
    pub category_text_a: String,
    #[serde(rename = "CategoryTextB")]
    pub category_text_b: String,
    // #[serde(rename = "IsSlang")]
    // pub is_slang: bool,
    // #[serde(rename = "SourceId")]
    // pub source_id: String,
    // #[serde(rename = "Tags")]
    // pub tags: String,
    #[serde(rename = "TermA")]
    pub term_a: String,
    #[serde(rename = "TermB")]
    pub term_b: String,
    // #[serde(rename = "TermTypeId")]
    // pub term_type_id: String,
    #[serde(rename = "TermTypeTextA")]
    pub term_type_text_a: Option<String>,
    #[serde(rename = "TermTypeTextB")]
    pub term_type_text_b: Option<String>,
    // #[serde(rename = "TranslationId")]
    // pub translation_id: String,
}

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct AvailabilityOnOtherDictionaries {
//     pub ende: bool,
//     pub enes: bool,
//     pub enfr: bool,
//     pub entr: bool,
// }

// #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct Voice {
//     #[serde(rename = "VoiceAccent")]
//     pub voice_accent: String,
//     #[serde(rename = "VoiceUrl")]
//     pub voice_url: String,
// }
