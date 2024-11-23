use crate::document::DocumentData;
use crate::error::{Error, Result};
use crate::file_info::FileInfo;
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::{
    ChatCompletionMessage, ChatCompletionRequest, MessageRole, Tool, ToolChoiceType, ToolType,
};
use openai_api_rs::v1::chat_completion::{Content, ContentType, ImageUrl, ImageUrlType};
use openai_api_rs::v1::common::GPT4_O;
use openai_api_rs::v1::types::{Function, FunctionParameters, JSONSchemaDefine, JSONSchemaType};
use serde_json;
use std::collections::HashMap;
use crate::api_key;

pub async fn query_ai(file_info: FileInfo) -> Result<DocumentData> {
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

    let tools = vec![
        Tool {
            r#type: ToolType::Function,
            function: Function {
                name: "return_document_content".to_string(),
                description: Some("Please use this function to return the transcribed content of the document, the keywords you assigned, the title you assigned and the date you determined.".to_string()),
                parameters: FunctionParameters {
                    schema_type: JSONSchemaType::Object,
                    properties: Some(HashMap::from(
                        [
                            (
                                "content".to_string(),
                                Box::new(JSONSchemaDefine {
                                    schema_type: Some(JSONSchemaType::String),
                                    description: Some("The contents of the document".to_string()),
                                    ..JSONSchemaDefine::default()
                                })
                            ),
                            (
                                "keywords".to_string(),
                                Box::new(JSONSchemaDefine {
                                    schema_type: Some(JSONSchemaType::Array),
                                    description: Some("The keywords assigned to the document".to_string()),
                                    items: Some(Box::new(JSONSchemaDefine {
                                        schema_type: Some(JSONSchemaType::String),
                                        ..JSONSchemaDefine::default()
                                    })),
                                    ..JSONSchemaDefine::default()
                                })
                            ),
                            (
                                "title".to_string(),
                                Box::new(JSONSchemaDefine {
                                    schema_type: Some(JSONSchemaType::String),
                                    description: Some("The title assigned to the document".to_string()),
                                    ..JSONSchemaDefine::default()
                                })
                            ),
                            (
                                "date".to_string(),
                                Box::new(JSONSchemaDefine {
                                    schema_type: Some(JSONSchemaType::String),
                                    description: Some("The date assigned to the document in YYYY-MM-DD".to_string()),
                                    ..JSONSchemaDefine::default()
                                })
                            ),
                        ]
                    )),
                    required: Some(vec![
                                   "content".to_string(),
                                   "keywords".to_string(),
                                   "title".to_string(),
                                   "date".to_string(),
                    ]),
                }
            }
        },
    ];
    let mut messages = vec![ChatCompletionMessage {
        role: MessageRole::system,
        content: Content::Text("You will be given a scan of a document. It may consist of one or more pages. You shall provide as output: (1) a transcription of the contents of the document in the language of the document, or a summary if you are unable to transcribe the entire document; (2) between 2 and 5 keywords describing the content of the document in the language of the document; (3) a title describing the document in the language of the document, the title must be usable as a filename and must not contain whitespaces; (4) a date to be associated with the document.".to_string()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }];
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
    let req = ChatCompletionRequest::new(GPT4_O.to_string(), messages)
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
