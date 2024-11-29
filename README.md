# MrDocument

## Synopsis

MrDocument watches a directory on the filesystem.
Whenever a PDF file appears in this directory,
it is sent to ChatGPT.
There it is transcribed, summarized and categorized by document class and keywords.
Keywords are added to the PDF's metadata.
The PDF file is also renamed according to the schema `{DATE}-{CLASS}-{TITLE}`
where date, class and title are determined by the AI depending on the document's content.

## Getting Started

When you first run MrDocument, it will create a default profile in `{CONFIG}/MrDocument/profile`
where `{CONFIG}` is your user's configuration directory, e.g. `~/.config` or `~/Library/Application Support`.
MrDocument will probably complain about a missing OpenAI API key at this point.
Store your API key in `{CONFIG}/MrDocument/openai-api-key`.

## Profiles

The default profile will point to `{HOME}/MrDocument`.
Inside this directory the subdirs `inbox`, `outbox`, `transit`, `processed` and `error` will be created.
Placing a file in the inbox will make MrDocument process it.
The file will move through `transit` into `processed` or `error`.
If there is no error, the result will be placed in `outbox`.

The subdirs can be renamed in the profile.
Additional profiles can be created too.
You can also set custom instructions for a profile, although this is rather rudimentary right now.

Please note: Creating or writing a file in the profile directory will instantly load or reload it.
So take care not to save the profile in an inconsistent state,
or copy it elsewhere for editing.

## Installation

For MacOS, there is an installer `mrdocument-install` that will create a background service.
