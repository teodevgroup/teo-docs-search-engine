#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use std::fs;
use std::path::Path;
use once_cell::sync::Lazy;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::{ReloadPolicy, schema::{Schema, STORED, TEXT}, TantivyDocument, Index, IndexWriter};
use tantivy::schema::Value;

static SCHEMA: Lazy<Schema> = Lazy::new(|| {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("fileLocation", TEXT | STORED);
    schema_builder.add_text_field("urlPath", TEXT | STORED);
    schema_builder.add_text_field("content", TEXT | STORED);
    schema_builder.build()
});

#[napi(object)]
pub struct Record {
    pub file_location: String,
    pub url_path: String,
    pub content: String,
}

#[napi]
pub struct SearchIndex {
    index: Index
}

impl SearchIndex {

    fn _index(tmp_dir: &Path) -> Index {
        if tmp_dir.exists() {
            fs::remove_dir_all(&tmp_dir).unwrap();
        }
        fs::create_dir(&tmp_dir).unwrap();
        Index::create_in_dir(&tmp_dir, SCHEMA.clone()).unwrap()
    }

    fn _clear(&self) {
        let writer: IndexWriter<TantivyDocument> = self.index.writer(50_000_000).unwrap();
        writer.delete_all_documents().unwrap();
    }

    fn _insert(&self, records: Vec<Record>) {
        let mut writer: IndexWriter<TantivyDocument> = self.index.writer(50_000_000).unwrap();
        for record in records {
            let file_location_field = SCHEMA.get_field("fileLocation").unwrap();
            let url_path_field = SCHEMA.get_field("urlPath").unwrap();
            let content_field = SCHEMA.get_field("content").unwrap();
            let mut document = TantivyDocument::default();
            document.add_text(file_location_field, record.file_location);
            document.add_text(url_path_field, record.url_path);
            document.add_text(content_field, record.content);
            writer.add_document(document).unwrap();
        }
        writer.commit().unwrap();
    }

    fn _search(&self, text: String) -> Vec<Record> {
        let content_field = SCHEMA.get_field("content").unwrap();
        let reader = self.index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into().unwrap();
        let searcher = reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![content_field]);
        let query = query_parser.parse_query(text.as_str()).unwrap();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10000)).unwrap();
        let mut results = vec![];
        for (_score, doc_address) in top_docs {
            let file_location_field = SCHEMA.get_field("fileLocation").unwrap();
            let url_path_field = SCHEMA.get_field("urlPath").unwrap();
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
            let file_location_value = retrieved_doc.get_first(file_location_field).unwrap().as_str().unwrap();
            let url_path_value = retrieved_doc.get_first(url_path_field).unwrap().as_str().unwrap();
            let content_value = retrieved_doc.get_first(content_field).unwrap().as_str().unwrap();
            results.push(Record {
                content: content_value.to_owned(),
                file_location: file_location_value.to_owned(),
                url_path: url_path_value.to_owned(),
            });
        }
        results
    }
}

#[napi]
impl SearchIndex {

    #[napi(constructor)]
    pub fn new(dir: String) -> Self {
        Self { index: Self::_index(Path::new(&dir)) }
    }

    #[napi]
    pub fn clear(&self) {
        self._clear()
    }

    #[napi]
    pub fn insert(&self, records: Vec<Record>) {
        self._insert(records)
    }

    #[napi]
    pub fn search(&self, text: String) -> Vec<Record> {
        self._search(text)
    }
}
