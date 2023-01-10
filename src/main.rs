use futures::StreamExt;
use std::{collections::HashSet, env, io::BufReader, path::PathBuf};
use tempfile::TempDir;

use serde::{Deserialize, Serialize};

use askama::Template;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
struct Phonetic {
    text: Option<String>,
    audio: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Definition {
    definition: String,
    synonyms: Vec<String>,
    antonyms: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Meaning {
    #[serde(rename = "partOfSpeech")]
    part_of_speech: String,
    definitions: Vec<Definition>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Word {
    word: String,
    phonetics: Vec<Phonetic>,
    meanings: Vec<Meaning>,
}

#[derive(Error, Debug)]
enum DownloadError {
    #[error("data store disconnected")]
    ReqwestError(#[from] reqwest::Error),

    #[error("io error")]
    IoError(#[from] std::io::Error),
}

async fn download_audio_file(
    url: String,
    temp_dir: &TempDir,
) -> Result<(String, PathBuf), DownloadError> {
    let file_name = url.split("/").last().expect("No file name found in url");
    let file_path = temp_dir.path().join(file_name);

    let mut tmp_file = tokio::fs::File::create(&file_path).await?;
    let mut byte_stream = reqwest::get(&url).await?.bytes_stream();

    while let Some(item) = byte_stream.next().await {
        tokio::io::copy(&mut item?.as_ref(), &mut tmp_file).await?;
    }

    Ok((file_name.to_owned(), file_path))
}

#[derive(Template)]
#[template(path = "word.md")]
struct WordTemplate<'a> {
    words: &'a Vec<Word>,
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();

    let word = env::args().skip(1).next().expect("word not provided");
    let words: Vec<Word> = client
        .get(format!(
            "https://api.dictionaryapi.dev/api/v2/entries/en/{}",
            word
        ))
        .send()
        .await?
        .json()
        .await?;

    let all_audio_url = words
        .iter()
        .flat_map(|word| word.phonetics.iter())
        .map(|phonetic| phonetic.audio.clone())
        .collect::<HashSet<String>>();

    let word_page = WordTemplate { words: &words };

    let page = word_page.render().unwrap();

    tokio::spawn(async move {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let futures = all_audio_url
            .into_iter()
            .map(|url| download_audio_file(url.clone(), &temp_dir));

        let audio = futures::future::join_all(futures)
            .await
            .into_iter()
            .filter(|output| output.is_ok())
            .map(|output| output.unwrap())
            .filter(|(file_name, _)| file_name.ends_with("us.mp3"))
            .next();

        if let Some((_, file_path)) = audio {
            use rodio::{source::Source, Decoder, OutputStream};
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let file = BufReader::new(std::fs::File::open(file_path).unwrap());
            let source = Decoder::new(file).unwrap();
            stream_handle.play_raw(source.convert_samples()).unwrap();

            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });

    bat::PrettyPrinter::new()
        .input_from_bytes(page.as_bytes())
        .grid(false)
        .header(false)
        .line_numbers(false)
        .paging_mode(bat::PagingMode::Always)
        .language("md")
        .print()
        .unwrap();

    Ok(())
}
