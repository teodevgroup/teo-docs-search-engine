#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use std::env::current_dir;
use std::fs;
use std::sync::Arc;
use std::sync::Mutex;
use napi::{Error, Result, Status};
use once_cell::sync::Lazy;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::ReloadPolicy;
use tantivy::{schema::{Schema, STORED, TEXT}, Document, Index, IndexWriter};

#[napi]
pub struct Record {
    pub filepath: String,
    pub url_path: String,
    pub content: String,
}

static SCHEMA: Lazy<Schema> = Lazy::new(|| {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("filepath", TEXT | STORED);
    schema_builder.add_text_field("urlPath", TEXT | STORED);
    schema_builder.add_text_field("content", TEXT | STORED);
    schema_builder.build()
});

static INDEX: Lazy<Index> = Lazy::new(|| {
    let path = current_dir().unwrap();
    let tmp_dir = path.join(".fulltextcache");
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir).unwrap();
    }
    fs::create_dir(&tmp_dir).unwrap();
    let index = Index::create_in_dir(&tmp_dir, SCHEMA.clone()).unwrap();
    index
});

static INDEX_WRITER: Lazy<Arc<Mutex<IndexWriter>>> = Lazy::new(|| {
    let index_writer = INDEX.writer(50_000_000).unwrap();
    Arc::new(Mutex::new(index_writer))
});

#[napi]
pub fn build_insert_record(filepath: String, url_path: String, content: String) -> Result<()> {
    let filepath_field = SCHEMA.get_field("filepath").unwrap();
    let url_path_field = SCHEMA.get_field("urlPath").unwrap();
    let content_field = SCHEMA.get_field("content").unwrap();
    let mut document = Document::default();
    document.add_text(filepath_field, filepath);
    document.add_text(url_path_field, url_path);
    document.add_text(content_field, content);
    INDEX_WRITER.lock().unwrap().add_document(document).unwrap();
    Ok(())
}

#[napi]
pub fn build_commit() -> Result<()> {
    match INDEX_WRITER.lock().unwrap().commit() {
        Ok(_) => Ok(()),
        Err(err) => Err(Error::new(Status::InvalidArg, err.to_string()))
    }
}

#[napi]
pub fn search(text: String) -> Result<Vec<Record>> {
    let content_field = SCHEMA.get_field("content").unwrap();
    let reader = INDEX
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into().unwrap();
    let searcher = reader.searcher();
    let query_parser = QueryParser::for_index(&INDEX, vec![content_field]);
    let query = query_parser.parse_query(text.as_str()).unwrap();
    let top_docs = searcher.search(&query, &TopDocs::with_limit(10000)).unwrap();
    let mut result = vec![];
    for (_score, doc_address) in top_docs {
        let filepath_field = SCHEMA.get_field("filepath").unwrap();
        let url_path_field = SCHEMA.get_field("urlPath").unwrap();    
        let retrieved_doc = searcher.doc(doc_address).unwrap();
        let filepath_value = retrieved_doc.get_first(filepath_field).unwrap().as_text().unwrap();
        let url_path_value = retrieved_doc.get_first(url_path_field).unwrap().as_text().unwrap();
        let content_value = retrieved_doc.get_first(content_field).unwrap().as_text().unwrap();
        result.push(Record {
            content: content_value.to_owned(),
            filepath: filepath_value.to_owned(),
            url_path: url_path_value.to_owned(),
        });
    }
    Ok(result)
}