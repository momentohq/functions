//! Generates OpenAI embeddings for the Amazon fine-foods reviews dataset.
//! Pre-process the CSV into JSON files of ~12MB and POST each chunk to this
//! Function. The dataset is at:
//! https://www.kaggle.com/datasets/snap/amazon-fine-food-reviews?resource=download
//!
//! ```bash
//! any-json Reviews.csv Reviews.json
//! jq -c '_nwise(10000)' Reviews.json > reviews-chunked.json
//! split -l 1 -da 3 reviews-chunked.json reviews-batchembed.json
//!
//! for file in $(ls reviews-batchembed.json*); do
//!     curl $momento/functions/my-cache/fine-foods-embeddings \
//!         -H "authorization: $momento_api_key" \
//!         -XPOST --data-binary @$file > reviews-index-$i.json
//! done
//! ```

use itertools::Itertools;
use momento_functions_bytes::encoding::{Extract, Json};
use momento_functions_guest_web::{WebEnvironment, WebResponse, WebResult, invoke};
use momento_functions_host_log::{LogDestination, configure_logs};
use momento_functions_http::{Request as HttpRequest, invoke as http_invoke};
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

invoke!(generate_embeddings);
fn generate_embeddings(Json(documents): Json<Vec<DocumentInput>>) -> WebResult<WebResponse> {
    setup_logging()?;

    log::debug!("getting embeddings for {} documents", documents.len());
    let mut response = Vec::with_capacity(documents.len());

    for chunk in &documents.into_iter().chunks(2000) {
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

fn setup_logging() -> WebResult<()> {
    let env = WebEnvironment::load();
    configure_logs([LogDestination::topic(env.function_name()).into()])?;
    Ok(())
}

fn get_embeddings(mut documents: Vec<String>) -> WebResult<Vec<Vec<f32>>> {
    log::debug!("getting embeddings for document with content: {documents:?}");
    for document in &mut documents {
        if document.contains('\n') {
            *document = document.replace('\n', " ");
        }
    }

    let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let response = http_invoke(
        HttpRequest::new("https://api.openai.com/v1/embeddings", "POST")
            .with_header("authorization", format!("Bearer {openai_api_key}"))
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "model": "text-embedding-3-small",
                    "encoding_format": "float",
                    "input": documents,
                })
                .to_string(),
            ),
    )?;
    log::debug!("OpenAI response status: {}", response.status);

    #[derive(Deserialize, Debug)]
    struct EmbeddingResponse {
        data: Vec<EmbeddingData>,
    }
    #[derive(Deserialize, Serialize, Debug)]
    struct EmbeddingData {
        embedding: Vec<f32>,
        index: usize,
    }
    let Json(EmbeddingResponse { mut data }) = Json::<EmbeddingResponse>::extract(response.body)?;
    data.sort_by_key(|d| d.index);
    Ok(data.into_iter().map(|d| d.embedding).collect())
}
