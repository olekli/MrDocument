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
use serde_json::json;

fn default_tools() -> Vec<Tool> {
    vec![serde_json::from_value(json!({
        "type": "function",
        "function": {
            "name": "return_document_data",
            "description": "Please use this function to return the transcribed content \
                of the document, your summary of the content, \
                your classification of the document, the keywords you assigned, \
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
    })).unwrap()]
}

fn default_instructions() -> Vec<ChatCompletionMessage> {
    vec![serde_json::from_value(json!({
        "role": "system",
        "content": "You will be given a scan of a document. \
            It may consist of one or more pages. \
            You shall provide as output: \n\
            (1) A transcription of the contents of the document. \
            If the document is too large to provide a full transcription, \
            you may omit this. \n\
            (2) A summary of the content of the entire document. \n\
            (3) A classification of the document. \
            Please use rather broad and general concepts as classes. \
            The class must be usable as part of a filename and must not contain whitespaces or non-ascii characters. \
            The grammatical number of the word used as class should be singular if possible. \n\
            (4) Between 2 and 5 keywords describing the content of the document. \
            The class of the document should be one the keywords. \n\
            (5) A title describing the document. \
            It should be sufficiently specific to differentiate \
            this particular document from other documents of this class or with similar keywords. \
            The title must be usable as part of a filename and must not contain whitespaces or non-ascii characters. \n\
            (6) A date to be associated with the document.\n
            All output shall be in the language of the document.\n"
    })).unwrap()]
}

pub async fn query_ai(profile: ChatGptProfile, file_info: FileInfo) -> Result<DocumentData> {
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
    let mut messages = default_instructions();
    for instr in profile.additional_instructions {
        messages.push(ChatCompletionMessage {
            role: MessageRole::system,
            content: Content::Text(instr),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }
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
    let response = client.chat_completion(req).await?;
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
