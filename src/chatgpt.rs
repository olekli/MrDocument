use crate::api_key;
use crate::document::DocumentData;
use crate::error::{Error, Result};
use crate::file_info::FileInfo;
use crate::profile::ChatGptProfile;
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::{
    ChatCompletionMessage, ChatCompletionRequest, MessageRole, Tool, ToolChoiceType,
};
use openai_api_rs::v1::chat_completion::{Content, ContentType, ImageUrl, ImageUrlType};
use tokio::time::{timeout, Duration};
use serde_json::json;

fn default_tools() -> Vec<Tool> {
    vec![serde_json::from_value(json!({
        "type": "function",
        "function": {
            "name": "return_document_data",
            "description": "Please use this function to return the transcribed content \
                of the document, your summary of the content, \
                your classification of the document, the source of the document, \
                the keywords you assigned, \
                the title you assigned and the date you determined.",
            "parameters": {
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The contents of the document"
                    },
                    "summary": {
                        "type": "string",
                        "description": "Your summary of the content"
                    },
                    "class": {
                        "type": "string",
                        "description": "The class you assigned to the document"
                    },
                    "source": {
                        "type": "string",
                        "description": "The source you assigned to the document"
                    },
                    "keywords": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "The keywords assigned to the document"
                    },
                    "title": {
                        "type": "string",
                        "description": "The title assigned to the document"
                    },
                    "date": {
                        "type": "string",
                        "description": "The date assigned to the document in YYYY-MM-DD"
                    }
                },
                "required": [
                    "summary",
                    "class",
                    "keywords",
                    "title",
                    "date"
                ]
            }
        }
    }))
    .unwrap()]
}

fn make_outputs() -> Vec<String> {
    vec![
        "* A transcription of the contents of the document. If the document is too large to provide a full transcription, you may omit this.".to_string(),
         "* A summary of the content of the entire document.".to_string(),
         "* A classification of the document. Please use rather broad and general concepts as classes. The class must be usable as part of a filename and must not contain whitespaces or non-ascii characters. Please favor using hyphens over underscores as separators. The grammatical number of the word used as class should be singular if possible.".to_string(),
        "* The source of the document. This could be the author, creator, sender or issuer of the document. The source must be usable as part of a filename and must not contain whitespaces or non-ascii characters.".to_string(),
        "* Between 2 and 4 keywords describing the content of the document.".to_string(),
        "* A title describing the document. It should be sufficiently specific to differentiate this particular document from other documents of this class and source, but it should not duplicate words that are already found as class or source. The title must be usable as part of a filename and must not contain whitespaces or non-ascii characters.".to_string(),
        "* A date to be associated with the document. Please favor the date when the document was issued over any other dates found.".to_string(),
    ]
}

fn make_specs(classes: Vec<String>, sources: Vec<String>) -> Vec<String> {
    let mut result = vec![
        "* Please make sure that the language of all outputs matches the language of the input document.".to_string(),
    ];
    if classes.len() > 0 {
        result.push(format!("* When choosing the class of the document, check if any of these classes match before creating a new one: {}", classes.join(", ")));
    }
    if sources.len() > 0 {
        result.push(format!("* When choosing the source of the document, check if any of these sources match before creating a new one: {}", sources.join(", ")));
    }

    result
}

fn make_instructions(classes: Vec<String>, sources: Vec<String>) -> Vec<ChatCompletionMessage> {
    let outputs = make_outputs().join("\n");
    let specs = make_specs(classes, sources).join("\n");
    vec![serde_json::from_value(json!({
        "role": "system",
        "content": format!("You will be given a scan of a document. It may consist of one or more pages. You shall provide as output in the language of the document:\n{outputs}\n\nWhen producing the output, you shall observe the following points:\n{specs}\n"),
    })).unwrap()]
}

pub async fn query_ai(
    profile: ChatGptProfile,
    file_info: FileInfo,
    classes: Vec<String>,
    sources: Vec<String>,
) -> Result<DocumentData> {
    log::info!("Received {file_info:?}");
    let api_key = api_key::get();
    let client = OpenAIClient::builder()
        .with_api_key(api_key)
        .build()
        .map_err(|_| Error::NoApiKeyError)?;
    let files: Vec<String> = file_info
        .base64()
        .await?
        .into_iter()
        .map(|data| format!("data:{};base64,{}", file_info.mime_type(), data))
        .collect();

    let tools = default_tools();
    let mut messages = make_instructions(classes, sources);
    for instr in profile.additional_instructions {
        messages.push(ChatCompletionMessage {
            role: MessageRole::system,
            content: Content::Text(instr),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }
    log::debug!("Using instructions: {messages:?}");
    for file in files {
        messages.push(ChatCompletionMessage {
            role: MessageRole::user,
            content: Content::ImageUrl(vec![ImageUrl {
                r#type: ContentType::image_url,
                text: None,
                image_url: Some(ImageUrlType { url: file }),
            }]),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }
    let req = ChatCompletionRequest::new(profile.model, messages)
        .temperature(<u8 as Into<f64>>::into(profile.temperature) / 100.0)
        .tools(tools)
        .tool_choice(ToolChoiceType::Required);
    log::info!("Sending {file_info:?}");
    let response = timeout(Duration::from_secs(300), client.chat_completion(req)).await??;
    log::trace!("received response");
    let result_value: Result<serde_json::Value> = (|| {
        Ok(serde_json::from_str(
            &response
                .choices
                .first()
                .ok_or_else(|| Error::DoesNotProcessError(None))?
                .message
                .tool_calls
                .as_ref()
                .ok_or_else(|| Error::DoesNotProcessError(None))?
                .first()
                .ok_or_else(|| Error::DoesNotProcessError(None))?
                .function
                .arguments
                .as_ref()
                .ok_or_else(|| Error::DoesNotProcessError(None))?,
        )?)
    })()
    .map_err(|err| {
        if let Error::DoesNotProcessError(_) = err {
            Error::DoesNotProcessError(Some(response))
        } else {
            err
        }
    });
    let result: Result<DocumentData> = Ok(serde_json::from_value(result_value?)?);

    result
}
