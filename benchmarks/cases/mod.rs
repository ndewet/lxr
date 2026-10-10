mod java_like;
mod json;
mod keywords;
mod lisp_like;
mod punctuation;
mod rust_like;
mod small;
mod xml;

use crate::support::{generation, scanner};
use criterion::Criterion;

pub fn benchmarks(criterion: &mut Criterion) {
    generation::benchmarks(
        criterion,
        &[
            generation::Definition::new("small", small::definition),
            generation::Definition::new("rust-like", rust_like::definition),
            generation::Definition::new("java-like", java_like::definition),
            generation::Definition::new("lisp-like", lisp_like::definition),
            generation::Definition::new("json", json::definition),
            generation::Definition::new("xml", xml::definition),
            generation::Definition::new("punctuation", punctuation::definition),
            generation::Definition::new("keywords", keywords::definition),
        ],
    );
    scanner::benchmarks::<rust_like::Token>(criterion, "rust-like", rust_like::SOURCE);
    scanner::benchmarks::<java_like::Token>(criterion, "java-like", java_like::SOURCE);
    scanner::benchmarks::<lisp_like::Token>(criterion, "lisp-like", lisp_like::SOURCE);
    scanner::benchmarks::<json::Token>(criterion, "json", json::SOURCE);
    scanner::benchmarks::<xml::Token>(criterion, "xml", xml::SOURCE);
    scanner::benchmarks::<punctuation::Token>(criterion, "punctuation", punctuation::SOURCE);
    scanner::benchmarks::<keywords::Token>(criterion, "keywords", keywords::SOURCE);
}
