//! This example uses amazon data and openai embedding apis.
//! You'll need to pre-process the csv into json files of 12MB or thereabouts.
//!
//! ```bash
//! any-json Reviews.csv Reviews.json
//! jq -c '_nwise(10000)' Reviews.json > reviews-chunked.json
//! split -l 1 -da 3 reviews-chunked.json reviews-batchembed.json
//!
//! export API_KEY=$momento_api_key
//!
//! i=0
//! for file in $(ls reviews-batchembed.json*); do
//!     curl \
//!         $momento/functions/my-cache/fine-foods-embeddings \
//!         -H "authorization: $API_KEY" \
//!         -XPOST \
//!         --data-binary @$file > reviews-index-$i.json
//!     i=$((i + 1))
//! done
//! ```
//!
//! The fine foods dataset for this example can be found here:
//! https://www.kaggle.com/datasets/snap/amazon-fine-food-reviews?resource=download

use itertools::Itertools;
use log::LevelFilter;
use momento_functions::{WebResponse, WebResult};
use momento_functions_host::{
    encoding::Json,
    logging::{ConfigureLoggingInput, LogDestination},
    web_extensions::headers,
};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
struct DocumentInput {
    #[serde(alias = "Id")]
    id: String,
    #[serde(alias = "ProductId")]
    product_id: String,
    #[serde(alias = "UserId")]
    user_id: String,
    #[serde(alias = "ProfileName")]
    profile_name: String,
    #[serde(alias = "HelpfulnessNumerator", deserialize_with = "parse_i32")]
    helpfulness_numerator: i32,
    #[serde(alias = "HelpfulnessDenominator", deserialize_with = "parse_i32")]
    helpfulness_denominator: i32,
    #[serde(alias = "Score", deserialize_with = "parse_i32")]
    score: i32,
    #[serde(alias = "Time", deserialize_with = "parse_u32")]
    time: u32,
    #[serde(alias = "Summary")]
    summary: String,
    #[serde(alias = "Text")]
    text: String,
}

fn parse_i32<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    buf.parse().map_err(serde::de::Error::custom)
}

fn parse_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    buf.parse().map_err(serde::de::Error::custom)
}

#[derive(Serialize, Debug)]
struct DocumentOutput {
    embedding: Vec<f32>,
    id: String,
    product_id: String,
    user_id: String,
    profile_name: String,
    helpfulness_numerator: i32,
    helpfulness_denominator: i32,
    score: i32,
    time: u32,
    summary: String,
    text: String,
}

momento_functions::post!(generate_embeddings);
fn generate_embeddings(Json(documents): Json<Vec<DocumentInput>>) -> WebResult<WebResponse> {
    let headers = headers();
    setup_logging(&headers)?;

    log::debug!("getting embeddings for {} documents", documents.len());
    let mut response = Vec::with_capacity(documents.len());

    let chunks = documents.into_iter().chunks(2000);
    for chunk in chunks.into_iter() {
        let chunk: Vec<_> = chunk.collect();
        let embeddings = get_embeddings(chunk.iter().map(|d| d.text.clone()).collect())?;
        response.extend(chunk.into_iter().zip(embeddings).map(|(input, embedding)| {
            DocumentOutput {
                embedding,
                id: input.id,
                product_id: input.product_id,
                user_id: input.user_id,
                profile_name: input.profile_name,
                helpfulness_numerator: input.helpfulness_numerator,
                helpfulness_denominator: input.helpfulness_denominator,
                score: input.score,
                time: input.time,
                summary: input.summary,
                text: input.text,
            }
        }));
    }

    Ok(WebResponse::new()
        .with_status(200)
        .with_body(Json(response))?)
}

// ------------------------------------------------------
// | Utility functions for convenience
// ------------------------------------------------------

fn setup_logging(headers: &[(String, String)]) -> WebResult<()> {
    let log_level = headers.iter().find_map(|(name, value)| {
        if name == "x-momento-log" {
            Some(value)
        } else {
            None
        }
    });
    if let Some(log_level) = log_level {
        let log_level = log_level
            .parse::<LevelFilter>()
            .unwrap_or(LevelFilter::Info);
        momento_functions_log::configure_logging(
            log_level,
            vec![ConfigureLoggingInput::new(LogDestination::Topic {
                topic: "fine-foods-embedding".to_string(),
            })],
        )?;
    }
    Ok(())
}

fn get_embeddings(mut documents: Vec<String>) -> WebResult<Vec<Vec<f32>>> {
    log::debug!("getting embeddings for document with content: {documents:?}");
    for document in &mut documents {
        if document.contains("\n") {
            // openai guide currently says to replace newlines with spaces. This, then, must be how you get the cargo to come.
            // https://platform.openai.com/docs/guides/embeddings
            *document = document.replace("\n", " ");
        }
    }
    // compile-time environment variable.
    // Runtime environment variable secrets will be supported in the future.
    const OPENAI_API_KEY: &str = env!("OPENAI_API_KEY");
    let result = momento_functions_host::http::post(
        "https://api.openai.com/v1/embeddings",
        [
            (
                "authorization".to_string(),
                format!("Bearer {OPENAI_API_KEY}"),
            ),
            ("content-type".to_string(), "application/json".to_string()),
        ],
        // 1536 float32 for text-embedding-3-small
        serde_json::json!({
            "model": "text-embedding-3-small",
            "encoding_format": "float",
            "input": documents,
        })
        .to_string(),
    );
    log::debug!("OpenAI response: {result:?}");
    let mut response = result?;

    #[derive(Deserialize, Debug)]
    struct EmbeddingResponse {
        data: Vec<EmbeddingData>,
    }
    #[derive(Deserialize, Serialize, Debug)]
    struct EmbeddingData {
        embedding: Vec<f32>,
        index: usize,
    }
    let Json(EmbeddingResponse { mut data }) = response.extract()?;

    data.sort_by_key(|d| d.index);
    Ok(data.into_iter().map(|d| d.embedding).collect())
}
